# yserver Android test (private)

A private test: can [yserver](https://github.com/joske/yserver), the X11 server written in Rust by
Jos Dehaes (MIT licence), build and run on Android, as a possible replacement for the Java X server
in Bannerlator?

This repo holds no yserver code. The workflow downloads yserver at the commit in `YSERVER_COMMIT`
and applies the small Android changes in `patches/`.

- `patches/` — Android build changes for yserver
- `deps/build-xkbcommon.sh` — libxkbcommon as a static Android library
- `client/` — `xprobe`, a small X11 test program (uses x11rb)
- `scripts/run-test.sh` — runs yserver headless with its frame recorder, then `xprobe`
- `PROGRESS_LOG.md` — what was done, newest first

Build: Actions → build → Run workflow (`check` or `build`).
