use vlib::syscalls::{DirEntryIter, O_DIRECTORY, O_RDONLY, close, getdents, open, read, write};

pub fn run(_args: &[&[u8]]) {
    let fd = open(b"/live/tasks", O_RDONLY | O_DIRECTORY);
    if fd < 0 {
        write(1, b"ps: failed to open /live/tasks\n");
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
                let mut path_buf = [0u8; 64];
                let path_len = build_path(&mut path_buf, entry.name());

                let status_fd = open(&path_buf[..path_len], O_RDONLY);
                if status_fd >= 0 {
                    let mut status_buf = [0u8; 256];
                    let n = read(status_fd as u64, &mut status_buf);
                    if n > 0 {
                        print_status(&status_buf[..n as usize]);
                    }
                    close(status_fd as u64);
                }
            }
        }
    }

    close(fd);
}

fn build_path(buf: &mut [u8], pid: &[u8]) -> usize {
    let prefix = b"/live/tasks/";
    let suffix = b"/status";

    let mut i = 0;

    for &b in prefix {
        buf[i] = b;
        i += 1;
    }

    for &b in pid {
        buf[i] = b;
        i += 1;
    }

    for &b in suffix {
        buf[i] = b;
        i += 1;
    }

    i
}

fn print_status(status: &[u8]) {
    let mut pid: &[u8] = b"?";
    let mut state: &[u8] = b"?";
    let mut mode: &[u8] = b"?";

    let mut i = 0;
    while i < status.len() {
        let line_start = i;

        while i < status.len() && status[i] != b'\n' {
            i += 1;
        }

        let line = &status[line_start..i];

        if line.starts_with(b"pid: ") {
            pid = &line[5..];
        } else if line.starts_with(b"state: ") {
            state = &line[7..];
        } else if line.starts_with(b"mode: ") {
            mode = &line[6..];
        }

        if i < status.len() && status[i] == b'\n' {
            i += 1;
        }
    }

    write(1, pid);
    write(1, b"\t");
    write(1, state);
    write(1, b"\t\t");
    write(1, mode);
    write(1, b"\n");
}
