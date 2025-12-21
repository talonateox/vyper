use core::fmt::{self, Write};

use crate::syscalls::write;

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write(1, s.as_bytes());
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

pub fn _print_bytes(bytes: &[u8]) {
    write(1, bytes);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::io::_print(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {{
        $crate::io::_print(format_args!($($arg)*));
        $crate::print!("\n");
    }};
}

#[macro_export]
macro_rules! print_bytes {
    ($bytes:expr) => {{
        $crate::io::_print_bytes($bytes);
    }};
}

#[macro_export]
macro_rules! println_bytes {
    ($bytes:expr) => {{
        $crate::io::_print_bytes($bytes);
        $crate::print!("\n");
    }};
}

#[macro_export]
macro_rules! as_str {
    ($bytes:expr) => {{ unsafe { core::str::from_utf8_unchecked($bytes) } }};
}
