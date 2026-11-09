//! Squirt's audio player: symphonia decodes, cpal plays.
//!
//! Three parts:
//! - A **decoder thread** owns the play queue. It decodes the current track,
//!   converts it to the output's sample rate and channel count, and pushes
//!   samples into a ring buffer. At the end of a track it opens the next one
//!   and keeps pushing into the same buffer, so albums play gapless.
//! - The **audio callback** (cpal) pops samples from the ring buffer and
//!   counts how many frames it has played.
//! - **Segments** record where in that frame count each track starts. The
//!   reported track and position follow what has actually been played, not
//!   what has been decoded ahead.

mod decode;

use decode::Source;
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("no audio output device")]
    NoOutput,
    #[error("audio output: {0}")]
    Output(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub status: Status,
    /// Index into the queue of the track being heard.
    pub index: Option<usize>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub queue_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A track started being heard.
    TrackStarted(usize),
    /// A track played through to its end (count it as a play).
    TrackCompleted(usize),
    /// A track could not be opened or decoded and was skipped.
    TrackFailed(usize, String),
    /// The last track finished.
    QueueEnded,
}

enum Cmd {
    Queue(Vec<PathBuf>, usize),
    Play,
    Pause,
    Next,
    Prev,
    Jump(usize),
    Seek(u64),
    Stop,
}

#[derive(Debug, Clone, Copy)]
struct Segment {
    /// Frame count (in played-frames terms) where this track starts.
    start: u64,
    index: usize,
    /// Track position at `start`, in ms (non-zero after a seek).
    base_ms: u64,
    duration_ms: u64,
    /// True if this track followed the previous one without a skip.
    gapless: bool,
}

struct Shared {
    frames_played: AtomicU64,
    paused: AtomicBool,
    volume: AtomicU32,
    /// Flush handshake: the decoder bumps `flush_req`; the callback empties the
    /// buffer and copies it to `flush_ack`.
    flush_req: AtomicU64,
    flush_ack: AtomicU64,
    segments: Mutex<VecDeque<Segment>>,
    status: Mutex<Status>,
    queue_len: AtomicU64,
    events: Mutex<Vec<Event>>,
}

pub struct Player {
    tx: Sender<Cmd>,
    shared: Arc<Shared>,
    rate: u32,
}

impl Player {
    /// Opens the default audio output and starts the player threads.
    pub fn new() -> Result<Self, PlayerError> {
        let shared = Arc::new(Shared {
            frames_played: AtomicU64::new(0),
            paused: AtomicBool::new(true),
            volume: AtomicU32::new(1.0f32.to_bits()),
            flush_req: AtomicU64::new(0),
            flush_ack: AtomicU64::new(0),
            segments: Mutex::new(VecDeque::new()),
            status: Mutex::new(Status::Stopped),
            queue_len: AtomicU64::new(0),
            events: Mutex::new(Vec::new()),
        });

        // The cpal stream must stay on the thread that created it, so a
        // dedicated thread owns it and reports back the output format.
        let (fmt_tx, fmt_rx) = mpsc::channel();
        let cb_shared = shared.clone();
        std::thread::Builder::new()
            .name("squirt-audio".into())
            .spawn(move || output::run(cb_shared, fmt_tx))
            .map_err(|e| PlayerError::Output(e.to_string()))?;
        let (rate, channels, prod) = fmt_rx
            .recv()
            .map_err(|_| PlayerError::Output("audio thread exited".into()))??;

        let (tx, rx) = mpsc::channel();
        let dec_shared = shared.clone();
        std::thread::Builder::new()
            .name("squirt-decoder".into())
            .spawn(move || Decoder::new(dec_shared, rx, prod, rate, channels).run())
            .map_err(|e| PlayerError::Output(e.to_string()))?;
        Ok(Self { tx, shared, rate })
    }

