use vlib::syscalls::write;

pub fn run(_args: &[&[u8]]) {
    write(1, b"commands:\n");
    write(1, b"  help          - what your looking at\n");
    write(1, b"  echo <text>   - echo some text\n");
    write(1, b"  ls <dir>      - list a directories contents\n");
    write(1, b"  exit          - say byebye to the shell :c\n");
}
