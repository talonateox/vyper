use vlib::{as_str, print, println};

pub fn run(args: &[&[u8]]) {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", as_str!(arg));
    }
    println!();
}
