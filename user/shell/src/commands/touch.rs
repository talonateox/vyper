use vlib::{println, syscalls::touch};

pub fn run(args: &[&[u8]]) {
    if args.is_empty() {
        println!("usage: touch <file>");
        return;
    }

    let path = args[0];

    touch(path);
}
