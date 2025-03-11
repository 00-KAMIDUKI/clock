use crate::io::{self, Write};

pub mod color;

pub const COLOR_SEQUENCE_SISE: usize = 19;

pub struct Context<Writer: Write> {
    pub writer: Writer,
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
    const ONE: [Self; 3] = [Self::off(1), Self::on(2), Self::NOP];
}

impl<Writer: Write> Context<Writer> {
    pub const fn new(writer: Writer) -> Self {
        Self { writer }
    }

    fn space(&mut self, n: usize) -> io::Result<()> {
        const SPACES: [u8; 3] = [b' '; 3];
        self.writer.write_all(unsafe { &SPACES.get_unchecked(..n) })
    }

    fn block(&mut self, n: usize) -> io::Result<()> {
        const BLOCKS: &[u8] = "█████".as_bytes();
        self.writer
            .write_all(unsafe { &BLOCKS.get_unchecked(..n * 3) })
    }

    fn do_draw(&mut self, Draw { spaces }: Draw) -> io::Result<()> {
        match spaces.signum() {
            1 => self.block(spaces as _),
            -1 => self.space(-spaces as _),
            _ => Ok(()),
        }
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
