# Objective-Rust (`objrust`)

Objective-Rust is a small source-to-source transpiler that adds class-style syntax to Rust. It translates `.orst` files into ordinary Rust, then can ask `rustc` to compile the generated code.

## Requirements

- Rust toolchain installed with `rustc` and Cargo available on `PATH`.

## Install

After this crate is published to crates.io:

```powershell
cargo install objrust-compiler
```

Or install the current GitHub version directly:

```powershell
cargo install --git https://github.com/amrhasanain/objrust.git objrust-compiler
```

## Quick start

Create `hello.orst`:

```rust
class Animal {
    pub name: String,

    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn speak(&self) {
        println!("Hello, I'm {}", self.name);
    }
}

fn main() {
    let cat = Animal::new("Meshmesh".to_string());
    cat.speak();
}
```

Compile and run it:

```powershell
objrust hello.orst --run
```

By default, this writes an executable named after the input file. The generated Rust source is kept in `objrust-output/` so it can be inspected when needed.

## CLI

```text
objrust <input.orst> [-o <output>] [options]
```

| Option | Description |
| --- | --- |
| `-o`, `--output <file>` | Choose the output file. A `.rs` extension writes Rust source; other extensions build an executable. |
| `-r`, `--run` | Run the executable after compiling it. |
| `-O`, `--release` | Compile with optimizations. |
| `-S`, `--emit-rust` | Transpile only; print Rust to stdout or write it to `-o`. |
| `-h`, `--help` | Show help. |
| `-V`, `--version` | Show the version. |

Examples:

```powershell
objrust examples/animals.orst --run
objrust examples/animals.orst --emit-rust --output animals.rs
objrust examples/animals.orst --output animals.exe
```

You can also run the CLI from a local checkout without installing it:

```powershell
cargo run -- examples/animals.orst --run
```

## Language features

- `class` declarations are translated to Rust structs and `impl` blocks.
- `extends` supports the inheritance syntax implemented by the transpiler.
- `virtual` and `override` generate trait-based dynamic dispatch.
- Attributes and ordinary Rust items can be used in `.orst` source.

See [`examples/animals.orst`](examples/animals.orst) for a fuller example.

## Using third-party crates

The current CLI invokes `rustc` directly to compile the generated Rust. It does not resolve dependencies from a consumer project's `Cargo.toml`. To use external crates, generate Rust with `--emit-rust` and integrate that source into a Cargo project that declares the dependencies. Cargo-based project builds are a possible future improvement.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE](LICENSE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
