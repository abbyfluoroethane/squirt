//! One track's decoder: symphonia packets in, interleaved f32 at the output's
//! sample rate and channel count out.

use rubato::{FftFixedIn, Resampler};
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

const RESAMPLE_CHUNK: usize = 1024;

pub struct Source {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    out_channels: usize,
    duration_ms: u64,
    resampler: Option<FftFixedIn<f32>>,
    /// Planar input waiting for a full resampler chunk.
    planar: Vec<Vec<f32>>,
    eof: bool,
}

impl Source {
    pub fn open(path: &Path, out_rate: u32, out_channels: usize, start_ms: u64) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions { enable_gapless: true, ..Default::default() }, &MetadataOptions::default())
            .map_err(|e| format!("unsupported format: {e}"))?;
        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("no audio track")?;
        let track_id = track.id;
        let params = track.codec_params.clone();
        let src_rate = params.sample_rate.ok_or("unknown sample rate")?;
        let duration_ms = match (params.n_frames, params.time_base) {
            (Some(n), Some(tb)) => {
                let t = tb.calc_time(n);
                t.seconds * 1000 + (t.frac * 1000.0) as u64
            }
            (Some(n), None) => n * 1000 / src_rate as u64,
            _ => 0,
        };
        let decoder = symphonia::default::get_codecs()
            .make(&params, &DecoderOptions::default())
            .map_err(|e| format!("unsupported codec: {e}"))?;

        if start_ms > 0 {
            format
                .seek(
                    SeekMode::Accurate,
                    SeekTo::Time { time: Time::new(start_ms / 1000, (start_ms % 1000) as f64 / 1000.0), track_id: Some(track_id) },
                )
                .map_err(|e| format!("seek failed: {e}"))?;
        }

        let resampler = if src_rate != out_rate {
            Some(
                FftFixedIn::<f32>::new(src_rate as usize, out_rate as usize, RESAMPLE_CHUNK, 2, out_channels)
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        Ok(Self {
            format,
            decoder,
            track_id,
            out_channels,
            duration_ms,
            resampler,
            planar: vec![Vec::new(); out_channels],
            eof: false,
        })
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    /// Decodes one packet's worth of audio, mapped to the output channels
    /// (still at the source rate). `None` at the end of the track.
    fn decode_packet(&mut self) -> Result<Option<Vec<f32>>, String> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                Err(SymError::ResetRequired) => return Ok(None),
                Err(e) => return Err(e.to_string()),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let audio = match self.decoder.decode(&packet) {
                Ok(a) => a,
                // A corrupt packet: skip it rather than stop the track.
                Err(SymError::DecodeError(_)) => continue,
                Err(e) => return Err(e.to_string()),
            };
            let spec = *audio.spec();
            let src_ch = spec.channels.count();
            let mut buf = SampleBuffer::<f32>::new(audio.capacity() as u64, spec);
            buf.copy_interleaved_ref(audio);
            let samples = buf.samples();
            if src_ch == self.out_channels {
                return Ok(Some(samples.to_vec()));
            }
            let frames = samples.len() / src_ch;
            let mut out = Vec::with_capacity(frames * self.out_channels);
            for f in samples.chunks_exact(src_ch) {
                for c in 0..self.out_channels {
                    out.push(match src_ch {
                        1 => f[0],
                        _ if c < src_ch => f[c],
                        _ => 0.0,
                    });
                }
            }
            return Ok(Some(out));
        }
    }

    /// The next chunk of interleaved output samples, or `None` when done.
    pub fn next_chunk(&mut self) -> Result<Option<Vec<f32>>, String> {
        if self.resampler.is_none() {
            return self.decode_packet();
        }
        let ch = self.out_channels;
        loop {
            let rs = self.resampler.as_mut().unwrap();
            let need = rs.input_frames_next();
            if self.planar[0].len() >= need {
                let input: Vec<&[f32]> = self.planar.iter().map(|c| &c[..need]).collect();
                let out = rs.process(&input, None).map_err(|e| e.to_string())?;
                for c in self.planar.iter_mut() {
                    c.drain(..need);
                }
                return Ok(Some(interleave(&out, ch)));
            }
            if self.eof {
                if self.planar[0].is_empty() {
                    return Ok(None);
                }
                let input: Vec<&[f32]> = self.planar.iter().map(|c| c.as_slice()).collect();
                let out = rs.process_partial(Some(&input), None).map_err(|e| e.to_string())?;
                for c in self.planar.iter_mut() {
                    c.clear();
                }
                return Ok(Some(interleave(&out, ch)));
            }
            match self.decode_packet()? {
                Some(samples) => {
                    for f in samples.chunks_exact(ch) {
                        for (c, s) in f.iter().enumerate() {
                            self.planar[c].push(*s);
                        }
                    }
                }
                None => self.eof = true,
            }
        }
    }
}

