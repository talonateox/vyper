use vlib::{println, syscalls::mkdir};

pub fn run(args: &[&[u8]]) {
    if args.is_empty() {
        println!("usage: mkdir <file>");
        return;
    }

    let path = args[0];

    mkdir(path);
}
