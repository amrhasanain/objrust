# Objective-Rust

I made Objective-Rust because I wanted to try writing Rust with classes. It lets me put class-style declarations in `.orst` files, then translates them into regular Rust. The `objrust` command can also compile and run the result.

## Give it a try

You’ll need Rust installed first, including Cargo and `rustc`. If you don’t have them yet, install Rust with [rustup](https://rustup.rs).

Once Objective-Rust is published on crates.io, install it with:

```powershell
cargo install objrust-compiler
```

Or install the version from GitHub:

```powershell
cargo install --git https://github.com/amrhasanain/objrust.git objrust-compiler
```

Create a file called `hello.orst`:

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

Run it from the folder where you saved the file:

```powershell
objrust hello.orst --run
```

That builds and runs the program. ObjRust also saves the Rust it generated in `objrust-output/`, so you can open it and see how the class was translated.

## A few useful commands

```powershell
# Build and run
objrust hello.orst --run

# Save the generated Rust without compiling it
objrust hello.orst --emit-rust --output hello.rs

# Build an executable with a specific name
objrust hello.orst --output hello.exe
```

Use `objrust --help` to see all the options. The short version:

- `-o` or `--output` chooses the output file.
- `-r` or `--run` runs the program after building it.
- `-O` or `--release` asks `rustc` to optimize the build.
- `-S` or `--emit-rust` outputs Rust without building an executable.

If you’ve cloned this repository, you can try the larger example directly:

```powershell
cargo run -- examples/animals.orst --run
```

## What works so far

ObjRust translates `class` declarations into Rust structs and `impl` blocks. The transpiler also supports the `extends`, `virtual`, and `override` syntax shown in [`examples/animals.orst`](examples/animals.orst). Regular Rust code can live in the same file.

This is an early project, so the syntax and compiler are still evolving. If you try it and hit a problem, please open an issue with a small `.orst` example that reproduces it.

## One current limitation

When it builds an executable, the CLI calls `rustc` directly. That means it doesn’t read dependencies from your project’s `Cargo.toml`. If your program uses another crate, generate the Rust with `--emit-rust` and use it in a Cargo project that declares the dependency.

You can also use the transpiler from Rust code:

```rust
let rust_source = objective_rust::transpile("class Animal { pub name: String, }")
    .expect("valid ObjRust source");
```

## License

You can use this project under either the Apache License 2.0 ([LICENSE](LICENSE)) or the MIT license ([LICENSE-MIT](LICENSE-MIT)).
