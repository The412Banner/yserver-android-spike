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
- **CI run:** (next)
- **Device test:** not started.
- **Resume point:** repo /root/src/yserver-android-spike; dispatch `build` (mode check first) on
  feat/android-build; assert headSha.
