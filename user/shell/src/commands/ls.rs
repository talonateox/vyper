use vlib::{
    as_str, println,
    syscalls::{DirEntryIter, O_DIRECTORY, O_RDONLY, close, getdents, open},
};

pub fn run(args: &[&[u8]]) {
    let path: &[u8] = if args.is_empty() { b"/" } else { args[0] };

    let fd = open(path, O_RDONLY | O_DIRECTORY);
    if fd < 0 {
        println!("cannot open '{}'", as_str!(path));
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
                println!("[DIR]  {}", as_str!(entry.name()));
            } else {
                println!("[FILE] {}", as_str!(entry.name()));
            }
        }
    }

    close(fd);
}
