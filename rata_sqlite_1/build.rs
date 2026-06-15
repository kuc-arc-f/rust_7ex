fn main() {
    println!("cargo:rerun-if-changed=sample.dll");
    println!("cargo:rustc-link-search=native=.");
    println!("cargo:rustc-link-lib=dylib=sample");
}