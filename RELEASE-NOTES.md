### 0.3.3

- Documentation improvements
- Add crate category

### 0.3.2

- Fix unused warnings on non-Linux OSs

### 0.3.1

- Split each way of accessing auxv into its own mod
- Add `stack` mod for navigating the ELF stack directly from the `environ` pointer
- Choose integer type based on pointer width instead of using `c_ulong`

### 0.2.0

- Add support for iterating over all auxv entries
- Change `GetauxvalProvider` to `Getauxval`.

### 0.1.0

- Initial release
