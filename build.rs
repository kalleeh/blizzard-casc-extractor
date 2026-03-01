fn main() {
    println!("cargo:rustc-link-search=native=lib");
    println!("cargo:rustc-link-lib=dylib=casc");
    // @loader_path is the directory containing the binary:
    //   debug:   target/debug/   -> ../../lib = tools/casc-extractor/lib/
    //   release: target/release/ -> ../../lib = tools/casc-extractor/lib/
    // The dylib's own install name is @rpath/casc.framework/Versions/1.0.0/casc,
    // so lib/casc.framework/Versions/1.0.0/casc must resolve (symlink created below).
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/../../lib");
}
