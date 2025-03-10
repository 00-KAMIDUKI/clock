use color::Color;

use crate::io::{self, Write};

pub mod color;

pub const COLOR_SEQUENCE_SISE: usize = 19;

pub struct Cache {
    color_on: [u8; COLOR_SEQUENCE_SISE],
    color_on_len: u8,
    color_off: [u8; COLOR_SEQUENCE_SISE],
    color_off_len: u8,
}

impl Cache {
    pub const fn new(on: Color, off: Color) -> Self {
        let mut color_on = [0; 19];
        let color_on_len = on.ansi_sequence_bg(&mut color_on) as _;

        let mut color_off = [0; 19];
        let color_off_len = off.ansi_sequence_bg(&mut color_off) as u8;

        Self {
            color_on,
            color_on_len,
            color_off,
            color_off_len,
        }
    }

    fn on_seq(&self) -> &[u8] {
        unsafe { self.color_on.get_unchecked(..self.color_on_len as usize) }
    }

    fn off_seq(&self) -> &[u8] {
        unsafe { self.color_off.get_unchecked(..self.color_off_len as usize) }
    }

    fn seq(&self, switch: bool) -> &[u8] {
        match switch {
            true => self.on_seq(),
            false => self.off_seq(),
        }
    }
}

pub struct Context<Writer: Write> {
    pub writer: Writer,
    cache: Cache,
    state: Option<bool>,
}

#[derive(Clone, Copy)]
pub struct Draw {
    spaces: i8,
}

impl Draw {
    const fn new(spaces: i8) -> Self {
        Self { spaces }
    }
    const fn on(spaces: u8) -> Self {
        Self::new(spaces as _)
    }

    const fn off(spaces: u8) -> Self {
        Self::new(-(spaces as i8))
    }

    const NOP: Self = Self::new(0);

    const LONG: [Self; 3] = [Self::on(5), Self::NOP, Self::NOP];
    const LEFT: [Self; 3] = [Self::on(2), Self::off(3), Self::NOP];
    const RIGHT: [Self; 3] = [Self::off(3), Self::on(2), Self::NOP];
    const LEFT_AND_RIGHT: [Self; 3] = [Self::on(2), Self::off(1), Self::on(2)];
    const ONE: [Self; 3] = [Self::off(1), Self::on(2), Self::off(1)];
}

impl<Writer: Write> Context<Writer> {
    pub const fn new(writer: Writer, cache: Cache) -> Self {
        Self {
            writer,
            cache,
            state: None,
        }
    }
    pub const fn from_colors(writer: Writer, on: Color, off: Color) -> Self {
        Self::new(writer, Cache::new(on, off))
    }

    fn switch(&mut self, v: bool) -> io::Result<()> {
        match self.state {
            Some(x) if x == v => Ok(()),
            _ => self
                .writer
                .write_all(self.cache.seq(v))
                .map(|_| self.state = Some(v)),
        }
    }

    fn space(&mut self, n: usize) -> io::Result<()> {
        const SPACES: [u8; 5] = [b' '; 5];
        self.writer.write_all(unsafe { &SPACES.get_unchecked(..n) })
    }

    fn do_draw(&mut self, Draw { spaces }: Draw) -> io::Result<()> {
        if spaces == 0 {
            return Ok(());
        }
        self.switch(spaces.is_positive())?;
        self.space(spaces.abs() as _)
    }

