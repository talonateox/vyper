use vlib::{as_str, println, syscalls::getcwd};

pub fn run(_args: &[&[u8]]) {
    let mut buf = [0u8; 256];
    let len = getcwd(&mut buf);

    if len < 0 {
        println!("pwd: failed to get current directory");
    } else {
        println!("{}", as_str!(&buf[..len as usize]));
    }
}
