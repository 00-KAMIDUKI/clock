#![no_std]
#![cfg_attr(not(test), no_main)]
#![no_builtins]
#![feature(concat_bytes, const_trait_impl, naked_functions)]

use core::{
    alloc::GlobalAlloc, arch::naked_asm, mem::MaybeUninit, panic::PanicInfo, ptr::null_mut,
};

use draw::{
    Cache,
    color::{Color, Literal},
    draw_time,
};
use io::{BufWriter, FdWriter, Write as _};

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
macro_rules! buffer_size {
    () => {
        b"[18t"
    };
}

#[inline(always)]
fn on_exit() -> io::Result<()> {
    FdWriter::stdout().write_all(concat_bytes!(restore_buffer!(), show_cursor!()))?;

    #[allow(static_mut_refs)]
    unsafe {
        nc::ioctl(0, nc::TCSETS, TERMIOS.as_ptr() as _)?;
    }

    Ok(())
}

#[naked]
extern "C" fn restorer() {
    unsafe { naked_asm!("mov rax, 0xf", "syscall",) }
}

fn set_signal_handler() {
    extern "C" fn terminate(_: i32) {
        _ = on_exit();
        utils::exit(0);
    }

    fn resize(_: i32) {}

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
            sa_restorer: Some(restorer),
            ..sa
        };
        _ = nc::rt_sigaction(nc::SIGWINCH, Some(&sa), None);
    }
}

static mut TERMIOS: MaybeUninit<nc::termios_t> = MaybeUninit::uninit();
static mut MARGIN_LEFT: MaybeUninit<([u8; 32], u8)> = MaybeUninit::uninit();
static mut MARGIN_TOP: MaybeUninit<([u8; 32], u8)> = MaybeUninit::uninit();

//fn parse_buffer_size(input: &[u8]) -> Option<[u64; 2]> {
//    let input = input.get(4..)?;
//    let ParseResult { value: rows, eaten } = <u64 as Parse>::parse(input);
//    let input = input.get(eaten + 1..)?;
//    let cols = <u64 as Parse>::parse(input).value;
//    Some([rows, cols])
//}

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
    const CACHE: Cache = Cache::new(Color::Bright(Literal::Blue), Color::Clear);
    let mut ctx = draw::Context::new(BufWriter::new(FdWriter::stdout(), buf), CACHE);

    #[allow(static_mut_refs)]
    unsafe {
        nc::ioctl(0, nc::TCGETS, TERMIOS.as_ptr() as _)?;
        let mut termios = TERMIOS.assume_init_ref().clone();
        termios.c_lflag &= !(nc::ECHO | nc::ICANON);
        nc::ioctl(0, nc::TCSETS, &raw const termios as _)?;
    }

    set_signal_handler();
    FdWriter::stdout().write_all(hide_cursor!())?;

    loop {
        ctx.writer.write_all(set_buffer!())?;

        FdWriter::stdout().write(buffer_size!())?;
        //let mut buf = MaybeUninit::<[u8; 32]>::uninit();
        //
        //let n = FdReader::stdin().read(unsafe { buf.assume_init_mut() })?;
        //let input = &unsafe { buf.assume_init_ref() }[..n];
        //
        //let mut buf = MaybeUninit::<[u8; 32]>::uninit();
        //let margin_left = if let Some([rows, cols]) = parse_buffer_size(input) {
        //    cursor_move(&mut ctx.writer, (rows - 5) / 2, Direction::Down)?;
        //    unsafe {
        //        let mut writer = SliceWriter::new(buf.assume_init_mut());
        //        cursor_move(&mut writer, (cols - 35) / 2, Direction::Right)?;
        //        let len = writer.len;
        //        Some(&buf.assume_init_ref()[..len])
        //    }
        //} else {
        //    None
        //};

        let mut time = MaybeUninit::uninit();
        unsafe {
            nc::time(time.assume_init_mut())?;
            let content = draw_time(time.assume_init() + 8 * 3600);
            ctx.draw(None, || content)?
        };
        ctx.writer.flush()?;
        unsafe {
            nc::nanosleep(
                &nc::timespec_t {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
                None,
            )
            .or_else(|e| match e {
                4 => Ok(()),
                e => Err(e),
            })?;
        }
        ctx.writer.write_all(restore_buffer!())?;
    }
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
