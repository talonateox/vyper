use vlib::{as_str, println};

use crate::input::parse_args;

mod cat;
mod echo;
mod help;
mod ls;
mod mkdir;
mod ps;
mod rm;
mod rmdir;
mod touch;

pub fn execute(line: &[u8]) {
    let mut argv: [&[u8]; 16] = [&[]; 16];
    let argc = parse_args(line, &mut argv);

    if argc == 0 {
        return;
    }

    let cmd = argv[0];
    let args = &argv[1..argc];

    match cmd {
        b"help" => help::run(args),
        b"echo" => echo::run(args),
        b"ls" => ls::run(args),
        b"cat" => cat::run(args),
        b"ps" => ps::run(args),
        b"touch" => touch::run(args),
        b"mkdir" => mkdir::run(args),
        b"rm" => rm::run(args),
        b"rmdir" => rmdir::run(args),
        b"exit" => {
            println!("byebye o7");
            vlib::syscalls::exit(0);
        }
        _ => {
            println!(
                "command '{}' doesnt exist\nuse 'help' for a list of commands.",
                as_str!(cmd)
            );
        }
    }
}
