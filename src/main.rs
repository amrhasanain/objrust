//! main.rs — Command-line front end for the transpiler (binary name: `objrust`).
//!
//! `objrust` works like a normal compiler driver: it transpiles the
//! Objective-Rust file to Rust, then calls `rustc` for you.
//!
//! Usage:
//!   objrust animal.orst -o animal.exe   transpile + compile to an executable
//!   objrust animal.orst                 same, output named after the input
//!   objrust animal.orst -r              compile, then run it
//!   objrust animal.orst -o animal.rs    only transpile (output ends in .rs)
//!   objrust animal.orst -S              only transpile, print Rust to stdout
//!
//! Generated Rust: when building an executable, the Rust produced by the
//! transpiler is saved as `objrust-output/<name>.rs` (the folder is created
//! in the current directory) and is always kept. rustc's error messages
//! refer to that file, so you can open it and read exactly what was compiled.
//!
//! This file only handles arguments, files and running programs. All real
//! work happens in the library (see lib.rs).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, process};

const USAGE: &str = "\
objrust — Objective-Rust compiler driver

USAGE:
    objrust <input.orst> [-o <output>] [options]

If <output> ends in .rs, only the Rust source is written.
Otherwise the program is compiled with rustc into an executable, and the
generated Rust is saved in ./objrust-output/<name>.rs
Without -o, the executable is named after the input file.

OPTIONS:
    -o, --output <file>   output file
    -r, --run             run the program after building it
    -O, --release         build with optimizations (rustc -C opt-level=3)
    -S, --emit-rust       only transpile; print Rust to stdout (or to -o)
    -h, --help            show this help
    -V, --version         show the version";

/// Print `error: message` and exit with status 1.
fn fail(message: &str) -> ! {
    eprintln!("error: {}", message);
    process::exit(1);
}

struct Options {
    input: String,
    output: Option<String>,
    run: bool,
    release: bool,
    emit_rust: bool,
}

fn parse_args() -> Options {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let (mut run, mut release, mut emit_rust) = (false, false, false);

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{}", USAGE);
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("objrust {}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            "-o" | "--output" => match args.next() {
                Some(path) => output = Some(path),
                None => fail("`-o` needs a file name"),
            },
            "-r" | "--run" => run = true,
            "-O" | "--release" => release = true,
            "-S" | "--emit-rust" => emit_rust = true,
            _ if arg.starts_with('-') => fail(&format!("unknown option `{}` (try --help)", arg)),
            _ => {
                if input.replace(arg).is_some() {
                    fail("only one input file is allowed");
                }
            }
        }
    }

    let Some(input) = input else {
        fail("no input file given (try --help)");
    };
    Options { input, output, run, release, emit_rust }
}

/// `animal.orst` -> `animal.exe` on Windows, `animal` elsewhere.
fn default_output(input: &str) -> PathBuf {
    let stem = Path::new(input).file_stem().and_then(|s| s.to_str()).unwrap_or("a");
    PathBuf::from(format!("{}{}", stem, env::consts::EXE_SUFFIX))
}

/// Folder (in the current directory) where generated Rust files are kept.
const OUTPUT_DIR: &str = "objrust-output";

/// Save the generated Rust as `objrust-output/<name>.rs` and compile it
/// with rustc into `out`.
fn build_executable(rust: &str, input: &str, out: &Path, release: bool) {
    let stem = Path::new(input).file_stem().and_then(|s| s.to_str()).unwrap_or("out");

    fs::create_dir_all(OUTPUT_DIR)
        .unwrap_or_else(|e| fail(&format!("cannot create folder `{}`: {}", OUTPUT_DIR, e)));
    let rs_path = Path::new(OUTPUT_DIR).join(format!("{}.rs", stem));
    fs::write(&rs_path, rust)
        .unwrap_or_else(|e| fail(&format!("cannot write `{}`: {}", rs_path.display(), e)));

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg(&rs_path).arg("-o").arg(out);
    if release {
        cmd.arg("-C").arg("opt-level=3");
    }

    match cmd.status() {
        Ok(status) if status.success() => {}
        Ok(_) => {
            eprintln!("error: rustc failed; see the generated Rust in {}", rs_path.display());
            process::exit(1);
        }
        Err(e) => fail(&format!("cannot run `rustc` ({}). Is Rust installed and on your PATH?", e)),
    }
}

fn main() {
    let opts = parse_args();

    let source = fs::read_to_string(&opts.input)
        .unwrap_or_else(|e| fail(&format!("cannot read `{}`: {}", opts.input, e)));

    // Errors print as `file.orst:line:col: message`.
    let rust = objective_rust::transpile(&source)
        .unwrap_or_else(|e| fail(&format!("{}:{}", opts.input, e)));

    let output = opts.output.as_deref().map(PathBuf::from);
    let wants_rust_only = opts.emit_rust
        || output.as_ref().map_or(false, |p| p.extension().map_or(false, |e| e == "rs"));

    // Mode 1: only transpile.
    if wants_rust_only {
        match output {
            Some(path) => fs::write(&path, rust)
                .unwrap_or_else(|e| fail(&format!("cannot write `{}`: {}", path.display(), e))),
            None => print!("{}", rust),
        }
        return;
    }

    // Mode 2: transpile + compile (+ run).
    let out = output.unwrap_or_else(|| default_output(&opts.input));
    build_executable(&rust, &opts.input, &out, opts.release);

    if opts.run {
        let exe = fs::canonicalize(&out).unwrap_or(out);
        match Command::new(&exe).status() {
            Ok(status) => process::exit(status.code().unwrap_or(0)),
            Err(e) => fail(&format!("cannot run `{}`: {}", exe.display(), e)),
        }
    }
}