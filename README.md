# yserver for Android

[yserver](https://github.com/joske/yserver), the X11 server written in Rust by **Jos Dehaes** (MIT), built
for Android (aarch64) for [Bannerlator](https://github.com/The412Banner/Bannerlator): Bannerlator runs it
instead of its Java X server when a game sets `BANNER_YSERVER=1`, with its windows shown by Bannerlator's
Wayland compositor.

This repo holds no yserver code. The workflow downloads yserver at the commit in `YSERVER_COMMIT`, merges
**SetZero**'s rootless Wayland host ([joske/yserver#349](https://github.com/joske/yserver/pull/349)) at
`PR349_COMMIT`, and applies the Android changes in `patches/`.

- `patches/` — Android build, AHardwareBuffer hand-off to Bannerlator's compositor, frame pacing, newest-frame-only presents, hidden Wine desktop, HUD frame rate
- `deps/` — libxkbcommon as a static Android library, stub libraries, an xshmfence implementation
- `client/` — `xprobe`, `ahbprobe`, `vkprobe`: small test programs (x11rb)
- `scripts/run-test.sh` — runs yserver headless with its frame recorder, then `xprobe`
- `PROGRESS_LOG.md` — what was done, newest first
- Releases — the built `yserver` binary Bannerlator's CI bundles (`lib/arm64-v8a/libyserver.so`)

Licence: the patches and scripts here are MIT (`LICENSE`); yserver is MIT, Copyright (c) 2026 Jos Dehaes
(`LICENSE-yserver`), and the built binaries carry both.

Build: Actions → build → Run workflow (`check` or `build`).
