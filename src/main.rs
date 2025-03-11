#![no_std]
#![cfg_attr(not(test), no_main)]
#![no_builtins]
#![feature(concat_bytes, const_trait_impl, naked_functions)]

use core::{
    alloc::GlobalAlloc, arch::naked_asm, mem::MaybeUninit, panic::PanicInfo, ptr::null_mut,
};

use draw::draw_time;
use io::{ArrayWriter, BufWriter, FdWriter, Write as _};
use io_uring::IoUring;

pub mod draw;
pub mod fmt;
pub mod io;
pub mod io_uring;
pub mod parse;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        core::fmt::Write::write_fmt(&mut crate::io::FdWriter::stdout(), format_args!($($arg)*)).unwrap()
    }
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => {
        core::fmt::Write::write_fmt(&mut crate::io::FdWriter::stderr(), format_args!($($arg)*)).unwrap()
    }
}

#[macro_export]
macro_rules! set_buffer {
    () => {
        b"[?1049h"
    };
}

#[macro_export]
macro_rules! restore_buffer {
    () => {
        b"[?1049l"
    };
}

#[macro_export]
macro_rules! hide_cursor {
    () => {
        b"[?25l"
    };
}

#[macro_export]
macro_rules! show_cursor {
    () => {
        b"[?25h"
    };
}

#[macro_export]
macro_rules! cursor_position {
    () => {
        b"[H"
    };
}

#[macro_export]
macro_rules! buffer_size {
    () => {
        b"[18t"
    };
}

#[macro_export]
macro_rules! fg_color {
    (black) => {
        b"[30m"
    };
    (red) => {
        b"[31m"
    };
    (green) => {
        b"[32m"
    };
    (yellow) => {
        b"[33m"
    };
    (blue) => {
        b"[34m"
    };
    (magenta) => {
        b"[35m"
    };
    (cyan) => {
        b"[36m"
    };
    (white) => {
        b"[37m"
    };

    (br_black) => {
        b"[90m"
    };
    (br_red) => {
        b"[91m"
    };
    (br_green) => {
        b"[92m"
    };
    (br_yellow) => {
        b"[93m"
    };
    (br_blue) => {
        b"[94m"
    };
    (br_magenta) => {
        b"[95m"
    };
    (br_cyan) => {
        b"[96m"
    };
    (br_white) => {
        b"[97m"
    };
}

#[inline(always)]
fn on_exit() -> io::Result<()> {
    FdWriter::stdout().write_all(concat_bytes!(restore_buffer!(), show_cursor!()))?;

    #[allow(static_mut_refs)]
    unsafe {
        nc::ioctl(io::STDIN, nc::TCSETS, TERMIOS.as_ptr() as _)?;
    }

    Ok(())
}

#[naked]
extern "C" fn restorer() {
    unsafe { naked_asm!("mov rax, 0xf", "syscall",) }
}

type MarginBuf = [u8; 32];

fn resize() {
    let winsz = MaybeUninit::<nc::winsize_t>::uninit();
    #[allow(static_mut_refs)]
    unsafe {
        nc::ioctl(io::STDIN, nc::TIOCGWINSZ, winsz.as_ptr() as _)
            .unwrap_or_else(|e| utils::exit(e as _));
        let nc::winsize_t { ws_row, ws_col, .. } = winsz.assume_init();
        let mut margin_left = MaybeUninit::<MarginBuf>::uninit();
        let mut writer = ArrayWriter::new(margin_left.assume_init_mut());
        _ = cursor_move(&mut writer, ((ws_col - 38) / 2) as _, Direction::Right);
        let len = writer.len;
        MARGIN_LEFT.write((margin_left.assume_init(), len as _));

        let mut margin_top = MaybeUninit::<MarginBuf>::uninit();
        let mut writer = ArrayWriter::new(margin_top.assume_init_mut());
        _ = cursor_move(&mut writer, ((ws_row - 5) / 2) as _, Direction::Down);
        let len = writer.len;
        MARGIN_TOP.write((margin_top.assume_init(), len as _));
    };
}

fn set_signal_handler() {
    extern "C" fn terminate(_: i32) {
        _ = on_exit();
        utils::exit(0);
    }

    fn resize(_: i32) {
        crate::resize();
    }

    unsafe {
        let sa = nc::sigaction_t {
            sa_handler: terminate as _,
            sa_flags: nc::SA_RESTORER,
            sa_restorer: None,
            ..Default::default()
        };
        _ = nc::rt_sigaction(nc::SIGINT, Some(&sa), None);
        _ = nc::rt_sigaction(nc::SIGTERM, Some(&sa), None);

        let sa = nc::sigaction_t {
            sa_handler: resize as _,
            sa_flags: nc::SA_RESTORER | nc::SA_RESTART,
            sa_restorer: Some(restorer),
            ..Default::default()
        };
        _ = nc::rt_sigaction(nc::SIGWINCH, Some(&sa), None);
    }
}

static mut TERMIOS: MaybeUninit<nc::termios_t> = MaybeUninit::uninit();
static mut MARGIN_LEFT: MaybeUninit<(MarginBuf, u8)> = MaybeUninit::uninit();
static mut MARGIN_TOP: MaybeUninit<(MarginBuf, u8)> = MaybeUninit::uninit();

