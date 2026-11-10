# Progress log

Times are EST (America/New_York).

## Lessons learned (don't repeat these)
- `workflow_dispatch` only works once the workflow file is on the default branch; with work kept off
  main, the workflow runs on push to feat/** instead (log-only commits skip it via paths-ignore).
- Adreno 750 (Qualcomm blob 1.3.128) does not set VK_QUEUE_TRANSFER_BIT on its graphics queue
  (allowed by the spec); code that demands the bit finds "no suitable device".
- Running a test "as Termux" through root keeps root's SELinux domain (u:r:magisk:s0); add
  `runcon u:r:untrusted_app_27:s0:c183,c256,c512,c768` after `su 10183` to get Termux's real sandbox.

## 2026-11-10 — ✅ Step 2 PASSED on the Pocket FIT (feat/android-build 9460d31f, device-proven headless)
- **Device run 2 (18:41:39 EST, package 9460d31f, uid 10183 in u:r:untrusted_app_27:s0:c183,c256,c512,c768):**
  SHA256SUMS ok. **xprobe 15/15 PASS**, run-test exit 0.
  - vkprobe: 1 device, "Adreno (TM) 750", Vulkan 1.3.128, driver build 43c70540de (07/17/24);
    queue family 0 = GRAPHICS|COMPUTE|SPARSE_BINDING|OPTICAL_FLOW_NV|PROTECTED (no TRANSFER bit →
    confirms the run-1 cause; patch 0004 fixes it), family 1 COMPUTE, family 2 SPARSE_BINDING.
    Extensions: AHB import yes, external_memory_fd yes, external_semaphore_fd yes, timeline yes;
    dma_buf NO, image_drm_format_modifier NO, physical_device_drm NO.
  - yserver: core Vulkan 1.3 on the Adreno blob; socket ready 230 ms; RSS 45.3 MB idle,
    52.4 MB after the probe (peak 54.6 MB); 2,968 font names from /system/fonts (log line still says
    "fontconfig"); input devices named "macOS keyboard/pointer" (portable backend defaults).
  - 21 extensions offered: RANDR RENDER GE BIG-REQUESTS XINERAMA XKEYBOARD XInputExtension XFIXES
    SHAPE SYNC DAMAGE Composite Present DPMS MIT-SCREEN-SAVER MIT-SHM XTEST X-Resource XC-MISC
    VidMode (+1 cut off). Missing for Wine: DRI3, GLX (no DRM render node / dma-buf on Android).
  - xprobe: connect 0.5 ms; window mapped + exposed 27 ms; fill + PutImage + MIT-SHM AttachFd
    (v1.2) read back exactly; XKB 1.0; keymap 8..255, 677 keysyms; unknown core request → BadRequest
    and the stream stays in step; OpenFont "fixed" opens; 10,000 round trips 173.4 ms = 17.3 µs
    each; PutImage 572 MB/s; recorder 68 frames, last one matches.
  - After: crash buffer empty, newest tombstone 12:09 (before the run), no yserver process left,
    0 files not owned by uid 10183, yserver.log 0 error/panic lines, clean SIGTERM shutdown.
- **Proof level:** device-proven for a headless yserver + test client inside Termux's app sandbox.
  NOT tested: a real display, Wine, game frames (DRI3/AHB), touch/controller input, Bannerlator.
- **Left on the device:** ~/yserver-test/{c8167005,9460d31f} in Termux's home (packages + outputs),
  /sdcard/Download/yserver-android-spike-{c8167005,9460d31f}.tar.gz and
  yserver-android-spike-step2.sh. Nothing installed, Bannerlator untouched.
- **Open / next (user decides):** step 3 (Wine through yserver in a Bannerlator test build), the
  AHB DRI3 path, an Android display host; the TRANSFER-bit check is an upstream yserver bug worth
  reporting (author not contacted). Branch not merged.

## 2026-11-10 — Steps 0–2 started (feat/android-build)
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
- **Device run 1 (2026-11-10 18:35 EST, Pocket FIT, package c8167005, uid 10183 in
  u:r:untrusted_app_27:s0:c183,c256,c512,c768):** SHA256SUMS ok; yserver STARTS on Android
  (`startup — yserver 1.9.0`) but exits at once: `vulkan: no suitable physical device (need a
  graphics + transfer queue)`. yserver requires the TRANSFER bit explicitly
  (`yserver-render/src/kms/vk/device.rs:1705`); the Vulkan spec makes it optional for graphics /
  compute queues. Not yet known whether Adreno omits it or the sandbox sees no device at all.
  RUST_LOG=debug added nothing. Next build: patch 0004 (graphics/compute imply transfer) + `vkprobe`
  (devices, queue flags, external-memory extensions), run first in run-test.sh.
- **CI run 42924919473** (headSha 9460d31f): ✅ green; package `yserver-android-9460d31f.tar.gz`
  sha256 `ce4b2cd844c97a3f051c771e0862165cf2ec6aa5b6f112b4ea2333f2c55ddfce`, staged
  /sdcard/Download/yserver-android-spike-9460d31f.tar.gz (host == device). Device run 2 next:
  `sh /sdcard/Download/yserver-android-spike-step2.sh 9460d31f` in the sandbox (same command as run 1).
- **Device test (step 2, Pocket FIT):** planned as — stage to /sdcard/Download/yserver-android-spike-c8167005.tar.gz
  (hash host == device), unpack as uid 10183 in Termux's app context into
  ~/yserver-test/c8167005, check SHA256SUMS, run `sh run-test.sh` there. Nothing is installed; no
  app is started; Bannerlator untouched.
- **Resume point:** if the session dies: re-run the stage/unpack/run commands above as
  `rootsh "su -g 10183 -G 3003 -G 9997 -G 20183 -G 50183 10183 -c 'runcon u:r:untrusted_app_27:s0:c183,c256,c512,c768 /system/bin/sh -c \"...\"'"`;
  results land in ~/yserver-test/c8167005/out-*/ (summary.txt, xprobe.txt, yserver.log, rec/).
- **Resume point:** repo /root/src/yserver-android-spike; dispatch `build` (mode check first) on
  feat/android-build; assert headSha.
