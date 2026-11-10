#!/system/bin/sh
# Starts yserver headless with the frame recorder, runs xprobe against it, stops it.
# Usage (from the unpacked folder, as the Termux user): sh run-test.sh
set -u
D=$(cd "$(dirname "$0")" && pwd)
OUT="$D/out-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUT/rec" "$D/tmp"
export YSERVER_TMPDIR="$D/tmp"
export XKB_CONFIG_ROOT="$D/xkb"
export RUST_LOG="${RUST_LOG:-info}"
SOCK="$D/tmp/.X11-unix/X7"
rm -f "$SOCK"

START=$(date +%s%N)
"$D/yserver" :7 -nolisten tcp --headless --rootless --rootless-record "$OUT/rec" >"$OUT/yserver.log" 2>&1 &
YS=$!
i=0
while [ ! -S "$SOCK" ] && [ $i -lt 150 ] && kill -0 $YS 2>/dev/null; do sleep 0.1; i=$((i+1)); done
READY=$(date +%s%N)
S="$OUT/summary.txt"
echo "yserver pid $YS; socket after $(( (READY - START) / 1000000 )) ms" | tee "$S"
if [ ! -S "$SOCK" ]; then
    echo "FAIL: no socket at $SOCK" | tee -a "$S"
    kill $YS 2>/dev/null
    tail -40 "$OUT/yserver.log"
    exit 1
fi
grep -E "VmRSS|VmHWM" /proc/$YS/status | sed 's/^/idle   /' | tee -a "$S"

"$D/xprobe" --socket "$SOCK" --record-dir "$OUT/rec" >"$OUT/xprobe.txt" 2>&1
RC=$?
cat "$OUT/xprobe.txt"
grep -E "VmRSS|VmHWM" /proc/$YS/status | sed 's/^/after  /' | tee -a "$S"
if kill -0 $YS 2>/dev/null; then echo "yserver still running after the probe" | tee -a "$S"; else echo "yserver EXITED during the probe" | tee -a "$S"; fi
kill $YS 2>/dev/null
wait $YS 2>/dev/null
echo "xprobe exit $RC; recorded files: $(ls "$OUT/rec" | wc -l); log: $OUT" | tee -a "$S"
grep -c -i -E "panic|error" "$OUT/yserver.log" | sed 's/^/yserver.log panic|error lines: /' | tee -a "$S"
exit $RC
