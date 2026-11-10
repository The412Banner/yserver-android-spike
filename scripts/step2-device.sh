#!/system/bin/sh
# yserver Android test, step 2: unpack the staged package into Termux's home and run it.
# Run as uid 10183 in Termux's app context.
export PATH=/system/bin
set -u
echo "context: $(cat /proc/self/attr/current)  uid: $(id -u)"
H=/data/data/com.termux/files/home
S=${1:?usage: step2-device.sh <sha8>}
D=$H/yserver-test/$S
PKG=/sdcard/Download/yserver-android-spike-$S.tar.gz
if [ ! -x "$D/yserver" ]; then
    mkdir -p "$D" || exit 1
    tar -xzf "$PKG" -C "$D" || exit 1
fi
cd "$D" || exit 1
sha256sum -c SHA256SUMS >/dev/null && echo "SHA256SUMS ok" || { echo "SHA256SUMS FAILED"; exit 1; }
cat VERSION
sh "$D/run-test.sh"
echo "run-test exit $?"