    /// Replaces the queue and starts playing at `start`.
    pub fn play_queue(&self, paths: Vec<PathBuf>, start: usize) {
        let _ = self.tx.send(Cmd::Queue(paths, start));
    }
    pub fn play(&self) {
        let _ = self.tx.send(Cmd::Play);
    }
    pub fn pause(&self) {
        let _ = self.tx.send(Cmd::Pause);
    }
    pub fn toggle(&self) {
        if self.state().status == Status::Playing {
            self.pause()
        } else {
            self.play()
        }
    }
    pub fn next(&self) {
        let _ = self.tx.send(Cmd::Next);
    }
    /// Restarts the track, or goes to the previous one in the first 3 seconds.
    pub fn prev(&self) {
        let _ = self.tx.send(Cmd::Prev);
    }
    pub fn jump(&self, index: usize) {
        let _ = self.tx.send(Cmd::Jump(index));
    }
    pub fn seek(&self, position_ms: u64) {
        let _ = self.tx.send(Cmd::Seek(position_ms));
    }
    pub fn stop(&self) {
        let _ = self.tx.send(Cmd::Stop);
    }
    /// Volume from 0.0 to 1.0. Applied with a squared curve so the slider feels even.
    pub fn set_volume(&self, v: f32) {
        self.shared.volume.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn state(&self) -> PlayerState {
        let played = self.shared.frames_played.load(Ordering::Relaxed);
        let seg = current_segment(&self.shared.segments.lock().unwrap(), played);
        let (index, position_ms, duration_ms) = match seg {
            Some(s) => {
                let pos = s.base_ms + (played.saturating_sub(s.start)) * 1000 / self.rate as u64;
                (Some(s.index), pos.min(s.duration_ms.max(pos)), s.duration_ms)
            }
            None => (None, 0, 0),
        };
        PlayerState {
            status: *self.shared.status.lock().unwrap(),
            index,
            position_ms,
            duration_ms,
            volume: f32::from_bits(self.shared.volume.load(Ordering::Relaxed)),
            queue_len: self.shared.queue_len.load(Ordering::Relaxed) as usize,
        }
    }

    /// Events since the last call.
    pub fn take_events(&self) -> Vec<Event> {
        std::mem::take(&mut *self.shared.events.lock().unwrap())
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Stop);
    }
}

fn current_segment(segs: &VecDeque<Segment>, played: u64) -> Option<Segment> {
    segs.iter().rev().find(|s| s.start <= played).copied()
}

// ------------------------------------------------------------------ output

mod output {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, SizedSample};

    type Format = Result<(u32, usize, HeapProd<f32>), PlayerError>;

    pub(super) fn run(shared: Arc<Shared>, fmt_tx: Sender<Format>) {
        let stream = match open(shared) {
            Ok((stream, rate, channels, prod)) => {
                let _ = fmt_tx.send(Ok((rate, channels, prod)));
                stream
            }
            Err(e) => {
                let _ = fmt_tx.send(Err(e));
                return;
            }
        };
        // Keep the stream alive for the life of the process.
        let _stream = stream;
        loop {
            std::thread::park();
        }
    }

    fn open(shared: Arc<Shared>) -> Result<(cpal::Stream, u32, usize, HeapProd<f32>), PlayerError> {
        let device = cpal::default_host().default_output_device().ok_or(PlayerError::NoOutput)?;
        let supported = device.default_output_config().map_err(|e| PlayerError::Output(e.to_string()))?;
        let rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;
        // About 2 s of audio buffered ahead.
        let (prod, cons) = HeapRb::<f32>::new(rate as usize * channels * 2).split();
        let config: cpal::StreamConfig = supported.config();
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&device, &config, shared, cons, channels),
            cpal::SampleFormat::I16 => build::<i16>(&device, &config, shared, cons, channels),
            cpal::SampleFormat::U16 => build::<u16>(&device, &config, shared, cons, channels),
            cpal::SampleFormat::I32 => build::<i32>(&device, &config, shared, cons, channels),
            other => return Err(PlayerError::Output(format!("unsupported sample format {other}"))),
        }?;
        stream.play().map_err(|e| PlayerError::Output(e.to_string()))?;
        Ok((stream, rate, channels, prod))
    }

    fn build<T: SizedSample + FromSample<f32>>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        shared: Arc<Shared>,
        mut cons: HeapCons<f32>,
        channels: usize,
    ) -> Result<cpal::Stream, PlayerError> {
        let mut scratch: Vec<f32> = Vec::new();
        let mut seen_flush = 0u64;
        device
            .build_output_stream(
                config,
                move |out: &mut [T], _| {
                    let req = shared.flush_req.load(Ordering::Acquire);
                    if req != seen_flush {
                        cons.clear();
                        seen_flush = req;
                        shared.flush_ack.store(req, Ordering::Release);
                    }
                    if shared.paused.load(Ordering::Relaxed) {
                        out.fill(T::EQUILIBRIUM);
                        return;
                    }
                    scratch.resize(out.len(), 0.0);
                    let got = cons.pop_slice(&mut scratch);
                    let v = f32::from_bits(shared.volume.load(Ordering::Relaxed));
                    let gain = v * v;
                    for (o, s) in out.iter_mut().zip(&scratch[..got]) {
                        *o = T::from_sample(s * gain);
                    }
                    out[got..].fill(T::EQUILIBRIUM);
                    shared.frames_played.fetch_add((got / channels) as u64, Ordering::Relaxed);
                },
                |e| eprintln!("audio output error: {e}"),
                None,
            )
            .map_err(|e| PlayerError::Output(e.to_string()))
    }
}

