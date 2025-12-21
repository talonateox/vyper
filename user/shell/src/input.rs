use vlib::syscalls::{getch, write};

pub fn read_line(buf: &mut [u8]) -> usize {
    let mut pos = 0;

    loop {
        let c = getch();

        match c {
            b'\n' => {
                write(1, b"\n");
                return pos;
            }
            8 | 127 => {
                if pos > 0 {
                    pos -= 1;
                    write(1, b"\x08 \x08");
                }
            }
            32..=126 => {
                if pos < buf.len() - 1 {
                    buf[pos] = c;
                    pos += 1;
                    write(1, &[c]);
                }
            }
            _ => {}
        }
    }
}

pub fn parse_args<'a>(line: &'a [u8], argv: &mut [&'a [u8]]) -> usize {
    let mut argc = 0;
    let mut i = 0;

    while i < line.len() && argc < argv.len() {
        while i < line.len() && line[i] == b' ' {
            i += 1;
        }

        if i >= line.len() {
            break;
        }

        let start = i;
        while i < line.len() && line[i] != b' ' {
            i += 1;
        }

        argv[argc] = &line[start..i];
        argc += 1;
    }

    argc
}
