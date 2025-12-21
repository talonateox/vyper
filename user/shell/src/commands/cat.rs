use vlib::syscalls::{O_RDONLY, close, open, read, write};

pub fn run(args: &[&[u8]]) {
    if args.is_empty() {
        write(1, b"usage: cat <file>\n");
        return;
    }

    let path = args[0];

    let fd = open(path, O_RDONLY);
    if fd < 0 {
        write(1, b"cat: cannot open '");
        write(1, path);
        write(1, b"'\n");
        return;
    }

    let fd = fd as u64;
    let mut buf = [0u8; 512];

    loop {
        let bytes_read = read(fd, &mut buf);
        if bytes_read == 0 {
            break;
        }
        write(1, &buf[..bytes_read as usize]);
    }

    close(fd);
}