// ----------------------------------------------------------------- decoder

struct Decoder {
    shared: Arc<Shared>,
    rx: Receiver<Cmd>,
    prod: HeapProd<f32>,
    rate: u32,
    channels: usize,
    queue: Vec<PathBuf>,
    /// Track being decoded (may be ahead of the one being heard).
    current: Option<(usize, Source)>,
    frames_written: u64,
    /// Samples decoded but not yet pushed because the buffer was full.
    pending: Vec<f32>,
    pending_pos: usize,
    last_heard: Option<usize>,
    ended: bool,
    flush_gen: u64,
}

impl Decoder {
    fn new(shared: Arc<Shared>, rx: Receiver<Cmd>, prod: HeapProd<f32>, rate: u32, channels: usize) -> Self {
        Self {
            shared,
            rx,
            prod,
            rate,
            channels,
            queue: Vec::new(),
            current: None,
            frames_written: 0,
            pending: Vec::new(),
            pending_pos: 0,
            last_heard: None,
            ended: true,
            flush_gen: 0,
        }
    }

    fn set_status(&self, s: Status) {
        *self.shared.status.lock().unwrap() = s;
        self.shared.paused.store(s != Status::Playing, Ordering::Relaxed);
    }

    fn event(&self, e: Event) {
        self.shared.events.lock().unwrap().push(e);
    }

