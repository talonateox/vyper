use vlib::{as_str, println, syscalls::chdir};

pub fn run(args: &[&[u8]]) {
    let path: &[u8] = if args.is_empty() { b"/" } else { args[0] };

    if chdir(path) < 0 {
        println!("cd: no such directory '{}'", as_str!(path));
    }
}
