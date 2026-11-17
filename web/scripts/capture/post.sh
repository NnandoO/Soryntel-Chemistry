#!/usr/bin/env bash
# post.sh <name> [width] — speed footage back up, composite the cursor, encode MP4 + poster.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=${OUT:-$HERE/out}; DEST=${DEST:-$HERE/../../public/media}; SLOW=${SLOW:-3}
name=$1; W=${2:-1440}
src=$OUT/$name.nut
t0=$(ffprobe -v error -show_entries packet=pts_time -of csv=p=0 "$src" | sort -n | awk 'NR==1')
# Cursor commands in output time: (wallclock - first frame) / SLOW.
awk -v t0="$t0" -v s="$SLOW" '
  NF==3 { t=($1-t0)/s; if (t<0) t=0; printf "%.4f overlay@c x %d, overlay@c y %d;\n", t, $2-10, $3-6; lx=$2; ly=$3 }
  $2=="D" { t=($1-t0)/s; if (t<0) t=0; printf "%.4f overlay@r x %d, overlay@r y %d, overlay@r enable 1;\n%.4f overlay@r enable 0;\n", t, lx-36, ly-36, t+0.28 }
' "$OUT/$name.mouse" > "$OUT/$name.cmd"
init=$(awk 'NF==3{printf "0.0 overlay@c x %d, overlay@c y %d;\n",$2-10,$3-6; exit}' "$OUT/$name.mouse")
sed -i "1i $init" "$OUT/$name.cmd"
mkdir -p "$DEST"
ffmpeg -y -loglevel error -i "$src" -loop 1 -i "$HERE/cursor.png" -loop 1 -i "$HERE/ring.png" -filter_complex \
 "[0:v]setpts=(PTS-STARTPTS)/$SLOW,fps=60,sendcmd=f=$OUT/$name.cmd[v];[2:v]setpts=N/60/TB[r];[1:v]setpts=N/60/TB[c];[v][r]overlay@r=x=-200:y=-200:enable=0:shortest=1[v2];[v2][c]overlay@c=x=-100:y=-100:shortest=1,scale=$W:-2:flags=lanczos,format=yuv420p[o]" \
 -map "[o]" -r 60 -c:v libx264 -preset slow -crf 23 -profile:v high -movflags +faststart -an "$DEST/$name.mp4"
# No WebM: on flat UI footage VP9 came out 40–70% larger than x264 at equal quality.
ffmpeg -y -loglevel error -sseof -0.2 -i "$DEST/$name.mp4" -frames:v 1 -q:v 3 "$DEST/$name.jpg"
ls -la "$DEST/$name".*
