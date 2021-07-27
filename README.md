The Windows command line is passed to applications as a string. To get an array of arguments it's necessary to parse this string, which is what this crate does. This list of arguments can then be used by higher level argument parsers.

It uses the latest C/C++ parsing rules so that it is consistent with using `argv` from a C/C++ program.

# Using

Add this to your `Cargo.toml` file

```ini
[dependencies.winarg]
version = "0.2.0"
```

# Example

```rust
for arg in winarg::args_native().skip(1) {
    if arg == "--help" {
        println!("help me!");
    }
}
```
