use vlib::{println, syscalls::rmdir};

pub fn run(args: &[&[u8]]) {
    if args.is_empty() {
        println!("usage: rmdir <directory>");
        return;
    }

    let path = args[0];

    rmdir(path);
}
