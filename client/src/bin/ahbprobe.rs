//! ahbprobe: hands AHardwareBuffers to an X server the way the guest Vulkan wrapper does
//! (DRI3 PixmapFromBuffers, modifier 1255, handle sent down a socketpair, 1-byte ack),
//! then checks the pixels after CopyArea and PresentPixmap and times the presents.
//! Prints PASS / FAIL / INFO lines and a SUMMARY line; exit 0 only when nothing failed.

#[cfg(not(target_os = "android"))]
fn main() {
    eprintln!("ahbprobe runs on Android only");
    std::process::exit(2);
}

#[cfg(target_os = "android")]
fn main() {
    android::main();
}

#[cfg(target_os = "android")]
mod android {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use x11rb::connection::{Connection, RequestConnection};
    use x11rb::errors::ReplyError;
    use x11rb::protocol::dri3::ConnectionExt as _;
    use x11rb::protocol::present::{self, ConnectionExt as _};
    use x11rb::protocol::xproto::*;
    use x11rb::protocol::Event;
    use x11rb::rust_connection::{DefaultStream, RustConnection};

    type Res<T> = Result<T, Box<dyn std::error::Error>>;

    const W: u16 = 256;
    const H: u16 = 256;
    const MODIFIER_AHB_SOCKET: u64 = 1255;
    const FORMAT_RGBA8888: u32 = 1;
    const FORMAT_BGRA8888: u32 = 5;
    const USAGE_CPU_READ_RARELY: u64 = 2;
    const USAGE_CPU_WRITE_OFTEN: u64 = 3 << 4;
    const USAGE_GPU_SAMPLED_IMAGE: u64 = 1 << 8;
    const USAGE_GPU_FRAMEBUFFER: u64 = 1 << 9;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct AhbDesc {
        width: u32,
        height: u32,
        layers: u32,
        format: u32,
        usage: u64,
        stride: u32,
        rfu0: u32,
        rfu1: u64,
    }

    #[link(name = "nativewindow")]
    extern "C" {
        fn AHardwareBuffer_allocate(desc: *const AhbDesc, out: *mut *mut libc::c_void) -> libc::c_int;
        fn AHardwareBuffer_release(buffer: *mut libc::c_void);
        fn AHardwareBuffer_describe(buffer: *const libc::c_void, out: *mut AhbDesc);
        fn AHardwareBuffer_lock(
            buffer: *mut libc::c_void,
            usage: u64,
            fence: i32,
            rect: *const libc::c_void,
            out: *mut *mut libc::c_void,
        ) -> libc::c_int;
        fn AHardwareBuffer_unlock(buffer: *mut libc::c_void, fence: *mut i32) -> libc::c_int;
        fn AHardwareBuffer_sendHandleToUnixSocket(buffer: *const libc::c_void, socket: libc::c_int) -> libc::c_int;
    }

    struct Ahb(*mut libc::c_void);

    impl Drop for Ahb {
        fn drop(&mut self) {
            unsafe { AHardwareBuffer_release(self.0) };
        }
    }

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

    pub fn main() {
        let mut args = std::env::args().skip(1);
        let mut socket: Option<PathBuf> = None;
        while let Some(a) = args.next() {
            match a.as_str() {
                "--socket" => socket = args.next().map(PathBuf::from),
                _ => {
                    eprintln!("usage: ahbprobe --socket PATH");
                    std::process::exit(2);
                }
            }
        }
        let Some(socket) = socket else {
            eprintln!("--socket is required");
            std::process::exit(2);
        };
        let mut r = Report { pass: 0, fail: 0 };
        if let Err(e) = run(&socket, &mut r) {
            r.bad("run", format!("stopped: {e}"));
        }
        println!("SUMMARY pass={} fail={}", r.pass, r.fail);
        std::process::exit(if r.fail == 0 { 0 } else { 1 });
    }

    fn ms(d: Duration) -> f64 {
        d.as_secs_f64() * 1000.0
    }

