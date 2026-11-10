#!/usr/bin/env bash
# Builds libxkbcommon as a static library for aarch64 Android with the NDK.
# Usage: deps/build-xkbcommon.sh <out-dir>   (needs NDK_BIN, meson, ninja, bison)
set -euo pipefail

VERSION=1.13.2
COMMIT=d1442aaa6b635551182e83c3b55037f4c118c962
API=26
OUT=$(realpath -m "$1")
WORK=$(mktemp -d)

git -C "$WORK" init -q src
git -C "$WORK/src" fetch -q --depth 1 https://github.com/xkbcommon/libxkbcommon.git "refs/tags/xkbcommon-$VERSION"
git -C "$WORK/src" checkout -q FETCH_HEAD
test "$(git -C "$WORK/src" rev-parse HEAD)" = "$COMMIT" || { echo "libxkbcommon tag moved" >&2; exit 1; }

cat > "$WORK/android.cross" <<EOF
[binaries]
c = '$NDK_BIN/aarch64-linux-android$API-clang'
ar = '$NDK_BIN/llvm-ar'
strip = '$NDK_BIN/llvm-strip'

[host_machine]
system = 'android'
cpu_family = 'aarch64'
cpu = 'aarch64'
endian = 'little'
EOF

meson setup "$WORK/build" "$WORK/src" \
  --cross-file "$WORK/android.cross" \
  --prefix "$OUT" --libdir lib \
  --buildtype release \
  -Ddefault_library=static \
  -Denable-tools=false -Denable-x11=false -Denable-wayland=false \
  -Denable-docs=false -Denable-xkbregistry=false -Denable-bash-completion=false \
  -Dxkb-config-root=/usr/share/X11/xkb
ninja -C "$WORK/build"
ninja -C "$WORK/build" install
test -s "$OUT/lib/libxkbcommon.a"
ls -la "$OUT/lib"
