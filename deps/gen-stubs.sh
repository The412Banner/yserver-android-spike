#!/bin/sh
# Stub libgbm.a / libinput.a / libudev.a for the zero-card Android build: the
# KMS platform's DRM/libinput/udev code is compiled in, but with no card and the
# Wayland host's seat it is never called. Every stub aborts with its name.
# usage: gen-stubs.sh <symbols dir> <out lib dir> <cc> <ar>
set -eu
SYMS=$1 OUT=$2 CC=$3 AR=$4
mkdir -p "$OUT"
for n in gbm input udev; do
  c="$OUT/stub_$n.c"
  {
    echo '#include <stdio.h>'
    echo '#include <stdlib.h>'
    printf '%s\n' 'static void die(const char *s) { fprintf(stderr, "yserver: stub %s called (no DRM card / libinput on Android)\n", s); abort(); }'
    while read -r s; do echo "void $s(void) { die(\"$s\"); }"; done < "$SYMS/$n.syms"
  } > "$c"
  "$CC" -c -O1 -fPIC -o "$OUT/stub_$n.o" "$c"
  rm -f "$OUT/lib$n.a"
  "$AR" rcs "$OUT/lib$n.a" "$OUT/stub_$n.o"
done
mkdir -p "$OUT/pkgconfig"
for n in gbm:gbm input:libinput udev:libudev; do
  lib=${n%%:*} pc=${n#*:}
  printf 'libdir=%s\nName: %s\nDescription: stub\nVersion: 99\nLibs: -L${libdir} -l%s\nCflags:\n' "$OUT" "$pc" "$lib" > "$OUT/pkgconfig/$pc.pc"
done
ls -la "$OUT"
