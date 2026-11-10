//! xprobe: fixed X11 checks against one server socket, for the yserver Android test.
//! Prints PASS / FAIL / INFO lines and a SUMMARY line; exit code 0 only when nothing failed.

use std::fs::{self, File};
use std::io::{IoSlice, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use x11rb::connection::{Connection, RequestConnection};
use x11rb::errors::ReplyError;
use x11rb::protocol::shm::ConnectionExt as _;
use x11rb::protocol::xkb::ConnectionExt as _;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::rust_connection::{DefaultStream, RustConnection};
use x11rb::wrapper::ConnectionExt as _;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

const W: u16 = 256;
const H: u16 = 256;
const RED: u32 = 0x00ff_0000;

struct Report {
    pass: u32,
    fail: u32,
}

impl Report {
    fn ok(&mut self, name: &str, detail: impl AsRef<str>) {
        self.pass += 1;
        println!("PASS {name}: {}", detail.as_ref());
    }
    fn bad(&mut self, name: &str, detail: impl AsRef<str>) {
        self.fail += 1;
        println!("FAIL {name}: {}", detail.as_ref());
    }
    fn info(&self, name: &str, detail: impl AsRef<str>) {
        println!("INFO {name}: {}", detail.as_ref());
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut socket: Option<PathBuf> = None;
    let mut record: Option<PathBuf> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--socket" => socket = args.next().map(PathBuf::from),
            "--record-dir" => record = args.next().map(PathBuf::from),
            _ => {
                eprintln!("usage: xprobe --socket PATH [--record-dir DIR]");
                std::process::exit(2);
            }
        }
    }
    let Some(socket) = socket else {
        eprintln!("--socket is required");
        std::process::exit(2);
    };
    let mut r = Report { pass: 0, fail: 0 };
    if let Err(e) = run(&socket, record.as_deref(), &mut r) {
        r.bad("run", format!("stopped: {e}"));
    }
    println!("SUMMARY pass={} fail={}", r.pass, r.fail);
    std::process::exit(if r.fail == 0 { 0 } else { 1 });
}

