#!/usr/bin/env bash
# record.sh <module> <name> — drive the app with acts/<name>.sh on a virtual display and capture lossless footage.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
: "${APP:?set APP to the soryntel-desktop binary}"
OUT=${OUT:-$HERE/out}; DISP=${DISP:-:77}; H=${H:-872}
mod=$1; name=$2
mkdir -p "$OUT"
export DISPLAY=$DISP LOG=$OUT/$name.mouse
: > "$LOG"
pgrep -f "Xvfb $DISP" >/dev/null || { Xvfb "$DISP" -screen 0 1600x1000x24 >/dev/null 2>&1 & sleep 1; }
env -u WAYLAND_DISPLAY LIBGL_ALWAYS_SOFTWARE=1 "$APP" --module "$mod" > "$OUT/$name.log" 2>&1 &
pid=$!; sleep 4
win=$(xdotool search --pid "$pid" | tail -1)
xdotool windowmove "$win" 0 0 windowsize "$win" 1440 900; xdotool windowfocus "$win"; sleep 2
source "$HERE/lib.sh"
mv 1430 $((H - 10))
# -copyts keeps wall-clock PTS so the cursor log aligns frame-exactly.
ffmpeg -y -loglevel error -f x11grab -copyts -draw_mouse 0 -framerate 30 -video_size 1440x$H -i "$DISP.0+0,0" \
  -c:v libx264 -preset ultrafast -qp 0 "$OUT/$name.nut" &
fp=$!; sleep 1.2
source "$HERE/acts/$name.sh"
w 0.8
kill -INT $fp; wait $fp 2>/dev/null || true
kill $pid; wait $pid 2>/dev/null || true
echo "recorded $name"