fn interleave(planar: &[Vec<f32>], channels: usize) -> Vec<f32> {
    let frames = planar.first().map_or(0, |c| c.len());
    let mut out = Vec::with_capacity(frames * channels);
    for i in 0..frames {
        for c in planar.iter().take(channels) {
            out.push(c[i]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a 16-bit PCM WAV of a 440 Hz tone.
    fn write_wav(path: &Path, rate: u32, channels: u16, secs: f32) {
        let frames = (rate as f32 * secs) as u32;
        let data_len = frames * channels as u32 * 2;
        let mut b = Vec::new();
        b.extend(b"RIFF");
        b.extend((36 + data_len).to_le_bytes());
        b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(channels.to_le_bytes());
        b.extend(rate.to_le_bytes());
        b.extend((rate * channels as u32 * 2).to_le_bytes());
        b.extend((channels * 2).to_le_bytes());
        b.extend(16u16.to_le_bytes());
        b.extend(b"data");
        b.extend(data_len.to_le_bytes());
        for i in 0..frames {
            let v = ((i as f32 / rate as f32 * 440.0 * std::f32::consts::TAU).sin() * 8000.0) as i16;
            for _ in 0..channels {
                b.extend(v.to_le_bytes());
            }
        }
        std::fs::write(path, b).unwrap();
    }

    fn total_frames(src: &mut Source, channels: usize) -> usize {
        let mut n = 0;
        while let Some(chunk) = src.next_chunk().unwrap() {
            assert_eq!(chunk.len() % channels, 0);
            n += chunk.len() / channels;
        }
        n
    }

    #[test]
    fn same_rate_mono_to_stereo() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t.wav");
        write_wav(&p, 48000, 1, 1.0);
        let mut s = Source::open(&p, 48000, 2, 0).unwrap();
        assert_eq!(s.duration_ms(), 1000);
        assert_eq!(total_frames(&mut s, 2), 48000);
    }

    #[test]
    fn resamples_44k_to_48k() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t.wav");
        write_wav(&p, 44100, 2, 2.0);
        let mut s = Source::open(&p, 48000, 2, 0).unwrap();
        let n = total_frames(&mut s, 2) as i64;
        // FFT resampling adds a little delay padding; allow a few chunks.
        assert!((n - 96000).abs() < 4096, "got {n} frames");
    }

    #[test]
    fn seek_skips_ahead() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t.wav");
        write_wav(&p, 48000, 2, 2.0);
        let mut s = Source::open(&p, 48000, 2, 1500).unwrap();
        let n = total_frames(&mut s, 2) as i64;
        assert!((n - 24000).abs() < 2048, "got {n} frames");
    }

    #[test]
    fn missing_file_is_an_error() {
        assert!(Source::open(Path::new("/nonexistent.mp3"), 48000, 2, 0).is_err());
    }
}

#[cfg(test)]
mod real_file {
    use super::*;
    /// SQUIRT_SEEK_FILE=/path/to/song cargo test -p squirt-player real_seek -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_seek() {
        let p = std::env::var("SQUIRT_SEEK_FILE").unwrap();
        let t = std::time::Instant::now();
        let mut s = Source::open(Path::new(&p), 48000, 2, 60_000).unwrap();
        let first = s.next_chunk().unwrap().map(|c| c.len());
        println!("opened at 60s in {:?}, duration {} ms, first chunk {:?}", t.elapsed(), s.duration_ms(), first);
    }
}