    /// Expected X pixel (0x00RRGGBB) at (x, y) of the test image.
    fn expected(x: usize, y: usize, flip: bool) -> u32 {
        let (r, g, b) = if flip { (0x40u32, (x * 4) as u32 & 0xff, (y * 4) as u32 & 0xff) } else { (0x80u32, (y * 4) as u32 & 0xff, (x * 4) as u32 & 0xff) };
        (r << 16) | (g << 8) | b
    }

    fn allocate(format: u32, flip: bool) -> Res<(Ahb, AhbDesc)> {
        let want = AhbDesc {
            width: u32::from(W),
            height: u32::from(H),
            layers: 1,
            format,
            usage: USAGE_GPU_SAMPLED_IMAGE | USAGE_GPU_FRAMEBUFFER | USAGE_CPU_WRITE_OFTEN | USAGE_CPU_READ_RARELY,
            ..Default::default()
        };
        let mut out = std::ptr::null_mut();
        let rc = unsafe { AHardwareBuffer_allocate(&want, &mut out) };
        if rc != 0 || out.is_null() {
            return Err(format!("AHardwareBuffer_allocate(format {format}) = {rc}").into());
        }
        let buf = Ahb(out);
        let mut desc = AhbDesc::default();
        unsafe { AHardwareBuffer_describe(buf.0, &mut desc) };
        let mut addr = std::ptr::null_mut();
        let rc = unsafe { AHardwareBuffer_lock(buf.0, USAGE_CPU_WRITE_OFTEN, -1, std::ptr::null(), &mut addr) };
        if rc != 0 || addr.is_null() {
            return Err(format!("AHardwareBuffer_lock = {rc}").into());
        }
        let stride = desc.stride as usize;
        let px = addr.cast::<u8>();
        for y in 0..H as usize {
            for x in 0..W as usize {
                let c = expected(x, y, flip);
                let (r, g, b) = ((c >> 16) as u8, (c >> 8) as u8, c as u8);
                let bytes = if format == FORMAT_BGRA8888 { [b, g, r, 0xff] } else { [r, g, b, 0xff] };
                unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), px.add((y * stride + x) * 4), 4) };
            }
        }
        let mut fence = -1;
        unsafe { AHardwareBuffer_unlock(buf.0, &mut fence) };
        if fence >= 0 {
            unsafe { libc::close(fence) };
        }
        Ok((buf, desc))
    }

    /// Send the buffer like the wrapper does; returns (pixmap, ms until the server's ack byte).
    fn send(conn: &RustConnection, win: Window, buf: &Ahb, desc: &AhbDesc) -> Res<(Pixmap, f64)> {
        let mut sv = [0 as libc::c_int; 2];
        if unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, sv.as_mut_ptr()) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let ours = unsafe { OwnedFd::from_raw_fd(sv[0]) };
        let theirs = unsafe { OwnedFd::from_raw_fd(sv[1]) };
        let rc = unsafe { AHardwareBuffer_sendHandleToUnixSocket(buf.0, ours.as_raw_fd()) };
        if rc != 0 {
            return Err(format!("AHardwareBuffer_sendHandleToUnixSocket = {rc}").into());
        }
        let pixmap = conn.generate_id()?;
        let t = Instant::now();
        let cookie = conn.dri3_pixmap_from_buffers(
            pixmap,
            win,
            W,
            H,
            desc.stride * 4,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            24,
            32,
            MODIFIER_AHB_SOCKET,
            vec![theirs],
        )?;
        conn.flush()?;
        let mut pfd = libc::pollfd { fd: ours.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let ready = unsafe { libc::poll(&mut pfd, 1, 3000) };
        if ready != 1 {
            return Err("no ack byte from the server within 3 s (a real client would hang here)".into());
        }
        let mut byte = 0u8;
        unsafe { libc::read(ours.as_raw_fd(), (&mut byte as *mut u8).cast(), 1) };
        let ack_ms = ms(t.elapsed());
        cookie.check()?;
        Ok((pixmap, ack_ms))
    }

    fn compare(data: &[u8], flip: bool) -> usize {
        let mut bad = 0;
        for y in 0..H as usize {
            for x in 0..W as usize {
                let i = (y * W as usize + x) * 4;
                let got = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) & 0x00ff_ffff;
                if got != expected(x, y, flip) {
                    bad += 1;
                }
            }
        }
        bad
    }

    fn first_diff(data: &[u8], flip: bool) -> String {
        for y in 0..H as usize {
            for x in 0..W as usize {
                let i = (y * W as usize + x) * 4;
                let got = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) & 0x00ff_ffff;
                let want = expected(x, y, flip);
                if got != want {
                    return format!("first at ({x},{y}): got 0x{got:06x}, want 0x{want:06x}");
                }
            }
        }
        String::new()
    }

    fn wait_complete(conn: &RustConnection, serial: u32, timeout: Duration) -> Res<Option<Duration>> {
        let t = Instant::now();
        while t.elapsed() < timeout {
            match conn.poll_for_event()? {
                Some(Event::PresentCompleteNotify(e)) if e.serial == serial => return Ok(Some(t.elapsed())),
                Some(_) => {}
                None => std::thread::sleep(Duration::from_micros(100)),
            }
        }
        Ok(None)
    }

    fn run(socket: &std::path::Path, r: &mut Report) -> Res<()> {
        let unix = std::os::unix::net::UnixStream::connect(socket)?;
        let (stream, _peer) = DefaultStream::from_unix_stream(unix)?;
        let conn = RustConnection::connect_to_stream(stream, 0)?;
        let screen = conn.setup().roots[0].clone();

        if conn.extension_information(x11rb::protocol::dri3::X11_EXTENSION_NAME)?.is_none() {
            r.bad("dri3", "DRI3 not offered");
            return Ok(());
        }
        let v = conn.dri3_query_version(1, 2)?.reply()?;
        if (v.major_version, v.minor_version) >= (1, 2) {
            r.ok("dri3", format!("version {}.{}", v.major_version, v.minor_version));
        } else {
            r.bad("dri3", format!("version {}.{}; PixmapFromBuffers needs 1.2", v.major_version, v.minor_version));
            return Ok(());
        }
        let pv = conn.present_query_version(1, 2)?.reply()?;
        r.info("present", format!("version {}.{}", pv.major_version, pv.minor_version));

        let win = conn.generate_id()?;
        conn.create_window(
            screen.root_depth,
            win,
            screen.root,
            300,
            10,
            W,
            H,
            0,
            WindowClass::INPUT_OUTPUT,
            screen.root_visual,
            &CreateWindowAux::new().background_pixel(screen.black_pixel),
        )?;
        conn.map_window(win)?;
        let gc = conn.generate_id()?;
        conn.create_gc(gc, win, &CreateGCAux::new().foreground(0x00ff_0000))?;
        let eid = conn.generate_id()?;
        conn.present_select_input(eid, win, present::EventMask::COMPLETE_NOTIFY | present::EventMask::IDLE_NOTIFY)?;
        conn.get_input_focus()?.reply()?;

        let mut pixmaps = Vec::new();
        let mut buffers = Vec::new();
        for (name, format, flip) in [("bgra", FORMAT_BGRA8888, false), ("rgba", FORMAT_RGBA8888, true)] {
            let (buf, desc) = match allocate(format, flip) {
                Ok(v) => v,
                Err(e) => {
                    r.bad(&format!("{name}-allocate"), e.to_string());
                    continue;
                }
            };
            r.info(&format!("{name}-buffer"), format!("{}x{} stride {} format {} usage 0x{:x}", desc.width, desc.height, desc.stride, desc.format, desc.usage));
            let (pixmap, ack_ms) = match send(&conn, win, &buf, &desc) {
                Ok(v) => v,
                Err(e) => {
                    r.bad(&format!("{name}-import"), e.to_string());
                    continue;
                }
            };
            r.ok(&format!("{name}-import"), format!("PixmapFromBuffers accepted, ack after {ack_ms:.2} ms"));

            conn.poly_fill_rectangle(win, gc, &[Rectangle { x: 0, y: 0, width: W, height: H }])?;
            conn.copy_area(pixmap, win, gc, 0, 0, 0, 0, W, H)?;
            let img = conn.get_image(ImageFormat::Z_PIXMAP, win, 0, 0, W, H, !0)?.reply()?;
            match compare(&img.data, flip) {
                0 => r.ok(&format!("{name}-copyarea"), "CopyArea from the buffer reads back exactly"),
                n => r.bad(&format!("{name}-copyarea"), format!("{n} of 65536 pixels differ; {}", first_diff(&img.data, flip))),
            }

            conn.poly_fill_rectangle(win, gc, &[Rectangle { x: 0, y: 0, width: W, height: H }])?;
            let serial = 1000 + format;
            conn.present_pixmap(win, pixmap, serial, 0, 0, 0, 0, 0, 0, 0, present::Option::NONE.into(), 0, 0, 0, &[])?;
            conn.flush()?;
            match wait_complete(&conn, serial, Duration::from_secs(3))? {
                Some(d) => {
                    let img = conn.get_image(ImageFormat::Z_PIXMAP, win, 0, 0, W, H, !0)?.reply()?;
                    match compare(&img.data, flip) {
                        0 => r.ok(&format!("{name}-present"), format!("PresentPixmap complete in {:.2} ms, window reads back exactly", ms(d))),
                        n => r.bad(&format!("{name}-present"), format!("complete in {:.2} ms but {n} pixels differ; {}", ms(d), first_diff(&img.data, flip))),
                    }
                }
                None => r.bad(&format!("{name}-present"), "no PresentCompleteNotify within 3 s"),
            }
            pixmaps.push(pixmap);
            buffers.push(buf);
        }

        // Swapchain-like loop: alternate the imported buffers, wait for each completion.
        if !pixmaps.is_empty() {
            let n = 300u32;
            let mut worst = Duration::ZERO;
            let mut missed = 0;
            let t = Instant::now();
            for i in 0..n {
                let serial = 5000 + i;
                conn.present_pixmap(win, pixmaps[i as usize % pixmaps.len()], serial, 0, 0, 0, 0, 0, 0, 0, present::Option::NONE.into(), 0, 0, 0, &[])?;
                conn.flush()?;
                match wait_complete(&conn, serial, Duration::from_secs(1))? {
                    Some(d) => worst = worst.max(d),
                    None => missed += 1,
                }
            }
            let el = t.elapsed();
            if missed == 0 {
                r.ok("present-loop", format!("{n} presents in {:.0} ms = {:.0}/s, {:.2} ms average, worst {:.2} ms", ms(el), f64::from(n) / el.as_secs_f64(), ms(el) / f64::from(n), ms(worst)));
            } else {
                r.bad("present-loop", format!("{missed} of {n} presents never completed"));
            }
        }

        // A buffer the server cannot read must fail cleanly and still be acked.
        let pixmap = conn.generate_id()?;
        let mut sv = [0 as libc::c_int; 2];
        unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, sv.as_mut_ptr()) };
        let ours = unsafe { OwnedFd::from_raw_fd(sv[0]) };
        let theirs = unsafe { OwnedFd::from_raw_fd(sv[1]) };
        let cookie = conn.dri3_pixmap_from_buffers(pixmap, win, W, H, u32::from(W) * 4, 0, 0, 0, 0, 0, 0, 0, 24, 32, MODIFIER_AHB_SOCKET, vec![theirs])?;
        conn.flush()?;
        let mut pfd = libc::pollfd { fd: ours.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let acked = unsafe { libc::poll(&mut pfd, 1, 3000) } == 1;
        match (acked, cookie.check()) {
            (true, Err(ReplyError::X11Error(e))) => r.ok("empty-socket", format!("acked, refused with {:?}", e.error_kind)),
            (true, Ok(())) => r.bad("empty-socket", "acked but accepted a socket with no buffer"),
            (false, _) => r.bad("empty-socket", "no ack byte: a real client would hang"),
            (true, Err(e)) => r.bad("empty-socket", format!("{e}")),
        }
        conn.get_input_focus()?.reply()?;
        r.ok("after-empty", "server still answers");

        for p in pixmaps {
            conn.free_pixmap(p)?;
        }
        conn.free_gc(gc)?;
        conn.destroy_window(win)?;
        conn.get_input_focus()?.reply()?;
        drop(buffers);
        Ok(())
    }
}
