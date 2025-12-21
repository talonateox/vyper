use vlib::syscalls::write;

pub fn run(args: &[&[u8]]) {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            write(1, b" ");
        }
        write(1, arg);
    }
    write(1, b"\n");
}
