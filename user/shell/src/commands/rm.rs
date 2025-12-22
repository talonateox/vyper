use vlib::{println, syscalls::unlink};

pub fn run(args: &[&[u8]]) {
    if args.is_empty() {
        println!("usage: rm <file>");
        return;
    }

    let path = args[0];

    unlink(path);
}
