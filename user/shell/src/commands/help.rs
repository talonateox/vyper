use vlib::println;

pub fn run(_args: &[&[u8]]) {
    println!("commands:");
    println!("  help          - what you're looking at");
    println!("  echo <text>   - echo some text");
    println!("  ls <dir>      - list a directories contents");
    println!("  touch <file>  - create a file");
    println!("  mkdir <dir>   - create a directory");
    println!("  rm <file>     - delete a file");
    println!("  rmdir <dir    - delete a directory");
    println!("  cat <file>    - display a files content");
    println!("  ps            - list running tasks in /live/tasks");
    println!("  exit          - say byebye to the shell :c");
}
