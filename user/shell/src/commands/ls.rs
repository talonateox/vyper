use vlib::syscalls::{DirEntryIter, O_DIRECTORY, O_RDONLY, close, getdents, open, write};

pub fn run(args: &[&[u8]]) {
    let path: &[u8] = if args.is_empty() { b"/" } else { args[0] };

    let fd = open(path, O_RDONLY | O_DIRECTORY);
    if fd < 0 {
        write(1, b"ls: cannot open '");
        write(1, path);
        write(1, b"'\n");
        return;
    }

    let fd = fd as u64;
    let mut buf = [0u8; 1024];

    loop {
        let bytes_read = getdents(fd, &mut buf);

        if bytes_read <= 0 {
            break;
        }

        for entry in DirEntryIter::new(&buf, bytes_read as usize) {
            if entry.is_dir() {
                write(1, b"[DIR]  ");
            } else {
                write(1, b"[FILE] ");
            }

            write(1, entry.name());
            write(1, b"\n");
        }
    }

    close(fd);
}
