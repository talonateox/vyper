use vlib::syscalls::write;

use crate::input::parse_args;

mod cat;
mod echo;
mod help;
mod ls;
mod ps;

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
        b"exit" => {
            write(1, b"byebye o7\n");
            vlib::syscalls::exit(0);
        }
        _ => {
            write(1, b"command '");
            write(1, cmd);
            write(1, b"' doesnt exist.\n");
            write(1, b"use 'help' for a list of commands.\n");
        }
    }
}
