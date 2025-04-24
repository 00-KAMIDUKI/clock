# Clock - A Minimalist TUI Digital Clock
![Screenshot](https://github.com/user-attachments/assets/882b54be-0387-429b-a1bc-999a6cc90854)

A featherweight digital clock for terminal (TUI),
written in Rust with zero dependencies (no stdlib, no libc).
Designed for Linux systems.

## Key Features
- 🕒 Real-time digital clock display in terminal
- ⚡ 10KB compiled binary size (not stripped)
- 🦀 No-std Rust implementation
- 🚫 No libc dependency (100% pure syscalls)
- ⌨️ Simple keyboard controls (quit with `q` or `Ctrl-C`)

## Build & run (requires Rust nightly)
```sh
rustup toolchain install nightly
cargo +nightly build --release && ./target/release/clock
```

## Requirements
- linux kernel version >=5.4
- x86-64 (more architecures will be supported in the future)

## Coming Features
⏱️ Stopwatch | ⏲️ Timer | 🌐 Timezones
🖥️ More architectures