    /// Empties the ring buffer and waits for the callback to confirm, so no
    /// stale audio is heard after a skip or seek.
    fn flush(&mut self) {
        self.flush_gen += 1;
        self.shared.flush_req.store(self.flush_gen, Ordering::Release);
        // The callback runs every few ms; if output is stalled, give up after 200 ms.
        for _ in 0..200 {
            if self.shared.flush_ack.load(Ordering::Acquire) == self.flush_gen {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        self.pending.clear();
        self.pending_pos = 0;
        self.frames_written = self.shared.frames_played.load(Ordering::Relaxed);
        self.shared.segments.lock().unwrap().clear();
    }

    /// Opens queue entry `index` (skipping unreadable ones) at `start_ms`.
    fn open(&mut self, mut index: usize, start_ms: u64, gapless: bool) -> bool {
        while index < self.queue.len() {
            match Source::open(&self.queue[index], self.rate, self.channels, start_ms) {
                Ok(src) => {
                    self.shared.segments.lock().unwrap().push_back(Segment {
                        start: self.frames_written,
                        index,
                        base_ms: start_ms,
                        duration_ms: src.duration_ms(),
                        gapless,
                    });
                    self.current = Some((index, src));
                    self.ended = false;
                    return true;
                }
                Err(e) => {
                    self.event(Event::TrackFailed(index, e));
                    index += 1;
                }
            }
        }
        self.current = None;
        self.ended = true;
        false
    }

    fn start_at(&mut self, index: usize, start_ms: u64) {
        self.flush();
        self.last_heard = None;
        if self.open(index, start_ms, false) {
            self.set_status(Status::Playing);
        } else {
            self.set_status(Status::Stopped);
        }
    }

    fn heard(&self) -> Option<Segment> {
        current_segment(&self.shared.segments.lock().unwrap(), self.shared.frames_played.load(Ordering::Relaxed))
    }

    fn handle(&mut self, cmd: Cmd) -> bool {
        match cmd {
            Cmd::Queue(paths, start) => {
                self.queue = paths;
                self.shared.queue_len.store(self.queue.len() as u64, Ordering::Relaxed);
                self.start_at(start, 0);
            }
            Cmd::Play => {
                if self.current.is_none() && self.heard().is_none() && !self.queue.is_empty() {
                    self.start_at(0, 0);
                } else if self.heard().is_some() {
                    self.set_status(Status::Playing);
                }
            }
            Cmd::Pause => {
                if *self.shared.status.lock().unwrap() == Status::Playing {
                    self.set_status(Status::Paused);
                }
            }
            Cmd::Next => {
                if let Some(s) = self.heard() {
                    if s.index + 1 < self.queue.len() {
                        self.start_at(s.index + 1, 0);
                    }
                }
            }
            Cmd::Prev => {
                if let Some(s) = self.heard() {
                    let played = self.shared.frames_played.load(Ordering::Relaxed);
                    let pos = s.base_ms + played.saturating_sub(s.start) * 1000 / self.rate as u64;
                    let target = if pos < 3000 && s.index > 0 { s.index - 1 } else { s.index };
                    self.start_at(target, 0);
                }
            }
            Cmd::Jump(i) => {
                if i < self.queue.len() {
                    self.start_at(i, 0);
                }
            }
            Cmd::Seek(ms) => {
                if let Some(s) = self.heard() {
                    let was = *self.shared.status.lock().unwrap();
                    self.start_at(s.index, ms.min(s.duration_ms));
                    if was == Status::Paused {
                        self.set_status(Status::Paused);
                    }
                }
            }
            Cmd::Stop => {
                self.flush();
                self.current = None;
                self.ended = true;
                self.set_status(Status::Stopped);
                return false;
            }
        }
        true
    }

    fn run(mut self) {
        loop {
            // Commands: block when idle, poll while decoding.
            let cmd = if self.current.is_none() && self.pending_pos >= self.pending.len() {
                match self.rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(c) => Some(c),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            } else {
                match self.rx.try_recv() {
                    Ok(c) => Some(c),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => return,
                }
            };
            if let Some(c) = cmd {
                let stop = matches!(c, Cmd::Stop);
                self.handle(c);
                if stop {
                    // Stop clears the player; keep running for later commands.
                    continue;
                }
            }

            self.track_heard_changes();

            // Push pending samples first.
            if self.pending_pos < self.pending.len() {
                let n = self.prod.push_slice(&self.pending[self.pending_pos..]);
                self.pending_pos += n;
                self.frames_written += (n / self.channels) as u64;
                if self.pending_pos < self.pending.len() {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
            }

            // Decode more.
            let Some((index, src)) = self.current.as_mut() else { continue };
            let index = *index;
            match src.next_chunk() {
                Ok(Some(samples)) => {
                    self.pending = samples;
                    self.pending_pos = 0;
                }
                Ok(None) => {
                    // End of track: continue straight into the next (gapless).
                    self.current = None;
                    if !self.open(index + 1, 0, true) {
                        self.ended = true;
                    }
                }
                Err(e) => {
                    self.event(Event::TrackFailed(index, e));
                    self.current = None;
                    self.open(index + 1, 0, true);
                }
            }
        }
    }

    /// Emits started/completed events as the heard track changes, and notices
    /// the end of the queue once everything decoded has been played.
    fn track_heard_changes(&mut self) {
        let heard = self.heard();
        let now = heard.map(|s| s.index);
        if now != self.last_heard {
            if let (Some(prev), Some(s)) = (self.last_heard, heard) {
                if s.gapless {
                    self.event(Event::TrackCompleted(prev));
                }
            }
            if let Some(i) = now {
                self.event(Event::TrackStarted(i));
            }
            self.last_heard = now;
        }
        let played = self.shared.frames_played.load(Ordering::Relaxed);
        let status = *self.shared.status.lock().unwrap();
        if self.ended
            && self.current.is_none()
            && self.pending_pos >= self.pending.len()
            && status == Status::Playing
            && played >= self.frames_written
            && self.prod.occupied_len() == 0
        {
            if let Some(i) = self.last_heard {
                self.event(Event::TrackCompleted(i));
            }
            self.event(Event::QueueEnded);
            self.shared.segments.lock().unwrap().clear();
            self.last_heard = None;
            self.set_status(Status::Stopped);
        }
    }
}
