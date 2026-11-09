#!/usr/bin/env bash
# Builds the minimal, LGPL-only ffmpeg and ffprobe that ship with Squirt.
#
#   scripts/build-ffmpeg.sh [target-triple]
#
# Output: app/src-tauri/binaries/{ffmpeg,ffprobe}-<triple>[.exe] (the names
# Tauri expects for sidecars), plus FFMPEG-LICENSE.txt and ffmpeg-build.txt
# recording the exact version and configure line.
#
# Licensing: configured without --enable-gpl / --enable-nonfree / version3,
# so the result is LGPL-2.1-or-later. That rules out x264, so H.264 is
# encoded with the OS's own encoder (VideoToolbox on macOS, Media Foundation
# on Windows); everywhere, ffmpeg's built-in MPEG-4 Part 2 encoder (which the
# Zune HD also plays) is the fallback. --disable-autodetect keeps anything not
# listed here from being linked in.
set -euo pipefail

VERSION=7.1.2
SHA256=089bc60fb59d6aecc5d994ff530fd0dcb3ee39aa55867849a2bbc4e555f9c304
URL="https://ffmpeg.org/releases/ffmpeg-${VERSION}.tar.xz"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/app/src-tauri/binaries"
TRIPLE="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
WORK="${FFMPEG_WORK:-$ROOT/target/ffmpeg-build/$TRIPLE}"
JOBS="${JOBS:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)}"

mkdir -p "$WORK" "$OUT"
cd "$WORK"
if [ ! -f "ffmpeg-$VERSION.tar.xz" ]; then
	curl -fsSL -o "ffmpeg-$VERSION.tar.xz" "$URL"
fi
if command -v shasum >/dev/null; then got=$(shasum -a 256 "ffmpeg-$VERSION.tar.xz" | cut -d' ' -f1); else got=$(sha256sum "ffmpeg-$VERSION.tar.xz" | cut -d' ' -f1); fi
if [ "$SHA256" != "PENDING" ] && [ "$got" != "$SHA256" ]; then
	echo "checksum mismatch for ffmpeg-$VERSION.tar.xz: $got" >&2
	exit 1
fi
rm -rf "ffmpeg-$VERSION" && tar xf "ffmpeg-$VERSION.tar.xz"
cd "ffmpeg-$VERSION"

COMMON=(
	--disable-everything --disable-autodetect
	--disable-doc --disable-debug --disable-network --disable-ffplay
	--enable-static --disable-shared
	--enable-ffmpeg --enable-ffprobe
	--enable-avcodec --enable-avformat --enable-avfilter --enable-swscale --enable-swresample
	--enable-protocol=file,pipe
	# Reading: common video containers, plus audio-only files for probing.
	--enable-demuxer=mov,matroska,avi,asf,mpegts,mpegps,m4v,h264,hevc,image2,mp3,aac,wav,flac,ogg
	--enable-decoder=h264,hevc,vp8,vp9,mpeg4,mpeg2video,mpeg1video,msmpeg4v1,msmpeg4v2,msmpeg4v3,wmv1,wmv2,wmv3,vc1,mjpeg,png
	--enable-decoder=aac,mp3,mp3float,ac3,eac3,opus,vorbis,flac,alac,wmav1,wmav2,pcm_s16le,pcm_s24le,pcm_f32le
	--enable-parser=h264,hevc,aac,mpeg4video,mpegvideo,vp8,vp9,mpegaudio,ac3,vc1,mjpeg,png,flac,opus,vorbis
	--enable-bsf=h264_mp4toannexb,hevc_mp4toannexb,aac_adtstoasc,extract_extradata
	# Writing: MP4 for the Zune, JPEG/PNG for thumbnails.
	--enable-encoder=mpeg4,aac,mjpeg,png
	--enable-muxer=mp4,mov,ipod,image2,adts
	--enable-filter=scale,format,fps,pad,crop,setsar,setdar,transpose,hflip,vflip,null,anull,aresample,aformat,select,thumbnail,trim,atrim
)

EXT=""
case "$TRIPLE" in
	*apple-darwin)
		arch=${TRIPLE%%-*}; [ "$arch" = aarch64 ] && arch=arm64
		PLATFORM=(--enable-videotoolbox --enable-audiotoolbox --enable-encoder=h264_videotoolbox,hevc_videotoolbox
			--arch="$arch" --cc="clang -arch $arch" --extra-cflags="-mmacosx-version-min=11.0" --extra-ldflags="-mmacosx-version-min=11.0")
		;;
	*windows*)
		EXT=".exe"
		PLATFORM=(--enable-mediafoundation --enable-encoder=h264_mf,hevc_mf --target-os=mingw32 --extra-ldflags=-static)
		;;
	*linux*)
		PLATFORM=(--extra-ldflags=-static)
		;;
	*) echo "unsupported target $TRIPLE" >&2; exit 1 ;;
esac

./configure "${COMMON[@]}" "${PLATFORM[@]}"
make -j"$JOBS"

# Refuse to ship a build that isn't LGPL.
if ! ./ffmpeg$EXT -hide_banner -L 2>&1 | grep -q "GNU Lesser General Public"; then
	echo "refusing to ship: this ffmpeg build is not LGPL" >&2
	exit 1
fi

strip ffmpeg$EXT ffprobe$EXT 2>/dev/null || true
cp ffmpeg$EXT "$OUT/ffmpeg-$TRIPLE$EXT"
cp ffprobe$EXT "$OUT/ffprobe-$TRIPLE$EXT"
cp COPYING.LGPLv2.1 "$OUT/FFMPEG-LICENSE.txt"
{
	echo "FFmpeg $VERSION, LGPL-2.1-or-later. Source: $URL"
	echo "Built for $TRIPLE with:"
	echo "./configure ${COMMON[*]} ${PLATFORM[*]}"
} > "$OUT/ffmpeg-build.txt"
ls -la "$OUT"
