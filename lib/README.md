# lib/

This directory holds the compiled CascLib shared library required at runtime.

## What is CascLib?

[CascLib](https://github.com/ladislav-zezula/CascLib) is an open source C++ library by Ladislav Zezula
for reading Blizzard CASC (Content Addressable Storage Container) archives. This project links against
it at runtime via Rust FFI to open and read files from game installations.

## Building CascLib

### macOS

```bash
git clone https://github.com/ladislav-zezula/CascLib
cd CascLib
mkdir build && cd build
cmake .. -DCASC_BUILD_SHARED_LIB=ON
make
cp libcasc.dylib /path/to/blizzard-casc-extractor/lib/
```

### Linux

```bash
git clone https://github.com/ladislav-zezula/CascLib
cd CascLib
mkdir build && cd build
cmake .. -DCASC_BUILD_SHARED_LIB=ON
make
cp libcasc.so /path/to/blizzard-casc-extractor/lib/
```

After placing the library here, set the dynamic linker path when running:

```bash
# macOS
DYLD_LIBRARY_PATH=lib ./target/release/casc-extractor

# Linux
LD_LIBRARY_PATH=lib ./target/release/casc-extractor
```

## Note on Windows

Windows support is not currently tested. Building a DLL with cmake should be possible but may require
additional configuration (`.lib` import library, adjusted `build.rs` link directives, etc.).
