# Progress log

Times are EST (America/New_York).

## Lessons learned (don't repeat these)
- (none yet)

## 2026-11-10 — Steps 0–2 started (feat/android-build, in progress)
- **Why:** user asked to test whether yserver (Rust X server, joske/yserver, MIT) can build and run
  on Android as a possible base for Bannerlator's X11 (and later Wayland) display. User approved a
  private repo and steps 0–2 on 2026-11-10 ~18:1x EST. Not contacting the author yet (user did not
  decide; everything stays private).
- **Plan:** 0 repo + workflow + test program; 1 CI build for aarch64-linux-android; 2 run on the
  Pocket FIT as the Termux user: yserver `--headless --rootless --rootless-record`, then `xprobe`.
- **yserver pin:** `031ec58def8ffd88155cbef0ed01f4c53f655742` (master, 2026-11-10).
- **Patch 0001 (from code reading, before any build):**
  - signals: Android uses the Linux signalfd path (`crates/yserver/src/lib.rs`);
  - anonymous shm: Android uses the memfd syscall; POSIX `shm_open` is not in bionic (`anon_shm.rs`);
  - MIT-SHM SysV `Attach`: refused on Android (no SysV shm in bionic / libc crate) (`server.rs`);
  - socket + lock dir: `YSERVER_TMPDIR` instead of the fixed `/tmp` (`surface/server.rs`);
  - freetype from its bundled source on Android (`crates/yserver/Cargo.toml` + `Cargo.lock`).
- **Other deps:** libxkbcommon 1.13.2 static via meson + NDK (API 26); fontconfig loaded at run time
  (`RUST_FONTCONFIG_DLOPEN=1`), no Android fontconfig shipped yet; xkb data from Ubuntu `xkb-data`.
- **xprobe checks:** connect + setup, extension list (Wine-relevant ones listed), window map/expose,
  fill + PutImage read-back, MIT-SHM AttachFd read-back, XKB UseExtension, keymap, unknown core
  request (must error and keep the stream in step), OpenFont "fixed" (must not kill the server),
  10,000 round trips, PutImage MB/s, recorded frame matches. `cargo check` on the host: clean.
- **CI run 42923030484** (headSha ebe716ef, push): FAILED at Compile — patch applied, libxkbcommon
  1.13.2 static built; `yserver-render` build script needs `glslc` (shader compiler) on PATH.
  Fix: `glslc` added to the runner packages (next run). Sandbox check for step 2: as uid 10183 with
  `runcon u:r:untrusted_app_27:s0:c183,c256,c512,c768` the shell runs in Termux's real app context
  and reads /sdcard/Download.
- **Patch 0002 (fonts, from code reading):** yserver opens its default font "fixed" at start and stops
  if fontconfig can't load (`kms/core.rs:728`). Android has no fontconfig, so Android uses the
  Windows stand-in that scans font folders with FreeType (`kms/font_scan.rs`): folders from
  `YSERVER_FONT_DIRS` or `/system/fonts`, monospace = Droid Sans Mono, Cutive Mono (both in the
  Pocket FIT's /system/fonts). fontconfig crate dropped on Android; Cargo.lock unchanged.
- **CI run 42923295781** (headSha b68398bd): FAILED at Compile — yserver-protocol and yserver-core
  compiled for Android (core: 1 warning); yserver-render: 2 errors, `libc::ioctl` request type is
  `c_int` on Android vs `c_ulong` on glibc (`kms/vk/dri3.rs:588`, `:765`). Patch 0003: Android uses
  the Linux constants that already cast to `libc::Ioctl`. All three patches apply in order to the pin
  (checked in a clean worktree).
- **CI run 42923932269** (headSha c8167005, push): ✅ GREEN — STEP 1 PASSED. Full build from an
  empty cache (329 crates, yserver release in 1m40s; NDK 27.3.13750724, Rust 1.99.1, meson 1.6.1,
  glslc 2023.8, xkb-data 2.41-2ubuntu1.1, libxkbcommon.a 600,242 B). Only warning: unused `Sysv`
  variant (our Android stub). `yserver` 8,580,032 B stripped, `xprobe` 552,928 B; both NEED only
  libc/libm/libdl (+ libz for yserver). Artifact `yserver-android-c8167005.tar.gz` sha256
  `bd4d03014cbb9ef6e5d63a6864e3e12fefbf41bd7686b32eb26cca39ea6fb010`.
- **Device test (step 2, Pocket FIT):** next — stage to /sdcard/Download/yserver-android-spike-c8167005.tar.gz
  (hash host == device), unpack as uid 10183 in Termux's app context into
  ~/yserver-test/c8167005, check SHA256SUMS, run `sh run-test.sh` there. Nothing is installed; no
  app is started; Bannerlator untouched.
- **Resume point:** if the session dies: re-run the stage/unpack/run commands above as
  `rootsh "su -g 10183 -G 3003 -G 9997 -G 20183 -G 50183 10183 -c 'runcon u:r:untrusted_app_27:s0:c183,c256,c512,c768 /system/bin/sh -c \"...\"'"`;
  results land in ~/yserver-test/c8167005/out-*/ (summary.txt, xprobe.txt, yserver.log, rec/).
- **Resume point:** repo /root/src/yserver-android-spike; dispatch `build` (mode check first) on
  feat/android-build; assert headSha.
