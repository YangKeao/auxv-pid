[![](https://img.shields.io/crates/v/auxv.svg)](https://crates.io/crates/auxv) [![](https://docs.rs/auxv/badge.svg)](https://docs.rs/auxv/)


Add the dependency to your `Cargo.toml`:

```toml
auxv = "0.3.0"
```

See the [documentation](https://docs.rs/auxv/).

### Running tests

Because the width of `unsigned long` varies between architectures, some tests are only run on 64-bit systems and others are only run on 32-bit systems. On a typical x64 Linux system with glibc, the default toolchain will be `x86_64-unknown-linux-gnu`, but you can run the 32-bit tests by installing the 32-bit glibc toolchain:

```
rustup target add i686-unknown-linux-gnu
```

To run tests for both 32bit and 64bit `c_ulong` on a 64-bit host:

```
cargo test
cargo test --target i686-unknown-linux-gnu
```

You should see different tests running in each case.

### Testing other OSs

There are various vagrant boxes defined in the Vagrantfile. Windows and macOS are a pain to set up build tools on, but eventually the code will compile there, though of course it won't work properly. It should work fine on all Linux boxes and the FreeBSD one.
