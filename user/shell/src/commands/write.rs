use vlib::{
    as_str, println,
    syscalls::{O_APPEND, O_CREAT, O_WRONLY, close, open, write},
};

pub fn run(args: &[&[u8]]) {
    if args.is_empty() {
        println!("usage: write <file>");
        return;
    }

    let path = args[0];
    let fd = open(path, O_WRONLY | O_APPEND | O_CREAT);
    if fd < 0 {
        println!("write: failed to open or create '{}'", as_str!(path));
        return;
    }

    let fd = fd as u64;

    for (i, arg) in args[1..].iter().enumerate() {
        if i > 0 {
            write(fd, b" ");
        }
        write(fd, arg);
    }
    write(fd, b"\n");
    close(fd);
}
