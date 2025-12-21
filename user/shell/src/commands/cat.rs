use vlib::{
    as_str, print, println,
    syscalls::{O_RDONLY, close, open, read},
};

pub fn run(args: &[&[u8]]) {
    if args.is_empty() {
        println!("usage: cat <file>");
        return;
    }

    let path = args[0];

    let fd = open(path, O_RDONLY);
    if fd < 0 {
        println!("cat: cannot open '{}'", as_str!(path));
        return;
    }

    let fd = fd as u64;
    let mut buf = [0u8; 512];

    loop {
        let bytes_read = read(fd, &mut buf);
        if bytes_read == 0 {
            break;
        }
        print!("{}", as_str!(&buf[..bytes_read as usize]));
    }

    close(fd);
}