fn margin_left() -> &'static [u8] {
    #[allow(static_mut_refs)]
    let (buf, len) = unsafe { MARGIN_LEFT.assume_init_ref() };
    unsafe { buf.get_unchecked(..*len as _) }
}

fn margin_top() -> &'static [u8] {
    #[allow(static_mut_refs)]
    let (buf, len) = unsafe { MARGIN_TOP.assume_init_ref() };
    unsafe { buf.get_unchecked(..*len as _) }
}

#[repr(u8)]
#[allow(unused)]
enum Direction {
    Up = b'A',
    Down = b'B',
    Right = b'C',
    Left = b'D',
}

fn cursor_move(writer: &mut impl io::Write, n: u64, direction: Direction) -> io::Result<()> {
    writer.write_all(b"[")?;
    writer.write_u64(n)?;
    writer.write_all(&[direction as _][..])?;
    Ok(())
}

fn main() -> io::Result<()> {
    let mut buf = MaybeUninit::<[u8; 1024]>::uninit();
    let buf = unsafe { buf.assume_init_mut() };
    let mut ctx = draw::Context::new(BufWriter::new(FdWriter::stdout(), buf));

    let mut redraw = || -> io::Result<()> {
        ctx.writer.write_all(concat_bytes!(
            restore_buffer!(),
            set_buffer!(),
            cursor_position!(),
            fg_color!(br_blue)
        ))?;
        ctx.writer.write(margin_top())?;
        let mut time = MaybeUninit::uninit();
        unsafe {
            nc::time(time.assume_init_mut())?;
            let content = draw_time(time.assume_init() + 8 * 3600);
            ctx.draw(Some(margin_left()), || content)?
        };
        ctx.writer.flush()?;
        Ok(())
    };

    #[allow(static_mut_refs)]
    unsafe {
        nc::ioctl(io::STDIN, nc::TCGETS, TERMIOS.as_ptr() as _)?;
        let mut termios = TERMIOS.assume_init_ref().clone();
        termios.c_lflag &= !(nc::ECHO | nc::ICANON);
        nc::ioctl(io::STDIN, nc::TCSETS, &raw const termios as _)?;
    }
    resize();
    redraw()?;
    set_signal_handler();
    FdWriter::stdout().write_all(hide_cursor!())?;

    #[repr(usize)]
    enum Token {
        Timeout = 1,
        Read,
    }
    let ring = IoUring::new(2)?;

    let mut input_buf = MaybeUninit::<[u8; 32]>::uninit();
    let mut sigset = nc::sigset_t::default();
    sigset.sig[0] |= 1 << (nc::SIGWINCH) - 1;
    ring.prepare_read(
        io::STDIN as _,
        unsafe { input_buf.assume_init_mut() },
        Token::Read as _,
    );
    let duration = nc::timespec_t {
        tv_sec: 1,
        tv_nsec: 0,
    };
    ring.prepare_timeout(&duration, Token::Timeout as _, 1 << 6); // multishot

    ring.submit(2)?;

    fn wait(ring: &IoUring, cb: &mut impl FnMut() -> io::Result<()>) -> io::Result<()> {
        loop {
            match ring.wait() {
                Ok(_) => break Ok(()),
                Err(x) if x == nc::EINTR => cb()?,
                Err(x) => break Err(x),
            }
        }
    }

    loop {
        wait(&ring, &mut redraw)?;
        let cqe = ring.complete();
        match cqe.user_data {
            x if x == Token::Timeout as _ => {
                redraw()?;
            }
            x if x == Token::Read as _ => {
                if cqe.res == 1 && [b'', b'q'].contains(&unsafe { input_buf.assume_init_ref() }[0])
                {
                    break;
                }
                ring.prepare_read(
                    io::STDIN as _,
                    unsafe { input_buf.assume_init_mut() },
                    Token::Read as _,
                );
            }
            _ => utils::unreachable(),
        }
        ring.submit(1)?;
    }
    on_exit()
}

#[cfg_attr(not(test), unsafe(no_mangle))]
extern "C" fn _start() -> ! {
    utils::exit(match main() {
        Ok(_) => 0,
        Err(e) => e as _,
    });
}

pub mod utils {
    use core::arch::asm;

    pub fn exit(state: usize) -> ! {
        _ = unsafe { nc::syscalls::syscall1(nc::SYS_EXIT, state) };
        unreachable()
    }

    pub fn unreachable() -> ! {
        unsafe { asm!("", options(noreturn)) }
    }

    #[inline(never)]
    pub const fn copy_nonoverlapping(mut src: *const u8, mut dst: *mut u8, mut count: usize) {
        while count > 0 {
            unsafe {
                *dst = *src;
                src = src.add(1);
                dst = dst.add(1);
            }
            count -= 1;
        }
    }
}

#[cfg_attr(not(test), panic_handler)]
pub fn panic(info: &PanicInfo) -> ! {
    _ = on_exit();
    if let Some(x) = info.location() {
        eprint!("{}: ", x);
    }
    eprint!("{}\n", info.message());
    utils::exit(1)
}

#[cfg_attr(not(test), global_allocator)]
pub static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

pub struct GlobalAllocator;

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}
