fn main() {
    println!("cargo:rustc-link-search=native=lib");
    println!("cargo:rustc-link-lib=dylib=casc");

    // Embed rpaths so the binary finds libcasc without LD_LIBRARY_PATH /
    // DYLD_LIBRARY_PATH. Each is relative to the directory holding the executable:
    //   target/{debug,release}/       -> ../../lib
    //   target/{debug,release}/deps/  -> ../../../lib   (test and bench binaries)
    //   alongside the binary          -> ./lib          (release packaging layout)
    //
    // The token differs by platform — `$ORIGIN` for ELF, `@loader_path` for
    // Mach-O — and using the wrong one silently yields no usable rpath at all.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let origin = match target_os.as_str() {
        "macos" | "ios" => "@loader_path",
        // Windows resolves DLLs by search path, not rpath.
        "windows" => return,
        _ => "$ORIGIN",
    };

    for suffix in ["/../../lib", "/../../../lib", "/lib"] {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}{}", origin, suffix);
    }

    // ELF linkers may mark the rpath as "new dtags" (RUNPATH), which is not
    // inherited by transitive loads; libcasc has no further dependencies here, so
    // RUNPATH is sufficient and no extra flag is needed.
}
