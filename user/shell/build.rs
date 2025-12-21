fn main() {
    println!("cargo:rustc-link-arg=-Tuser/shell/linker.ld");
    println!("cargo:rerun-if-changed=user/shell/linker.ld");
}
