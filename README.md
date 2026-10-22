# Objective-Rust

[![Crates.io version](https://img.shields.io/crates/v/objrust-compiler)](https://crates.io/crates/objrust-compiler)
[![License](https://img.shields.io/crates/l/objrust-compiler)](LICENSE)

I made Objective-Rust to experiment with writing Rust using classes. It lets you add class-style declarations to `.orst` files, then translates them into regular Rust. The `objrust` command can compile and run the generated code.

## Install

You’ll need Rust installed, including Cargo and `rustc`. If you don’t have them yet, [![Install Rust](https://img.shields.io/badge/Install-Rust%20via%20rustup-orange?logo=rust&logoColor=white)](https://rustup.rs)

Install ObjRust from crates.io:

```powershell
cargo install objrust-compiler
```

This installs the `objrust` command. To install directly from the GitHub repository instead:

```powershell
cargo install --git https://github.com/amrhasanain/objrust.git objrust-compiler
```

For `.orst` syntax highlighting and file icons in VS Code, install the [ObjRust extension](https://marketplace.visualstudio.com/items?itemName=amrhasanain.objectiverust).

## Try it

Save this as `hello.orst`:

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

Build and run it:

```powershell
objrust hello.orst --run
```

ObjRust keeps the generated Rust source in `objrust-output/`, so you can inspect the translation. The executable is named after the input file by default.

## Useful commands

```powershell
# Build and run
objrust hello.orst --run

# Write the generated Rust to a file
objrust hello.orst --emit-rust --output hello.rs

# Build an executable with a chosen name
objrust hello.orst --output hello.exe
```

Run `objrust --help` to see all options:

- `-o`, `--output`: choose the output file.
- `-r`, `--run`: run the program after building it.
- `-O`, `--release`: compile with optimizations.
- `-S`, `--emit-rust`: translate to Rust without compiling an executable.

If you’ve cloned this repository, you can run the larger example with:

```powershell
cargo run -- examples/animals.orst --run
```

## What ObjRust supports

ObjRust translates `class` declarations into Rust structs and `impl` blocks. It also supports `extends`, `virtual`, and `override`; [`examples/animals.orst`](examples/animals.orst) shows them in use. Regular Rust code can appear in the same `.orst` file.

The language is still evolving. If you run into a problem, please open an issue and include a small `.orst` file that reproduces it.

## Current limitation

When the CLI builds an executable, it calls `rustc` directly. It doesn’t resolve dependencies from a project's `Cargo.toml`. If your code uses another crate, emit the Rust with `--emit-rust` and add that source to a Cargo project that declares the dependency.

You can also call the transpiler as a Rust library:

```rust
fn main() {
    let rust_source = objective_rust::transpile("class Animal { pub name: String, }")
        .expect("valid ObjRust source");
    println!("{rust_source}");
}
```

## License

Licensed under either the Apache License 2.0 ([LICENSE](LICENSE)) or the MIT license ([LICENSE-MIT](LICENSE-MIT)), at your option.
