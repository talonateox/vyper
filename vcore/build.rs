fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    println!("cargo:rustc-link-arg=-Tvcore/linker-{arch}.ld");
    println!("cargo:rerun-if-changed=vcore/linker-{arch}.ld");
}
