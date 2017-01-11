![](https://img.shields.io/crates/v/auxv.svg)

### Just what is the auxiliary vector?

The auxiliary vector is some memory near the start of a running Linux program's stack. Specifically, it's a sequence of pairs of `unsigned long`s, with each pair comprising a *type* and a *value*. It is mostly there to help things like runtime linkers, but sometimes it's useful for other reasons. It is Linux-specific; it does not exist on other Unixes.

The types used in the aux vector are defined in various header files and typically prefixed with `AT_`. Some of the data there is not available from any other source, like `AT_HWCAP` and `AT_HWCAP2`. These expose bit vectors of architecture-dependent hardware capability information. On ARM, for instance, the bit `1 << 12` in the value for `AT_HWCAP` will be set if the CPU supports NEON, and `1 << 3` will be set in the value for `AT_HWCAP2` if the CPU supports SHA-256 acceleration. Handy, if you're doing that sort of thing.

Other types are typically not used directly by programs, like `AT_UID`: the real user id is great and all, but you'd be better off calling [`getuid(2)`](https://linux.die.net/man/2/getuid) in C or `libc::getuid` from Rust.

More info on the auxiliary vector:

- http://articles.manugarg.com/aboutelfauxiliaryvectors.html
- http://phrack.org/issues/58/5.html
- See `include/uapi/linux/auxvec.h` in the Linux source (or `getauxval(3)`) for defined types, as well as other header files for architecture-specific types. Searching for `define AT_` is a good start.
- See `fs/binfmt_elf.c` in the Linux source for how the vector is generated.

### Reading the auxiliary vector

First, add to your `Cargo.toml`:

```toml
auxv = "0.1.0"
```

There are two ways to access the auxiliary vector:

- [`getauxval(3)`](https://linux.die.net/man/3/getauxval) is a glibc-only function for accessing the Linux auxiliary vector. It is available from glibc 2.16 onwards. Since it is a non-standard extension, if you're not using glibc (musl, uclibc, etc), this will not be available. Also, if you're on glibc older than 2.19, glibc is unable to express the concept of "not found" and will instead "find" the value 0.
- `/proc/self/auxv` exposes the contents of the aux vector, but the OS may be configured to not allow access to it, so that won't always work either.

This library lets you use both of these options, so chances are pretty good that at least one of them will work in any given host. For most users, it would be best practice to try the `getauxval` way first, and then try the procfs way if `getauxval` is not available at runtime. Note that due to the aforementioned bug in glibc older than 2.19 you may get a spurious `0` as a successfully found value, so if that's a problem in your environment, you will probably need to do some experimentation to find the best fallback logic.

For most people, probably the most interesting data in auxv is for `AT_HWCaP` or `AT_HWCAP2` so those have constants defined in `auxv`, but you can of course use any other type as well; you'll just have to look up the appropriate number.

#### Using `getauxval`

Because the underlying `getauxval` C function is weakly linked, and only available on Linux, access to it is done via the trait `Getauxval` to provide some indirection. On `target_os="linux"`, the struct `NativeGetauxval` will be available, and that will call through to `getauxval` if it is available and return an appropriate error if it is not.

On all OSs, you can use `NotFoundGetauxval`. It (surprise!) always returns the error that indicates that the requested type was not found. Of course, you can also use any other stub implementation of the trait that you choose for testing, etc.

#### Using procfs

Since it's just doing file I/O and not odd linkage tricks, the code to work with procfs is available on all OSs but of course will return an error on non-Linux since it won't be able to find `/proc/self/auxv` (or anything else in `/proc`).

If you want a convenient way to query for just a handful of types, `search_procfs_auxv` is a good choice. You provide a slice of `c_ulong` types to look for, and it builds a map of type to value for the types you specify.

If, on the other hand, you want to inspect everything in the aux vector, `iterate_procfs_auxv` is what you want. It will let you iterate over every type/value pair in the aux vector. A minor wrinkle is that there are two layers of `Result`: one for around the initial `Iterator`, and another around each type/value pair. C'est la vie.

### Running tests

Because the width of `unsigned long` varies between architectures, some tests are only run on 64-bit systems and others are only run on 32-bit systems. However, the mapping isn't as simple as just the pointer width: there are some 64-bit systems with a 32-bit `unsigned long`.
 
 Anyway, on a typical x64 Linux system with glibc, the default toolchain will be `x86_64-unknown-linux-gnu`, but you can run the 32-bit tests by installing the 32-bit glibc toolchain:

```
rustup target add i686-unknown-linux-gnu
```

`build.rs` detects what the width of `c_ulong` is and sets a feature so that only the relevant tests will run by default. However, `build.rs` won't run with the selected target (it always runs with the default), so even when compiling the tests with a 32-bit toolchain, `build.rs` will think `unsigned long` is 64 bits. Therefore, we need to set a feature to override that detection. To force the 32-bit tests to run, set the feature `auxv-32bit-ulong`. If for some reason you want to force 64-bit tests, use `auxv-64bit-ulong`.

To run tests for both 32bit and 64bit `c_ulong` on a 64-bit host:

```
cargo test
cargo test --target i686-unknown-linux-gnu --feature auxv-32bit-ulong
```

You should see different tests running in each case.

### Testing weak linking with ancient glibc

This isn't worth the hassle to automate, but it's not hard with some help from Vagrant.

- Run `vagrant up` to spin up a VM of an old Debian box.
- `vagrant ssh` to shell in once it's booted
- `cd /vagrant && cargo test`
- You should see two test failures because `getauxval` is not available like this:

```
---- test_getauxv_hwcap_linux_doesnt_find_bogus_type stdout ----
        thread 'test_getauxv_hwcap_linux_doesnt_find_bogus_type' panicked at 'assertion failed: `(left == right)` (left: `NotFound`, right: `FunctionNotAvailable`)', tests/native_getauxval_tests.rs:26
note: Run with `RUST_BACKTRACE=1` for a backtrace.

---- test_getauxv_hwcap_linux_finds_hwcap stdout ----
        thread 'test_getauxv_hwcap_linux_finds_hwcap' panicked at 'called `Result::unwrap()` on an `Err` value: FunctionNotAvailable', ../src/libcore/result.rs:837
```

You can run `vagrant halt` to shut down the VM or `vagrant destroy` to delete it entirely.
