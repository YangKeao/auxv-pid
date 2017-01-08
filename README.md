### Just what is the auxiliary vector?

The auxiliary vector is some memory near the start of a running Linux program's stack. Specifically, it's a sequence of pairs of `unsigned long`s, with each pair comprising a *type* and a *value*. It is mostly there to help things like runtime linkers, but sometimes it's useful for other reasons.

The types used in the aux vector are typically prefixed with `AT_`. Some of the data there is not available from any other source, like `AT_HWCAP` and `AT_HWCAP2`. These expose bit vectors of architecture-dependent hardware capability information. On ARM, for instance, the bit `1 << 12` in the value for `AT_HWCAP` will be set if the CPU supports NEON, and `1 << 3` will be set in the value for `AT_HWCAP2` if the CPU supports SHA-256 acceleration. Handy, if you're doing that sort of thing.

Other types are typically not used directly by programs, like `AT_UID`: the real user id is great and all, but you'd be better off calling [`getuid(2)`](https://linux.die.net/man/2/getuid) in C or `libc::getuid` from Rust. 

More info on the auxiliary vector:

- http://articles.manugarg.com/aboutelfauxiliaryvectors.html
- http://phrack.org/issues/58/5.html
- See `include/uapi/linux/auxvec.h` in the Linux source (or `getauxval(3)` )for defined types
- See `fs/binfmt_elf.c` in the Linux source for how the vector is generated

### OK, how do I use it it?

There are two ways to access the auxiliary vector:

- [`getauxval(3)`](https://linux.die.net/man/3/getauxval) is a glibc-only function for accessing the Linux auxiliary vector. Since it is a non-standard extension, if you're not using glibc (musl, uclibc, etc), this will not be available. (Also, since we're talking about glibc issues, if you're on glibc older than 2.19, glibc is unable to express the concept of "not found" and will instead "find" the value 0.)
- `/proc/self/auxv` exposes the contents of the aux vector, but the OS may be configured to not allow access to it, so that won't always work either.

This library exposes both of them, so chances are pretty good that at least one of them will work in any given host. For most users, it would be best practice to try the `getauxval` way first, and then try the procfs way if `getauxval` is not available at runtime. Note that due to the aforementioned bug in glibc older than 2.19 you may get a spurious `0` as a successfully found value, so if that's a problem in your environment, you will probably need to do some experimentation to find the best fallback logic.

For most people, probably the most interesting data in auxv is for `AT_HWCaP` or `AT_HWCAP2` so those have constants defined in `auxv`, but you can of course use any other type as well; you'll just have to look up the appropriate number.

#### Using `getauxval`

Because the underlying `getauxval` C function is weakly linked, and only available on Linux, access to it is done via the trait `GetauxvalProvider` to provide some indirection. On `target_os="linux"`, the struct `NativeGetauxvalProvider` will be available, and that will call through to `getauxval` if it is available and return an appropriate error if it is not.

When you're not on Linux, you can use `NotFoundGetauxvalProvider`. It (surprise!) always returns the error that indicates that the requested type was not found. Of course, you can also use any other stub implementation of the trait that you choose for testing, etc.

If you wanted to switch between providers at compile time but always use the same auxv-related logic, you could do something like the following.

```rust
extern crate auxv;

use auxv::{GetauxvalProvider, AT_HWCAP};
#[cfg(target_os="linux")]
use auxv::NativeGetauxvalProvider;
#[cfg(not(target_os="linux"))]
use auxv::NotFoundGetauxvalProvider;

fn do_stuff_with_getauxval<G: GetauxvalProvider>(g: G) {
    // naturally, don't unwrap() in real code
    let hwcap = g.getauxval(AT_HWCAP).unwrap();
    // poke around in hwcap, etc
}

#[cfg(target_os="linux")]
fn detect_hardware() {
    let getauxval = NativeGetauxvalProvider {};
    do_stuff_with_getauxval(getauxval);
}

#[cfg(not(target_os="linux"))]
fn detect_hardware() {
    let getauxval = NotFoundGetauxvalProvider {};
    do_stuff_with_getauxval(getauxval);
}
```

#### Using procfs

`getauxval` only handles looking up one type at a time. Reading procfs, on the other hand, will see all types, but generally you only care about a couple of types, so the Rust wrapper accepts a slice of types to look for.

Since it's just doing file I/O and not odd linkage tricks, the function is available on all OSs but of course will only return an error on non-Linux since it won't be able to find `/proc/self/auxv` (or anything else in `/proc`).

```rust
extern crate auxv;

use auxv::{search_procfs_auxv, AT_HWCAP};

let data = search_procfs_auxv(&[AT_HWCAP]).unwrap();
assert!(*data.get(&AT_HWCAP).unwrap() > 0);
```

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