fn run(socket: &Path, record: Option<&Path>, r: &mut Report) -> Res<()> {
    let t0 = Instant::now();
    let unix = std::os::unix::net::UnixStream::connect(socket)?;
    let (stream, _peer) = DefaultStream::from_unix_stream(unix)?;
    let conn = RustConnection::connect_to_stream(stream, 0)?;
    let connect_ms = ms(t0.elapsed());
    let setup = conn.setup();
    let screen = setup.roots[0].clone();
    let depth = screen.root_depth;
    r.ok(
        "connect",
        format!(
            "{connect_ms:.1} ms; vendor {:?} release {}; screen {}x{} depth {}; max request {} bytes",
            String::from_utf8_lossy(&setup.vendor),
            setup.release_number,
            screen.width_in_pixels,
            screen.height_in_pixels,
            depth,
            conn.maximum_request_bytes()
        ),
    );
    if setup.image_byte_order != ImageOrder::LSB_FIRST {
        r.bad("byte-order", "server image byte order is not LSBFirst; pixel checks assume it");
    }

    let names: Vec<String> = conn
        .list_extensions()?
        .reply()?
        .names
        .iter()
        .map(|s| String::from_utf8_lossy(&s.name).into_owned())
        .collect();
    r.ok("extensions", format!("{} listed: {}", names.len(), names.join(" ")));
    let wanted = [
        "BIG-REQUESTS", "MIT-SHM", "XKEYBOARD", "RENDER", "XFIXES", "DRI3", "Present", "SYNC",
        "RANDR", "XInputExtension", "Composite", "SHAPE", "DAMAGE", "XTEST", "GLX",
    ];
    let missing: Vec<&str> = wanted.iter().copied().filter(|w| !names.iter().any(|n| n == w)).collect();
    r.info("wine-extensions", if missing.is_empty() { "all present".to_string() } else { format!("missing {}", missing.join(" ")) });

    // Window
    let win = conn.generate_id()?;
    conn.create_window(
        depth,
        win,
        screen.root,
        10,
        10,
        W,
        H,
        0,
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &CreateWindowAux::new()
            .background_pixel(screen.black_pixel)
            .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY),
    )?;
    conn.change_property8(PropMode::REPLACE, win, AtomEnum::WM_NAME, AtomEnum::STRING, b"xprobe")?;
    conn.map_window(win)?;
    conn.flush()?;
    let t = Instant::now();
    let deadline = t + Duration::from_secs(3);
    let (mut mapped, mut exposed) = (false, false);
    while Instant::now() < deadline && !(mapped && exposed) {
        match conn.poll_for_event()? {
            Some(Event::MapNotify(e)) if e.window == win => mapped = true,
            Some(Event::Expose(e)) if e.window == win => exposed = true,
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    }
    if mapped {
        r.ok("window", format!("mapped in {:.1} ms{}", ms(t.elapsed()), if exposed { ", exposed" } else { ", no Expose" }));
    } else {
        r.bad("window", "no MapNotify within 3 s");
    }

    // Core drawing
    let gc = conn.generate_id()?;
    conn.create_gc(gc, win, &CreateGCAux::new().foreground(RED))?;
    conn.poly_fill_rectangle(win, gc, &[Rectangle { x: 0, y: 0, width: W, height: H }])?;
    let one = conn.get_image(ImageFormat::Z_PIXMAP, win, 128, 200, 1, 1, !0)?.reply()?;
    let px = pixel(&one.data, 0);
    if px == RED {
        r.ok("fill", "PolyFillRectangle + GetImage pixel is red");
    } else {
        r.bad("fill", format!("pixel 0x{px:06x}, expected 0x{RED:06x}"));
    }
    let pat = pattern(64, 64, 0);
    conn.put_image(ImageFormat::Z_PIXMAP, win, gc, 64, 64, 16, 16, 0, depth, &pat)?;
    let back = conn.get_image(ImageFormat::Z_PIXMAP, win, 16, 16, 64, 64, !0)?.reply()?;
    match compare(&pat, &back.data) {
        0 => r.ok("putimage", "64x64 PutImage reads back exactly"),
        n => r.bad("putimage", format!("{n} of 4096 pixels differ")),
    }

    // MIT-SHM with an fd (AttachFd, MIT-SHM 1.2)
    if conn.extension_information(x11rb::protocol::shm::X11_EXTENSION_NAME)?.is_some() {
        let v = conn.shm_query_version()?.reply()?;
        let pat2 = pattern(64, 64, 1);
        let mut f = File::from(memfd()?);
        f.set_len(pat2.len() as u64)?;
        f.write_all(&pat2)?;
        let seg = conn.generate_id()?;
        let dup: OwnedFd = f.try_clone()?.into();
        conn.shm_attach_fd(seg, dup, true)?;
        conn.shm_put_image(win, gc, 64, 64, 0, 0, 64, 64, 96, 16, depth, ImageFormat::Z_PIXMAP.into(), false, seg, 0)?;
        let back = conn.get_image(ImageFormat::Z_PIXMAP, win, 96, 16, 64, 64, !0)?.reply()?;
        match compare(&pat2, &back.data) {
            0 => r.ok("mit-shm", format!("version {}.{}, AttachFd + ShmPutImage reads back exactly", v.major_version, v.minor_version)),
            n => r.bad("mit-shm", format!("{n} of 4096 pixels differ")),
        }
        conn.shm_detach(seg)?;
    } else {
        r.bad("mit-shm", "MIT-SHM not offered");
    }

    // Keyboard
    if conn.extension_information(x11rb::protocol::xkb::X11_EXTENSION_NAME)?.is_some() {
        let rep = conn.xkb_use_extension(1, 0)?.reply()?;
        if rep.supported {
            r.ok("xkb", format!("UseExtension 1.0 supported (server {}.{})", rep.server_major, rep.server_minor));
        } else {
            r.bad("xkb", "UseExtension 1.0 not supported");
        }
    } else {
        r.bad("xkb", "XKEYBOARD not offered");
    }
    let (min, max) = (setup.min_keycode, setup.max_keycode);
    let km = conn.get_keyboard_mapping(min, max - min + 1)?.reply()?;
    let set = km.keysyms.iter().filter(|&&k| k != 0).count();
    if set > 0 {
        r.ok("keymap", format!("keycodes {min}..{max}, {} keysyms per key, {set} set", km.keysyms_per_keycode));
    } else {
        r.bad("keymap", "no keysyms");
    }

    // Robustness: an unknown core request must get an error and leave the stream in step.
    let req = [120u8, 0, 1, 0];
    let cookie = conn.send_request_without_reply(&[IoSlice::new(&req)], Vec::new())?;
    match cookie.check() {
        Err(ReplyError::X11Error(e)) => r.ok("unknown-request", format!("answered with {:?}", e.error_kind)),
        Ok(()) => r.bad("unknown-request", "no error returned"),
        Err(e) => r.bad("unknown-request", format!("{e}")),
    }
    conn.get_input_focus()?.reply()?;
    r.ok("after-unknown", "server still answers");
    let fid = conn.generate_id()?;
    match conn.open_font(fid, b"fixed")?.check() {
        Ok(()) => r.ok("open-font", "\"fixed\" opened"),
        Err(ReplyError::X11Error(e)) => r.ok("open-font", format!("\"fixed\" refused with {:?}", e.error_kind)),
        Err(e) => r.bad("open-font", format!("{e}")),
    }
    conn.get_input_focus()?.reply()?;
    r.ok("after-font", "server still answers");

    // Timing
    let n = 10_000;
    let t = Instant::now();
    for _ in 0..n {
        conn.get_input_focus()?.reply()?;
    }
    let el = t.elapsed();
    r.ok("roundtrip", format!("{n} GetInputFocus in {:.1} ms = {:.1} us each", ms(el), el.as_secs_f64() * 1e6 / n as f64));
    let big = vec![0x55u8; W as usize * H as usize * 4];
    let rounds = 200;
    let t = Instant::now();
    for _ in 0..rounds {
        conn.put_image(ImageFormat::Z_PIXMAP, win, gc, W, H, 0, 0, 0, depth, &big)?;
    }
    conn.get_input_focus()?.reply()?;
    let el = t.elapsed();
    let mb = (big.len() * rounds) as f64 / (1024.0 * 1024.0);
    r.ok("putimage-speed", format!("{rounds} x 256x256 PutImage = {mb:.0} MB in {:.1} ms = {:.0} MB/s", ms(el), mb / el.as_secs_f64()));

    // Final picture for the recorder: red window with both patterns.
    conn.poly_fill_rectangle(win, gc, &[Rectangle { x: 0, y: 0, width: W, height: H }])?;
    conn.put_image(ImageFormat::Z_PIXMAP, win, gc, 64, 64, 16, 16, 0, depth, &pat)?;
    conn.get_input_focus()?.reply()?;
    if let Some(dir) = record {
        std::thread::sleep(Duration::from_millis(800));
        match check_record(dir, &pat) {
            Ok(msg) => r.ok("record", msg),
            Err(msg) => r.bad("record", msg),
        }
    }

    conn.free_gc(gc)?;
    conn.destroy_window(win)?;
    conn.get_input_focus()?.reply()?;
    r.info("total", format!("{:.0} ms", ms(t0.elapsed())));
    Ok(())
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// 0x00RRGGBB of pixel `i` in a 32 bpp LSBFirst Z-pixmap.
fn pixel(data: &[u8], i: usize) -> u32 {
    let p = &data[i * 4..i * 4 + 4];
    u32::from_le_bytes([p[0], p[1], p[2], p[3]]) & 0x00ff_ffff
}

/// Known BGRX test image; `kind` picks one of two layouts.
fn pattern(w: usize, h: usize, kind: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let (b, g, rr) = if kind == 0 { ((x * 4) as u8, (y * 4) as u8, 0x80) } else { (0x40, (x * 4) as u8, (y * 4) as u8) };
            v.extend_from_slice(&[b, g, rr, 0]);
        }
    }
    v
}

