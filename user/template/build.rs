fn main() {
    println!("cargo:rustc-link-arg=-Tuser/template/linker.ld");
    println!("cargo:rerun-if-changed=user/template/linker.ld");
}
