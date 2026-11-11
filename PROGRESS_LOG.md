# Progress log

Times are EST (America/New_York).

## Lessons learned (don't repeat these)
- (BL repo) the pre-launch checkpoint helper ran `git commit -am` in the BL worktree and swept unfinished Java edits into a
  "PROGRESS_LOG: launch checkpoint" commit (aa1cb3e9 = the live FPS-limiter code). Commit only PROGRESS_LOG.md in helpers.
- The local tree has files that earlier patches CREATE (e.g. 0005's ahb.rs). If they stay untracked, `git diff`
  for the next patch misses edits to them (0009 CI 42949396814 failed). Commit them locally first, and check a
  fresh `031ec58d + merge PR349 + all patches` tree equals the local tree before pushing.
- `workflow_dispatch` only works once the workflow file is on the default branch; with work kept off
  main, the workflow runs on push to feat/** instead (log-only commits skip it via paths-ignore).
- Adreno 750 (Qualcomm blob 1.3.128) does not set VK_QUEUE_TRANSFER_BIT on its graphics queue
  (allowed by the spec); code that demands the bit finds "no suitable device".
- Running a test "as Termux" through root keeps root's SELinux domain (u:r:magisk:s0); add
  `runcon u:r:untrusted_app_27:s0:c183,c256,c512,c768` after `su 10183` to get Termux's real sandbox.

## 2026-11-11 10:0x — tick timing result; renice experiment checkpoint
- speed4 (0021 ff704741, fan max): avg 2162.39 / 1% 582.99. Per second: rootless_tick max 3.4-3.5 ms (sum ~36 ms), reclaim_gpu max 2.3 ms (sum ~28 ms), rest < 0.05 ms. VRAM churn alloc=0/s free=0/s, get_image_calls/s=0 → spikes are not extra work (avg 13-17 us per step); suspect preemption by the game's threads (yserver runs at nice 0; BL's compositor thread runs at nice -10).
- Launch checkpoint: speed5 = AIO vk ys 30 s with all yserver threads reniced to -10 by root (perf-harness/ys-renice.sh) during the run.

## 2026-11-11 10:0x — games on 0020 OK; 0021 installed; launch checkpoint
- ys-game speedg on d88dac66 (0020): GTA IV 65.92 (before 66.03), DiRT 110.11 (before 116.8; DiRT run after a cooldown, ThermalStatus/heat not controlled — rerun with the fan at max). 0 FATAL, crash 0.
- Fan rule (user): no cooldowns; perf-harness/fan-max.sh (root loop pwm1=255, AYANEO rewrites it) before test blocks, fan-restore.sh after. Fan loop RUNNING now.
- 0021 CI 43041661192 green; yserver ff704741 staged /sdcard/Download/yserver-android-0021-95cd7778 and installed.
- Launch checkpoint: speed4 = AIO vk ys 30 s YSERVER_TICK_TIMING=1 YSERVER_LOOP_TELEMETRY=1.

## 2026-11-11 09:5x — 0020 A/B RESULT + 0021 debug timing; game launch checkpoint
- AIO vk ys 30 s on d88dac66: speed2 (0020 on) avg 2187.73 / 1% 581.45, longest request op145 0.14-0.27 ms, iter_wall_max ~3.8 ms; speed3 (same binary, YSERVER_PUTIMAGE_CLIP_CPU=1 YSERVER_COPY_WAIT_US=4000) 2118.01 / 328.84, op72 4.1-4.5 ms, iter_wall_max ~10 ms; speed0 (old 4e229a33) 2070.99 / 304.03. X11 ref 2164.97 / 827.94.
- Remaining ~3.8 ms iteration stall is outside requests → 0021 (95cd777): YSERVER_TICK_TIMING=1 per-step timing; CI 43041661192.
- Launch checkpoint: GTA IV + DiRT Showdown benchmarks on yserver d88dac66 (ys-game.sh gta|dirt speedg ys) for regressions (previous ys: GTA 66.03, DiRT 116.8).

## 2026-11-11 09:4x — 0020 built + staged; A/B launch checkpoint
- speed1 (old binary, YSERVER_DEBUG_SKIP_PRESENT_COPY=1): avg 2014.18, 1% 325.70, stalls unchanged (op72 4-5 ms, iter_wall_max ~10.5 ms) → removing the present copy alone gains nothing; direct hand-off (flip) parked.
- CI 43037424736 failed twice in "Tools" (runner DNS: azure.archive.ubuntu.com unresolvable) → workflow fallback 9cfe039 (glslc from NDK shader-tools, ninja from PyPI); CI 43039744744 GREEN (fallback used). yserver d88dac66 (contains YSERVER_PUTIMAGE_CLIP_CPU + YSERVER_COPY_WAIT_US), staged /sdcard/Download/yserver-android-0020-9cfe0394, installed in BL files/yserver (owner/label kept). Old binary e5bbb132 = /sdcard/Download/yserver-android-FINAL-4e229a33 (undo: cat it back).
- Launch checkpoint: speed2 = AIO vk ys 30 s new defaults; speed3 = same with YSERVER_PUTIMAGE_CLIP_CPU=1 YSERVER_COPY_WAIT_US=4000 (old behaviour).

## 2026-11-11 09:4x — speed job: measurement 1 + patch 0020 (CI 43037424736 building)
- speed0 (AIO vk ys 30 s, telemetry, yserver 4e229a33): avg 2070.99, 1% low 304.03. Loop: busy ~23 %, but iter_wall_max ~10 ms EVERY second; longest request op72 (PutImage) 4-5 ms (14/s, 28 ms/s); op145 (Present) 2144/s, 192 ms/s (0.09 ms each).
- Cause of slow PutImage: Wine's window-surface upload to the AIO window is clipped by its Vulkan child → draw.rs put_image (has_clip || children_clip) → put_image_rop_cpu → GPU readback (waits for the game's GPU work).
- 0020 (ed3fb44): clipped Copy PutImage per run on the GPU (YSERVER_PUTIMAGE_CLIP_CPU=1 = old path); ahb.rs copy wait off by default (YSERVER_COPY_WAIT_US = old wait). Local cargo check green.
- Launch checkpoint: AIO vk ys 30 s with YSERVER_DEBUG_SKIP_PRESENT_COPY=1 on the old binary (upper bound of removing the present copy). Restart: rootsh "sh /sdcard/Download/perf-harness/ys-aio.sh vk speed1 ys 30 'YSERVER_LOOP_TELEMETRY=1 YSERVER_DEBUG_SKIP_PRESENT_COPY=1'".

## 2026-11-11 09:3x — yserver speed job START (user: "do the yserver speed work"; feat/android-build 5341dae, yserver 4e229a33 on the FIT)
- Why: yserver = X11 speed in games, but cube avg 2120 vs Wayland zero-copy 2776 and 1% low 323 vs X11 847.
- Suspects (code read): rootless/ahb.rs tick waits up to COPY_WAIT_NS = 4 ms on the core thread for each window→AHB copy (all clients stall); every frame is copied twice inside yserver (Present pixmap → window backing, backing → pool AHB).
- Plan: 1 measure (AIO vk ys 30 s, YSERVER_LOOP_TELEMETRY=1: iter_wall_max); 2 no blocking wait; 3 hand the game's own AHB straight to the compositor when its pixmap covers the whole window; build in CI, stage files/yserver (backup first), A/B on the FIT.
- Launch checkpoint: AIO vk ys 30 s telemetry run (BL 4e8c7e65 = main 02577e6a). Restart: rootsh "sh /sdcard/Download/perf-harness/ys-aio.sh vk speed0 ys 30 YSERVER_LOOP_TELEMETRY=1".

## 2026-11-11 01:3x EST — ✅ FINISHED: yserver 4e229a33 (0001–0019) + BL 931c441d device-proven; parity with X11
- GTA IV CPU-mask fix (BL 95325269): ys 66.03 (= X11 66.1, Wayland 65.4). DiRT 116.8. Cube 2120 vs X11 2181. Frame cap 60.01. Steam client renders (store + friends). Click → Start menu works. Clipboard: compositor now offers ext_data_control_v1 (BL 931c441d), yserver's "no clipboard" warning gone (copy/paste not tried).
- Final staged: /sdcard/Download/yserver-android-FINAL-4e229a33 (e5bbb132…) = files/yserver/yserver on the FIT.
- Open: report the Adreno TRANSFER-bit queue bug upstream; ask Jos Dehaes + SetZero before anything public; 1 % lows in the cube lower than X11 (323 vs 847 fps); live limiter untested.

## 2026-11-11 00:5x EST — GTA IV gap = game main thread pinned to CPU 0 (no launch CPU mask in yserver mode); 0018/0019
- Ruled out: compositor copy (zero-copy 40.8), present-wait (41.6), async (41.0), immediate mode (41.1), target_msc (0017: GTA sends target 0), yserver's copy (0018 YSERVER_DEBUG_SKIP_PRESENT_COPY=1 → 43.0). GPU per frame equal (X11 61 % @15.2 ms ≈ 9.3 ms; ys 37 % @24.4 ms ≈ 9.0 ms).
- Found (ys-cpu.sh, p5f): ys → GTAIV.exe main thread `Cpus_allowed_list: 0` (the game pins itself to CPU 0, a 2.27 GHz little core) and it is the busiest thread; X11 → 0-7, heavy thread on core 2. X11 mode resets it via assignTaskAffinity (winhandler SetProcessAffinityMask on every app-window map), Wayland via the compositor's game report — in yserver mode that report names yserver ("launch CPU affinity for yserver").
- **0019:** YSERVER-MAP carries `wow64=` (_NET_WM_WOW64). **BL 95325269:** assignYserverTaskAffinity(class, pid, wow64) on YSERVER-MAP (same as X11); compositor's assignWaylandTaskAffinity skipped in yserver mode. Steam client on yserver: bootstrap "Updating Steam" window drew fine (run cut at 100 s mid-update).

## 2026-11-11 00:2x EST — frame cap works; zero-copy layer works; GTA IV 40 vs 66 → FIFO target_msc rounding (0017)
- **Frame cap (0016 + BL 77f321d6, limiter 60):** AIO cube avg 60.01 fps, 16.67 ms ± 1.39 ms, worst 22.96 ms; uncapped 2004 fps.
- **Zero-copy layer (BANNER_WAYLAND_ZERO_COPY=1):** compositor shows yserver's AHB pool on a display layer ("978 zero-copy frames, copy 0"); DiRT 117.0 fps (GPU-bound, same as copy path 116.8).
- **GTA IV benchmark:** ys 40.1 / wl 65.4 / x11 66.1 (0 FATAL). Not the compositor copy (zero-copy 40.8), not present-wait (41.6), not async on/off (41.0); yserver 1 % busy, each present done in ~1 ms, GPU 37 % → the game waits. Cause: Mesa FIFO sends explicit target_msc = last+1; yserver honoured them on its 144 Hz clock (async override only covered target 0), so a ~15 ms frame rounds up to 3–4 slots (~24 ms). The Java X server ignores target_msc → 66.
- **0017:** immediate mode (YSERVER_PRESENT_ASYNC=1, no cap) ignores target_msc/divisor like the Java server; frame cap live through `YSERVER_FPS_FILE` (yserver-core `frame_cap`, polled every 200 ms; Present clock + one-MSC-per-present follow it; async off while capped). BL: applyFpsLimit → YServerComponent.setFrameCapLive (writes imagefs/usr/tmp/.yserver-fps); compositor limiter not used in yserver mode.

## 2026-11-10 23:47 EST — ✅ DiRT Showdown on yserver: 116.8 fps (= X11 114.3 / Wayland 113–117), activation fixed
- Window activation chain (BL): YSERVER-MAP (0014/0015) → winHandler.bringToFront(class, hwnd) (+1 s); explorer.exe / conhost.exe never raised; when one maps (Wine activates the console window itself ~9 s into DiRT), the last game window is raised again after 0.3 s and 1.3 s (BL d948577f). Proof p4a: game 108–113 fps until conhost mapped at 23:22:36 → 16; p4c with re-raise: **av 116.8, min 85.5**, 0 FATAL, no hands.
- Note: the X11 (Java) path has the same console-steals-activation behaviour (DesktopHelper raises every app window) → its 16-fps runs tonight; pre-existing, not touched.
- 0016 (frame cap) + BL FPS limiter → YSERVER_PRESENT_HZ wiring pushed (CI).

## 2026-11-10 23:1x EST — blit fixes the Present cost; DiRT "16 fps" = inactive window (user found it); 0014/0015
- **0013 YSERVER_COPY_BLIT=1 on device (be806b03):** PresentPixmap 1.84 ms → ~0.1 ms (op145 416/s in 41.8 ms; core 7.5 % busy); cube 407 fps synced, HUD 142 fps on screen (full 144 Hz). With YSERVER_PRESENT_ASYNC=1 too: **2093 fps** (X11 Java 2062, Wayland 2677).
- **DiRT Showdown benchmark (800x600 windowed):** x11 114.3 / wl 113.1 at 22:2x, then ys AND later x11 runs at 16 fps with GPU 10 %. Traced: yserver handles each Present in ~3 ms, the game waits ~65 ms between them. **User (23:0x): "every time we ran yserver with the game windowed it was 16 FPS with no sound until I maximized the window during the last two runs"** → the game throttles + mutes while its window is inactive; maximizing activated it: x11 105.7, ys 92.3 (ys run still on the vkCmdCopyImage path). Wayland mode activates via the compositor's bring-to-front; X11 mode via DesktopHelper on map; yserver mode had nothing.
- **0014:** blit copies default on Android; `YSERVER-MAP window= hwnd= pid= class=` info line (target yserver::bannerlator) when a window with _NET_WM_HWND maps. **0015:** only application windows (WM_HINTS window_group == self, >1x1), like the Java server.
- **BL 7938d479:** YServerComponent reads yserver output through a pipe (log file + YSERVER-MAP → winHandler.bringToFront(class, hwnd), again after 1 s), YSERVER_PRESENT_ASYNC=1 default.
- Side findings: DXVK's 2nd swapchain is FIFO in both modes; `MESA_VK_WSI_PRESENT_MODE=immediate` also hid the problem (89 fps). kgsl "CP: AHB bus error" lines appear in every run incl. the X11 cube (harmless here).

## 2026-11-10 22:2x EST — P2b on device: AHB hand-off works; Adreno blob vkCmdCopyImage allocates per call (profiled)
- **0009 on device (yserver 47ef5361, BL c72254b5):** "goes to the host as AHardwareBuffers (1280x720, 3 buffers)", picture + colours correct, compositor HUD now shows (GPU buffers). AIO vk cube 500x500: 351 fps (was 153 CPU frames); HUD 79.8 fps on screen.
- **0012 (copy wait ≤4 ms):** HUD 101.4 fps on screen (9.9 ms). Cube 4642/15 s (309 fps); `YSERVER_PRESENT_ASYNC=1` 4452 (no change → not the MSC wait).
- **Loop telemetry (YSERVER_LOOP_TELEMETRY=1):** PresentPixmap (op145) ≈ 1.84 ms each, 59 % of the core thread; vk submit CPU only ~22 µs.
- **simpleperf (unstripped b1e83a57):** 89 % kernel: `vkCmdCopyImage` (qglinternal) → `gsl_memory_alloc_pure_64` → `kgsl_ioctl_gpuobj_alloc` → shmem pages → `clear_page` (60 % inclusive); `vkFreeCommandBuffers` → `ioctl_kgsl_sharedmem_free` (26 %). I.e. the Qualcomm driver allocates (and zeroes) GPU memory inside every vkCmdCopyImage of these images, freed with the command buffer. Same would hit every game frame (3.7 MB at 1280x720).
- **0013:** `YSERVER_COPY_BLIT=1` → engine copy + pool copy via vkCmdBlitImage, to test whether blit avoids it (else: shader copy).

## 2026-11-10 22:0x EST — P1 works on device; P2b (GPU copy into AHB pool) written; CI 42949724435 building
- **P1 device result (BL c72254b5 + yserver 80215caa):** yserver connected to Bannerlator's compositor: root 1280x720 (= container size), Present clock 144 Hz from the compositor, Wine desktop = xdg_toplevel, keyboard focus, DRI3 AHB pixmaps imported, clipboard missing (compositor has wlr data-control, not ext). Picture correct (Wine desktop + cube). AIO vk cube 500x500 15 s: X11 2062 fps, Wayland 2677 fps, yserver 153 fps (p1b). 0 FATAL.
- **Two limits:** (1) CPU readback into wl_shm; (2) yserver Present runs a synced present at the next host refresh (144 Hz) — Xorg semantics; the Java server ran them at once.
- **0009 (+0009a):** HandoffKind::Ahb on Android when the compositor has zwp_linux_dmabuf_v1 + banner_ahb_v1: per window a 3-buffer pool of AHardwareBuffers (format 5 BGRA, GPU_SAMPLED|GPU_FRAMEBUFFER|COMPOSER_OVERLAY|CPU_READ_RARELY = linear), GPU copy backing → buffer (record_backing_copy), fence polled (no CPU wait), then dma-buf wl_buffer (XRGB/ARGB8888, linear, fd = native handle data[0]) + banner_ahb_v1.attach (handle over a socketpair) once per buffer, attach/damage/frame/commit; wl_buffer.release frees the slot. `YSERVER_WAYLAND_CPU=1` keeps CPU frames.
- **0010:** `YSERVER_PRESENT_ASYNC=1` treats next-field presents as async (run now).
- BL 4e582b35: YSERVER_* / RUST_LOG from the game's env reach yserver (CI 42949617515).

## 2026-11-10 21:3x EST — ▶️ DISPLAY JOB started: step 1 (P0) = Android build of the rootless Wayland host (upstream PR #349)
- **User (21:2x EST):** "yes start step 1 and do all of them until finished, with games running fast and fluid as possible with the best performance possible, stage an APK for me to test only when you finish ... use root to build and drive the device yourself, install and run. device is yours". → standing permission to install/launch on the Pocket FIT **for this yserver display job only**; still: save before every launch, tell root-6a/root-15 before taking BL over; final APK staged for the user at the end.
- **Plan (P0–P3):** P0 build PR #349 (SetZero, ferrix-os up/v2-pr4 @ 6f3c46ff) for aarch64-linux-android; P1 BL `BANNER_YSERVER` = compositor + yserver `--rootless` as its Wayland client, Wine on winex11; P2 zero-copy (AHB pixmaps → dma-buf wl_buffer + banner_ahb_v1); P3 clipboard/size/cursor.
- **Local tree:** session scratchpad `yserver/` branch android-wl = 031ec58d (master) + merge of 6f3c46ff (PR #349; merge-base 34574860; clean) + patches 0001–0007 (all apply in order). P0 change being tried: `cfg(kms)`/epoll/eventfd/wayland_host on for android (build.rs), drm/gbm/input + wayland-client/-protocols deps for android (Cargo.toml); udev/libinput/gbm resolved by stub libs (never called with zero cards + host seat). Local `cargo check` (glslc installed in the container; CC=clang --target=aarch64-linux-gnu just for build-script C during check; fake libudev.pc).

## 2026-11-10 — ✅ Device run 4 PASSED: AHB DRI3 BGRA + RGBA (feat/android-build bc0a13f0, device-proven headless)
- **Run 4 (19:30:28 EST, package bc0a13f0, Termux sandbox uid 10183):** SHA256SUMS ok. xprobe **15/15**,
  ahbprobe **10/10**: DRI3 1.2, Present 1.4; BGRA import ack 0.10 ms, CopyArea + Present exact (3.55 ms);
  **RGBA import ack 0.48 ms, CopyArea + Present exact (2.60 ms) = patch 0006 works**; 300-present loop
  330/s, 3.03 ms avg, worst 5.52 ms (lower than run 3's 508/s because the RGBA half now really blits;
  in run 3 those copies failed and did nothing); empty socket refused; RSS 45.6 → 52.2 MB.
  yserver.log 0 WARN, 0 panic/error lines; crash buffer empty; socket 118 ms. Recorder 69 frames (defaults
  unchanged by patch 0007).
- **NOT tested:** real screen, Wine, input. Next = step 3 (BL build feat/yserver-spike, CI 42931263093).

## 2026-11-10 — Device run 3: AHB DRI3 works for BGRA, RGBA fixed in patch 0006 (feat/android-build, CI pending)
- **Why:** user "continue yserver" (19:12 EST); session root-66 took over from the earlier session.
- **Device run 3 (19:14:04 EST, package 9ae0b21f, Termux sandbox uid 10183):** SHA256SUMS ok; xprobe 15/15
  PASS (round trip 61.8 µs vs 17.3 µs in run 2, PutImage 457 vs 572 MB/s; Discord + Bannerlator were open).
  yserver now lists **22 extensions incl. DRI3** (GLX still missing). ahbprobe 8 pass / 2 fail:
  - DRI3 1.2, Present 1.4; BGRA (AHB format 5) import ack 0.13 ms, CopyArea + PresentPixmap read back
    exactly (present 3.19 ms); 300-present loop 508/s, 1.97 ms avg, worst 10.34 ms; empty socket acked +
    refused (Alloc); server still answers; RSS 45 → 53 MB; no panic; crash buffer empty.
  - **FAIL rgba-copyarea / rgba-present:** RGBA (format 1) import accepted, but every copy from it failed:
    yserver.log 152x `engine.copy_area failed ... UnsupportedDepth(0)`. Cause: `engine/fill_copy.rs`
    `copy_area` rejects src_format != dst_format (R8G8B8A8 pixmap vs B8G8R8A8 window; it uses
    cmd_copy_image). The present-loop PASS hid this (it checks completion only, alternates both pixmaps).
  - Does RGBA matter? The wrapper (leegao/bionic-vulkan-wrapper 3abf4870 `wsi_common_ahardware_buffer.c`)
    sends B8G8R8A8 (format 5) normally, but **R8G8B8A8 (format 1, linear) in its blit mode** → yes.
- **Fix (patch 0006, 3 files +103/-1):** copy_area allows the R8G8B8A8/B8G8R8A8 pair (src != dst);
  `RecordedCopyArea.dst_format`; `emit_converting_copy_area` = same barriers with the BLIT stage +
  1:1 `cmd_blit_image` (NEAREST). AHB images already have TRANSFER_SRC usage, OPTIMAL tiling; blit
  src/dst are mandatory format features for both formats. All 6 patches apply to the pin (clean worktree).
- **CI run 42930497153** (headSha 5a52496e): ✅ green.
- **Patch 0007 (record cap, for step 3):** `YSERVER_RECORD_EVERY` (keep every Nth frame per window) and
  `YSERVER_RECORD_MAX` (total pixel files); frames.log still lists every frame (`file=-` when skipped).
  Defaults unchanged (every frame, no cap). A game at 60 fps would otherwise write ~200 MB/s.
- **Step 3 STARTED:** Bannerlator branch feat/yserver-spike 4903bdb7 (`BANNER_YSERVER=1` → BL runs
  files/yserver/yserver instead of the Java X server, headless + record every 30th frame, max 40).
- **CI run 42931291050** (headSha bc0a13f0): ✅ green; package sha256 `e6a2dea679478539c1074b25129c883051f05d3a2c9041bdcbbb4a77a0a72fcb` (CI log == download == /sdcard/Download/yserver-android-spike-bc0a13f0.tar.gz, host + device). 19:3x EST device run 4 starting.
- **Next:** device run 4 (expect ahbprobe 10/10) → stage yserver into BL's files/yserver
  (owner = BL uid, BL's SELinux label) → ask the user before installing the BL test build.

## 2026-11-10 — Android buffer (AHB) DRI3 path (feat/android-build, IN PROGRESS, nothing built yet)
- **Why:** user 2026-11-10 ~18:45 EST: "do the android buffer path next". Game frames from the guest
  Vulkan wrapper arrive as DRI3 1.2 PixmapFromBuffers with modifier 1255 and a socket fd carrying
  an AHardwareBuffer (leegao/bionic-vulkan-wrapper `src/vulkan/wsi/wsi_common_x11.c` ~1924-1966;
  the client blocks on a 1-byte read until the server writes one, so the server must always ack).
- **Design (patch 0005, being written in the scratch clone, not committed):**
  - yserver-render `kms/vk/ahb.rs` (Android): libnativewindow FFI (recvHandleFromUnixSocket,
    describe, release), ack byte first, import via VK_ANDROID_external_memory_android_hardware_buffer
    (dedicated alloc + ImportAndroidHardwareBufferInfo); `ImageBacking::ImportedAhb`;
    device.rs enables the AHB ext + queue_family_foreign on Android, `VkContext.ahb`.
  - yserver `kms/render/backend/android_ahb.rs`: generic `backend_import_ahb_pixmap` (store_alloc is
    backend-private); SurfacePlatform on Android: DRI3 caps 1.2, import 1255 → AHB path.
  - Test client `ahbprobe`: allocate AHB (format 5 BGRA + 1 RGBA), fill, send like the wrapper,
    PixmapFromBuffers, ack read with timeout, CopyArea + PresentPixmap, GetImage compare, timings.
- **Written + committed (no build yet):** `patches/0005-android-ahb-dri3.patch` (8 files, +439/-4;
  all 5 patches apply in order to the pin, checked in a clean worktree) and `client/src/bin/ahbprobe.rs`
  (compile-checked for aarch64-linux-android locally: clean). ahbprobe: DRI3 ≥ 1.2, AHB format 5
  (BGRA) and 1 (RGBA) 256x256 → PixmapFromBuffers 1255 + ack timing → CopyArea read-back →
  PresentPixmap + CompleteNotify + read-back → 300-present loop (avg/worst) → empty socket must be
  acked + refused → server still answers. run-test.sh runs it after xprobe.
- **CI run 42927479742** (headSha 368efd01): FAILED at Compile — yserver-render (all the AHB Vulkan
  code) compiled clean; yserver crate: `unresolved import crate::kms::vk::ahb` (the crate re-exports
  yserver-render's vk modules by name, `crates/yserver/src/kms/vk/mod.rs`). Fixed in patch 0005.
- **CI run 42927873800** (headSha 9ae0b21f): ✅ green (only the old `Sysv` warning). yserver now also
  NEEDS libnativewindow.so. Package sha256 `c766e18f1893be7aa0b99d2ab2d39d37f4d2bc06756b5300a81da11fe893ab64`
  (CI log == download == /sdcard/Download/yserver-android-spike-9ae0b21f.tar.gz on host and device).
- **19:04 EST: HOLD** — Hogwarts Legacy running in Bannerlator (peer root-de: user mid-game since 19:00, first launch); device run 3 waits for root-de's "done".
- **19:15 EST: hold lifted** (session root-66 took over on the user's "continue yserver"): no Wine process on the FIT, Discord in front; package hash re-checked on device (c766e18f… == CI). Running device run 3 now.
- **Resume point:** device run 3 = `sh /sdcard/Download/yserver-android-spike-step2.sh 9ae0b21f` in the
  Termux sandbox (same rootsh/su/runcon command as runs 1-2); results in ~/yserver-test/9ae0b21f/out-*.

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