    pub fn draw<R: IntoIterator<Item = &'static DrawLineN>>(
        &mut self,
        margin_left: Option<&[u8]>,
        string: impl Fn() -> R,
    ) -> io::Result<()> {
        for line in 0..LINE_COUNT {
            if let Some(x) = margin_left {
                self.writer.write_all(x)?;
            }
            for &draw_line_n in string() {
                let draw_list = draw_line_n[line];
                for draw in draw_list {
                    self.do_draw(draw)?;
                }
                self.do_draw(Draw::off(1))?;
            }
            self.writer.write(b"\n")?;
        }
        Ok(())
    }

    //pub fn draw_text_array<const N: usize>(&mut self, s: &[u8; N]) -> io::Result<()> {
    //    self.draw(|| s.map(|x| DrawItem(x).draw_line_n()))
    //}
    //
    //pub fn draw_text(&mut self, s: &[u8]) -> io::Result<()> {
    //    self.draw(|| s.iter().map(|&x| DrawItem(x).draw_line_n()))
    //}
    //
    //pub fn draw_time(&mut self, seconds: usize) -> io::Result<()> {
    //    let [s, min, h] = time(seconds);
    //    let arr = unsafe {
    //        [
    //            DIGITS.get_unchecked(h / 10),
    //            DIGITS.get_unchecked(h % 10),
    //            &COLON,
    //            DIGITS.get_unchecked(min / 10),
    //            DIGITS.get_unchecked(min % 10),
    //            &COLON,
    //            DIGITS.get_unchecked(s / 10),
    //            DIGITS.get_unchecked(s % 10),
    //        ]
    //    };
    //    self.draw(|| arr)
    //}
}

pub fn draw_time(seconds: isize) -> [&'static DrawLineN; 8] {
    let [s, min, h] = time(seconds);
    let arr = unsafe {
        [
            DIGITS.get_unchecked((h / 10) as usize),
            DIGITS.get_unchecked((h % 10) as usize),
            &COLON,
            DIGITS.get_unchecked((min / 10) as usize),
            DIGITS.get_unchecked((min % 10) as usize),
            &COLON,
            DIGITS.get_unchecked((s / 10) as usize),
            DIGITS.get_unchecked((s % 10) as usize),
        ]
    };
    arr
}

#[must_use]
pub fn time(seconds: isize) -> [isize; 3] {
    let s = seconds % 60;
    let min = (seconds / 60) % 60;
    let h = (seconds / 3600) % 24;
    [s, min, h]
}

const LINE_COUNT: usize = 5;
type DrawLineN = [[Draw; 3]; LINE_COUNT];

const DIGITS: [DrawLineN; 10] = [
    [
        Draw::LONG,
        Draw::LEFT_AND_RIGHT,
        Draw::LEFT_AND_RIGHT,
        Draw::LEFT_AND_RIGHT,
        Draw::LONG,
    ],
    [Draw::ONE; LINE_COUNT],
    [Draw::LONG, Draw::RIGHT, Draw::LONG, Draw::LEFT, Draw::LONG],
    [Draw::LONG, Draw::RIGHT, Draw::LONG, Draw::RIGHT, Draw::LONG],
    [
        Draw::LEFT_AND_RIGHT,
        Draw::LEFT_AND_RIGHT,
        Draw::LONG,
        Draw::RIGHT,
        Draw::RIGHT,
    ],
    [Draw::LONG, Draw::LEFT, Draw::LONG, Draw::RIGHT, Draw::LONG],
    [
        Draw::LONG,
        Draw::LEFT,
        Draw::LONG,
        Draw::LEFT_AND_RIGHT,
        Draw::LONG,
    ],
    [
        Draw::LONG,
        Draw::RIGHT,
        Draw::RIGHT,
        Draw::RIGHT,
        Draw::RIGHT,
    ],
    [
        Draw::LONG,
        Draw::LEFT_AND_RIGHT,
        Draw::LONG,
        Draw::LEFT_AND_RIGHT,
        Draw::LONG,
    ],
    [
        Draw::LONG,
        Draw::LEFT_AND_RIGHT,
        Draw::LONG,
        Draw::RIGHT,
        Draw::LONG,
    ],
];

const COLON: DrawLineN = [
    [Draw::off(1), Draw::NOP, Draw::NOP],
    [Draw::on(1), Draw::NOP, Draw::NOP],
    [Draw::off(1), Draw::NOP, Draw::NOP],
    [Draw::on(1), Draw::NOP, Draw::NOP],
    [Draw::off(1), Draw::NOP, Draw::NOP],
];

//struct DrawItem(u8);
//
//impl DrawItem {
//    fn draw_line_n(self) -> &'static DrawLineN {
//        match self {
//            Self(b':') => &COLON,
//            Self(x) if (b'0'..=b'9').contains(&x) => &DIGITS[(x - b'0') as usize],
//            _ => crate::utils::unreachable(),
//        }
//    }
//}