/// Pixels that differ, ignoring the pad byte.
fn compare(want: &[u8], got: &[u8]) -> usize {
    if got.len() < want.len() {
        return want.len() / 4;
    }
    (0..want.len() / 4).filter(|&i| pixel(want, i) != pixel(got, i)).count()
}

fn memfd() -> std::io::Result<OwnedFd> {
    let name = b"xprobe\0";
    #[cfg(target_os = "android")]
    let fd = unsafe { libc::syscall(libc::SYS_memfd_create, name.as_ptr(), libc::MFD_CLOEXEC) } as libc::c_int;
    #[cfg(not(target_os = "android"))]
    let fd = unsafe { libc::memfd_create(name.as_ptr().cast(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// Newest recorded W x H frame: red at (128,200) and the pattern at (16,16).
fn check_record(dir: &Path, pat: &[u8]) -> Result<String, String> {
    let size = W as u64 * H as u64 * 4;
    let mut newest: Option<(SystemTime, PathBuf)> = None;
    let mut frames = 0;
    for e in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let e = e.map_err(|e| e.to_string())?;
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("bgra") {
            continue;
        }
        frames += 1;
        let m = e.metadata().map_err(|e| e.to_string())?;
        if m.len() != size {
            continue;
        }
        let t = m.modified().map_err(|e| e.to_string())?;
        if newest.as_ref().is_none_or(|(nt, _)| t >= *nt) {
            newest = Some((t, p));
        }
    }
    let Some((_, path)) = newest else {
        return Err(format!("{frames} .bgra frames, none {W}x{H}"));
    };
    let data = fs::read(&path).map_err(|e| e.to_string())?;
    let row = W as usize;
    let red = pixel(&data, 200 * row + 128);
    let mut bad = 0;
    for y in 0..64 {
        for x in 0..64 {
            if pixel(&data, (16 + y) * row + 16 + x) != pixel(pat, y * 64 + x) {
                bad += 1;
            }
        }
    }
    if red == RED && bad == 0 {
        Ok(format!("{frames} frames recorded; {} matches", path.file_name().unwrap_or_default().to_string_lossy()))
    } else {
        Err(format!("{}: red 0x{red:06x}, {bad} pattern pixels differ", path.display()))
    }
}
