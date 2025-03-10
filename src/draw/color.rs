use crate::{draw::COLOR_SEQUENCE_SISE, io};

#[derive(Clone, Copy)]
pub enum Literal {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

#[derive(Clone, Copy)]
pub enum Color {
    Normal(Literal),
    Bright(Literal),
    Ansi(u8),
    Rgb { r: u8, g: u8, b: u8 },
    Clear,
}

impl Color {
    #[must_use]
    pub const fn ansi_sequence_fg(self, buf: &mut [u8; COLOR_SEQUENCE_SISE]) -> usize {
        let mut writer = io::SliceWriter::new(buf);
        unsafe {
            match self {
                Color::Normal(literal) => {
                    writer.write_bytes_unchecked(b"[");
                    writer.write_u64_unchecked(literal as u64 + 30);
                    writer.write_bytes_unchecked(b"m");
                }
                Color::Bright(literal) => {
                    writer.write_bytes_unchecked(b"[");
                    writer.write_u64_unchecked(literal as u64 + 90);
                    writer.write_byte_unchecked(b'm');
                }
                Color::Ansi(n) => {
                    writer.write_bytes_unchecked(b"[38;5;");
                    writer.write_u64_unchecked(n as u64);
                    writer.write_byte_unchecked(b'm');
                }
                Color::Rgb { r, g, b } => {
                    writer.write_bytes_unchecked(b"[38;2;");
                    writer.write_u64_unchecked(r as u64);
                    writer.write_byte_unchecked(b';');
                    writer.write_u64_unchecked(g as u64);
                    writer.write_byte_unchecked(b';');
                    writer.write_u64_unchecked(b as u64);
                    writer.write_byte_unchecked(b'm');
                }
                Color::Clear => writer.write_bytes_unchecked(b"[39m"),
            }
        }
        writer.len
    }
    #[must_use]
    pub const fn ansi_sequence_bg(self, buf: &mut [u8; COLOR_SEQUENCE_SISE]) -> usize {
        let mut writer = io::SliceWriter::new(buf);
        unsafe {
            match self {
                Color::Normal(literal) => {
                    writer.write_bytes_unchecked(b"[");
                    writer.write_u64_unchecked(literal as u64 + 40);
                    writer.write_bytes_unchecked(b"m");
                }
                Color::Bright(literal) => {
                    writer.write_bytes_unchecked(b"[");
                    writer.write_u64_unchecked(literal as u64 + 100);
                    writer.write_byte_unchecked(b'm');
                }
                Color::Ansi(n) => {
                    writer.write_bytes_unchecked(b"[48;5;");
                    writer.write_u64_unchecked(n as u64);
                    writer.write_byte_unchecked(b'm');
                }
                Color::Rgb { r, g, b } => {
                    writer.write_bytes_unchecked(b"[48;2;");
                    writer.write_u64_unchecked(r as u64);
                    writer.write_byte_unchecked(b';');
                    writer.write_u64_unchecked(g as u64);
                    writer.write_byte_unchecked(b';');
                    writer.write_u64_unchecked(b as u64);
                    writer.write_byte_unchecked(b'm');
                }
                Color::Clear => writer.write_bytes_unchecked(b"[49m"),
            }
        }
        writer.len
    }
}
