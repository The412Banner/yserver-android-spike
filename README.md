# yserver for Android

Android (aarch64, bionic) builds of [yserver](https://github.com/joske/yserver), the X11 server written
in Rust by **Jos Dehaes**, made for [Bannerlator](https://github.com/The412Banner/Bannerlator). In
Bannerlator, yserver can take the place of the built-in Java X server for a game: Wine keeps using its
X11 driver (winex11), and yserver shows Wine's windows through Bannerlator's own Wayland compositor.

This repository holds **no yserver source**. Each build downloads yserver at a pinned commit, merges
**SetZero**'s rootless Wayland host ([joske/yserver#349](https://github.com/joske/yserver/pull/349)) at a
pinned commit, and applies the Android patches kept in [`patches/`](patches).

## Credits and licence

| What | Who | Licence |
|---|---|---|
| [yserver](https://github.com/joske/yserver) — the X server | Jos Dehaes | MIT ([`LICENSE-yserver`](LICENSE-yserver)) |
| Rootless Wayland host ([joske/yserver#349](https://github.com/joske/yserver/pull/349), open pull request) | SetZero | submitted to yserver (MIT) |
| Android patches, build scripts, test programs (this repository) | The412Banner | MIT ([`LICENSE`](LICENSE)) |

Built binaries contain all three and carry both licence files. Bannerlator (GPL-3.0) bundles the binary
with its licence in `assets/yserver/LICENSE` and `NOTICE_YSERVER.txt`.

## How Bannerlator uses it

- **Turning it on.** Add `BANNER_YSERVER=1` to a game's (or a container's) environment variables.
  `0`, `off`, or no variable means off. Without it nothing changes: the game runs on Bannerlator's Java
  X server (X11 mode) or on Wine's Wayland driver (Wayland mode), as configured.
- **What runs.** Bannerlator starts its Wayland compositor, then yserver as
  `yserver :0 -nolisten tcp --rootless` connected to that compositor. Wine (winex11) connects to
  yserver as an ordinary X server. Each top-level X window (with Wine's virtual desktop: the desktop
  window) becomes one Wayland surface of the compositor.
- **Which binary.** `files/yserver/yserver` in Bannerlator's app data, when present (for testing new
  builds by hand), otherwise the bundled `libyserver.so` from the APK's native library folder. With
  Bannerlator's `feat/yserver-bundle` branch (built and staged, not merged yet), its CI downloads the
  release named in `ci/yserver.json` and checks its SHA-256 before packaging it.
- **Keyboard data.** `files/yserver/xkb` when present, otherwise the XKB data already inside
  Bannerlator's Wine image (`imagefs/usr/share/X11/xkb`).
- **What Bannerlator passes to yserver.** `YSERVER_TMPDIR`, `XKB_CONFIG_ROOT`, `XDG_RUNTIME_DIR` +
  `WAYLAND_DISPLAY` (the compositor), `RUST_LOG=info`, `YSERVER_PRESENT_ASYNC=1`, `YSERVER_FPS_FILE`
  (its FPS limiter), and for game shortcuts `YSERVER_HIDE_CLASS=explorer.exe`. Any `YSERVER_*` or
  `RUST_LOG` variable set on the game is passed through as well.
- **What yserver tells Bannerlator.** Two log lines on its standard output, which Bannerlator reads:
  `YSERVER-MAP window=… hwnd=… pid=… wow64=… class=…` when a Wine application window is mapped
  (Bannerlator then brings it to the front inside Wine and applies the launch CPU mask), and
  `YSERVER-FPS window=… fps=…` once a second (the game's own frame rate, shown on Bannerlator's HUD).
  yserver's log is kept in `files/yserver/logs/`.

## Releases

| Release | Built from | Contents |
|---|---|---|
| [`r1-pre1`](https://github.com/The412Banner/yserver-android-spike/releases/tag/r1-pre1) (prerelease) | commit `77446386` (patches 0001–0029), CI run 43064600562; yserver `031ec58d`, PR #349 `6f3c46ff` | `yserver` (11,761,400 bytes, SHA-256 `cdb1aac0f1dd4b272c7e8386231c2037782ab659f70167388cf45d6d75b97872`), the full package, `SHA256SUMS`, both licences |

The full package (`yserver-android-<commit>.tar.gz`) holds the stripped `yserver`, the test programs
`xprobe`, `ahbprobe` and `vkprobe`, `run-test.sh`, the XKB data from the build machine, `VERSION`
and `SHA256SUMS`.

## How a build works

[`.github/workflows/build.yml`](.github/workflows/build.yml), on Ubuntu with the Android NDK (API 26,
aarch64):

1. **Tools** — `ninja`, `bison`, `xkb-data`, `glslc` from Ubuntu; when Ubuntu's mirror is unreachable,
   `glslc` comes from the NDK's `shader-tools` and `ninja` from PyPI. `meson` from PyPI.
2. **yserver source + patches** — clone joske/yserver at [`YSERVER_COMMIT`](YSERVER_COMMIT), merge
   SetZero's branch at [`PR349_COMMIT`](PR349_COMMIT), then `git apply` every file in `patches/` in
   name order.
3. **libxkbcommon** — built as a static Android library ([`deps/build-xkbcommon.sh`](deps/build-xkbcommon.sh)).
4. **Stub libraries** — empty `libgbm`, `libinput` and `libudev` with the needed symbols
   ([`deps/gen-stubs.sh`](deps/gen-stubs.sh), [`deps/stub-syms/`](deps/stub-syms)) so yserver links;
   with no DRM card they are never called. [`deps/xshmfence.c`](deps/xshmfence.c) is a small futex-based
   xshmfence for DRI3 fences.
5. **Compile** — `cargo build --release --target aarch64-linux-android -p yserver --features wayland`,
   plus the test programs in [`client/`](client).
6. **Package** — strip, collect, hash; upload the package (artifact `yserver-android`, 14 days) and the
   unstripped binary (artifact `yserver-android-symbols`, 7 days).

A build runs on every push to a `feat/**` or `fix/**` branch (commits that only touch `PROGRESS_LOG.md`
are skipped). The manual run (`mode: build` or `mode: check`) is available once the workflow is on the
default branch.

## Repository layout

| Path | What |
|---|---|
| `patches/` | The Android changes to yserver, applied in name order (below) |
| `parked/` | Patches kept out of the build (empty at `r1-pre1`) |
| `deps/` | libxkbcommon build script, stub libraries, xshmfence |
| `client/` | `xprobe` (fixed X11 protocol checks, PASS/FAIL lines, exit code 0 only when nothing failed), `ahbprobe` (AHardwareBuffer DRI3 import, copies and Present), `vkprobe` (Vulkan device and queue report); all use x11rb |
| `scripts/run-test.sh` | Runs yserver headless with its frame recorder, then `xprobe` |
| `scripts/step2-device.sh` | The device test used in the early sandbox runs |
| `YSERVER_COMMIT`, `PR349_COMMIT` | The pinned source commits |
| `PROGRESS_LOG.md` | Every step, measurement and mistake, newest first |

## The patches

**Android build**

| Patch | Change |
|---|---|
| 0001 | Builds for `target_os = "android"`: anonymous shared memory through `memfd_create`, MIT-SHM SysV attach refused (Android has no SysV shared memory; fd-based MIT-SHM works), Linux-only code paths enabled for Android, bundled FreeType, `YSERVER_TMPDIR` for the socket and lock (Android has no `/tmp`) |
| 0002 | Fonts from `/system/fonts` (no fontconfig on Android); `YSERVER_FONT_DIRS` overrides the list |
| 0003 | The DRI3 ioctl request type bionic expects |
| 0004 | Graphics and compute queues count as transfer-capable. Adreno's proprietary Vulkan driver does not report `TRANSFER` on its graphics queue, which the Vulkan spec allows, and yserver refused the device |
| 0005 | DRI3 import of Android hardware buffers sent the way Bannerlator's Vulkan wrapper sends them (modifier 1255: the buffer travels over a socket pair) |
| 0006 | RGBA hardware buffers copied into BGRA windows through a converting blit |
| 0007 | `YSERVER_RECORD_EVERY` / `YSERVER_RECORD_MAX` limit the headless frame recorder |
| 0008 | Builds the zero-card KMS platform and PR #349's rootless Wayland host on Android, with the stub gbm/libinput/udev |

**Showing frames through Bannerlator's compositor**

| Patch | Change |
|---|---|
| 0009, 0009a | Each top-level window reaches the compositor as Android hardware buffers: a GPU copy into a pool of three, handed over as a dma-buf `wl_buffer` plus Bannerlator's `banner_ahb_v1` protocol (so the compositor can put it on a display layer without another copy). `YSERVER_WAYLAND_CPU=1` sends CPU frames instead |
| 0012 | (Superseded by 0020) waited for that copy so the buffer reached the compositor in the same refresh |
| 0013, 0014 | Copies go through `vkCmdBlitImage` on Android (`YSERVER_COPY_BLIT`): Adreno's proprietary driver allocates and clears GPU memory inside every `vkCmdCopyImage`, which cost ~1.8 ms per present |

**Present timing and the FPS limiter**

| Patch | Change |
|---|---|
| 0010 | `YSERVER_PRESENT_ASYNC=1`: presents run at once, like Bannerlator's Java X server, instead of waiting for the host's refresh |
| 0011 | `YSERVER_PRESENT_HZ`: the rootless Present clock runs at that rate (frame cap) |
| 0016 | With a frame cap, a window's presents take one Present clock tick each, which also holds mailbox clients to the cap |
| 0017 | The cap follows Bannerlator's FPS limiter live through `YSERVER_FPS_FILE` (read a few times a second; `0` = no cap). Without a cap, the client's target MSC is ignored, so FIFO games are not rounded up to refresh slots |
| 0024, 0025, 0027, 0028 | **Newest frame only** (`YSERVER_PRESENT_LATEST`, on unless `0`; only with `YSERVER_PRESENT_ASYNC=1` and no cap): a present that asks for no particular frame (the async option — Vulkan IMMEDIATE / FIFO_RELAXED — or target 0 — Vulkan MAILBOX) waits for the next host frame, and a newer present for the same window replaces it; the replaced buffer goes back to the game uncopied. The window gets one copy per host frame instead of one per game frame. Presents with an explicit target (Vulkan FIFO) still run at once |

**Bannerlator integration**

| Patch | Change |
|---|---|
| 0014, 0015, 0019 | The `YSERVER-MAP` line for mapped Wine application windows (own window group, bigger than 1×1): window, `_NET_WM_HWND`, `_NET_WM_PID`, `_NET_WM_WOW64`, class |
| 0022, 0023 | `YSERVER_HIDE_CLASS=<class>[,<class>…]`: windows whose `WM_CLASS` contains one of the names draw nothing and clear to black on every exposure, so Wine's desktop (`explorer.exe`) stays black as in Bannerlator's other modes. Windows of other classes inside it draw normally |
| 0022, 0029 | The `YSERVER-FPS` line once a second: the presents per second of the busiest window, counted when they arrive (newest-frame-only copies fewer) |

**Speed**

| Patch | Change |
|---|---|
| 0020 | No more multi-millisecond stalls of yserver's single core thread: a clipped `PutImage` with the plain copy function uploads each clip run on the GPU (the old path read the destination back and waited for the busy GPU, 2–5 ms each; `YSERVER_PUTIMAGE_CLIP_CPU=1` restores it), and no blocking wait after the frame copy (`YSERVER_COPY_WAIT_US` restores one) |

**Diagnostics (inert unless switched on)**

| Patch | Switch | What it reports |
|---|---|---|
| 0018 | `YSERVER_DEBUG_SKIP_PRESENT_COPY=1` | Skips full-window present copies, to measure their cost (the picture stops updating) |
| 0021, 0022 | `YSERVER_TICK_TIMING=1` | Once a second, the longest and total time of each backend step between requests |
| 0026 | `RUST_LOG=…,present_pace=debug` | Why a parked present was or was not replaced |

yserver's own `YSERVER_LOOP_TELEMETRY=1` (per-second core-loop statistics) works too.

## Measurements

AYANEO Pocket FIT (Snapdragon, Adreno 750, 144 Hz screen), Bannerlator, Wine/Proton with DXVK, Wine
desktop 1280×720. Single runs; DiRT varies by a few percent between runs. "X11" is Bannerlator's Java X
server, "Wayland" is Wine's Wayland driver.

| Test | yserver up to 0019 | yserver with 0020–0029 | X11 | Wayland |
|---|---|---|---|---|
| AIO Vulkan cube, 30 s, average fps | 2,071–2,120 | 2,051–2,194 | 2,165–2,181 | 2,776 |
| AIO Vulkan cube, slowest 1 % (fps) | 304–323 | 554–595 | 828–847 | — |
| GTA IV benchmark (fps) | 66.0 | 65.9 | 66.1 | 65.4 |
| DiRT Showdown benchmark (fps) | 116.8 | 110.2 (r1-pre1), 113.6 (0020) | 114.3 | 113–117 |

The `r1-pre1` binary itself measured 2,057.7 fps average and 554 fps slowest 1 % in the cube, 65.9 in
GTA IV and 110.2 in DiRT.

- GPU load in the cube with newest frame only: 24–25 % (about 30 % without it); about 93 % of the
  cube's presents were replaced before a copy.
- The Windows Steam client (Chromium pages drawn by one process into another's window) renders on
  yserver; on Wine's Wayland driver it stays white or black.
- The remaining gap to X11 in the slowest 1 % comes from about 3–4 ms once a second while the test app
  redraws its title and label, and up to about 2 ms in the driver while reclaiming finished GPU work
  (measured with `YSERVER_TICK_TIMING`).

## Limits and untested

- Tested on one device (Adreno 750, Adreno's proprietary Vulkan driver). Other GPUs: not tested.
- OpenGL games through GLX are not supported on yserver (Bannerlator uses its EGL route there).
- Not tested yet: typing on the keyboard, clipboard copy/paste end to end, changing the FPS limiter
  during a game, HDR, frame generation.
- PR #349 is an open pull request upstream; this build follows the commit in `PR349_COMMIT`.

## Building and testing it yourself

Push a `feat/<topic>` branch with your changes to `patches/` (one file per change, numbered after the
last) and download the `yserver-android` artifact from the run. To try a build in Bannerlator without a
new APK, copy `yserver` to Bannerlator's `files/yserver/yserver` (owned by the app, mode 700) and set
`BANNER_YSERVER=1` on a game. To test the server alone on a device, run `run-test.sh` from the package:
it starts yserver headless with its frame recorder and runs `xprobe` against it.
