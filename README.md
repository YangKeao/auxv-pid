[![](https://img.shields.io/crates/v/auxv.svg)](https://crates.io/crates/auxv) [![](https://docs.rs/auxv/badge.svg)](https://docs.rs/auxv/)

### Just what is the auxiliary vector?

The auxiliary vector is some memory near the start of a running ELF program's stack. Specifically, it's a sequence of pairs of either 64 bit or 32 bit unsigned ints. The two components of the pair form a key and a value. This data is mostly there to help things like runtime linkers, but sometimes it's useful for other reasons. It is ELF-specific; it does not exist in, say, Mach-O.

If you're on a system with ELF executables (Linux, FreeBSD, other Unixes), run the example that shows its own auxv keys and values:

```
cargo run --example elf_stack_show_auxv
```

If you're unsure about whether a particular system will work, clone this repo and run the example listed at the top of the README. On Linux, FreeBSD, and other ELF systems, it should print a short table of a dozen or two numbers. On macOS, it tends to produce garbage numbers for a while before mercifully exiting normally. On Windows, the function is not available because their names are not POSIX compatible so it wouldn't even compile, so the example prints nothing.

The keys used in the aux vector are defined in various header files and typically prefixed with `AT_`. Some of the data there is not available from any other source, like `AT_HWCAP` and `AT_HWCAP2`. These expose bit vectors of architecture-dependent hardware capability information. On ARM, for instance, the bit `1 << 12` in the value for `AT_HWCAP` will be set if the CPU supports NEON, and `1 << 3` will be set in the value for `AT_HWCAP2` if the CPU supports SHA-256 acceleration. Handy, if you're doing that sort of thing.

Other keys are typically not used directly by programs, like `AT_UID`: the real user id is great and all, but you'd be better off calling [`getuid(2)`](https://linux.die.net/man/2/getuid) in C or `libc::getuid` from Rust.

For most people, probably the most interesting data in auxv is for `AT_HWCaP` or `AT_HWCAP2` so those have constants defined in `auxv`, but you can of course use any other type as well; you'll just have to look up the appropriate number.

More info on the auxiliary vector:

- http://articles.manugarg.com/aboutelfauxiliaryvectors.html
- http://phrack.org/issues/58/5.html
- See `include/uapi/linux/auxvec.h` in the Linux source (or `getauxval(3)`) for defined types, as well as other header files for architecture-specific types.
- See `fs/binfmt_elf.c` in the Linux source for how the vector is generated.
- Searching for `AT_` in your OS of choice is likely to yield some good leads on the available constants and how it's generated.

### Reading the auxiliary vector

Add the dependency to your `Cargo.toml`:

```toml
auxv = "0.2.0"
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
