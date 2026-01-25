fn main() {
    println!("cargo:rustc-link-search=native=lib");
    println!("cargo:rustc-link-lib=dylib=casc");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/../lib");
}
