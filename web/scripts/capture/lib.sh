# Input helpers. SLOW scales every wait so the software-rendered app can
# draw each step; post.sh speeds the footage back up by the same factor.
SLOW=${SLOW:-3}
LOG=${LOG:-/dev/null}
now(){ date +%s.%N; }
w(){ sleep "$(awk -v a="$1" -v s="$SLOW" 'BEGIN{print a*s}')"; }
mv(){ xdotool mousemove "$1" "$2"; echo "$(now) $1 $2" >> "$LOG"; }
down(){ xdotool mousedown 1; echo "$(now) D" >> "$LOG"; }
up(){ xdotool mouseup 1; echo "$(now) U" >> "$LOG"; }
click(){ down; w 0.06; up; }
key(){ xdotool key "$@"; }
typ(){ xdotool type --delay "$((70 * SLOW))" "$1"; }
# glide x1 y1 x2 y2 [steps]: eased pointer move, ~60 Hz in final footage
glide(){
  local n=$(( ${5:-40} * 13 / 10 ))
  awk -v a="$1" -v b="$2" -v c="$3" -v d="$4" -v n="$n" 'BEGIN{for(i=0;i<=n;i++){p=i/n;t=(p<.5)?4*p*p*p:1-(-2*p+2)^3/2;printf "%d %d\n",a+(c-a)*t,b+(d-b)*t}}' |
  while read -r x y; do mv "$x" "$y"; w 0.016; done
}
