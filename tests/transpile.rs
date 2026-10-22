//! tests/transpile.rs — End-to-end tests: Objective-Rust text in, Rust text out.
//!
//! Run with `cargo test`. These check the behaviour users care about:
//! ordinary Rust is untouched, classes are rewritten correctly, and mistakes
//! produce errors with useful positions.

use objective_rust::transpile;

#[test]
fn plain_rust_passes_through_unchanged() {
    let src = r##"
// a comment with class Foo { in it
fn main() {
    let class = 5;           // `class` as a variable name is fine
    let s = r#"class Bar { }"#;
    println!("{} {}", class, s);
}
"##;
    let out = transpile(src).unwrap();
    assert!(out.ends_with(src), "plain Rust must be copied byte for byte");
}

#[test]
fn simple_class_becomes_struct_and_impl() {
    let out = transpile(
        "pub class Point { pub x: i32, y: i32,\n pub fn new(x: i32, y: i32) -> Self { Self { x, y } } }",
    )
    .unwrap();
    assert!(out.contains("pub struct Point {"));
    assert!(out.contains("    pub x: i32,"));
    assert!(out.contains("    y: i32,"));
    assert!(out.contains("impl Point {"));
    assert!(out.contains("pub fn new(x: i32, y: i32) -> Self"));
}

#[test]
fn extends_adds_base_field_and_deref() {
    let out = transpile("class A { a: i32, } class B extends A { b: i32, }").unwrap();
    assert!(out.contains("    base: A,"));
    assert!(out.contains("impl std::ops::Deref for B"));
    assert!(out.contains("impl std::ops::DerefMut for B"));
}

#[test]
fn super_initialiser_becomes_base() {
    let out = transpile(
        "class A { a: i32, } class B extends A { fn new() -> Self { Self { super: A { a: 1 } } } }",
    )
    .unwrap();
    assert!(out.contains("Self { base: A { a: 1 } }"));
}

#[test]
fn super_paths_are_not_rewritten() {
    let out = transpile("class A { fn f(&self) { super::helper(); } }").unwrap();
    assert!(out.contains("super::helper()"));
}

#[test]
fn virtual_creates_a_trait_and_override_implements_it() {
    let out = transpile(
        "class A { virtual fn hi(&self) {} } class B extends A { override fn hi(&self) {} }",
    )
    .unwrap();
    assert!(out.contains("trait AVirtual {"));
    assert!(out.contains("fn hi(&self);"));
    assert!(out.contains("impl AVirtual for A {"));
    assert!(out.contains("impl AVirtual for B {"));
}

#[test]
fn override_resolves_through_a_chain_of_classes() {
    let out = transpile(
        "class A { virtual fn hi(&self) {} }
         class B extends A { override fn hi(&self) {} }
         class C extends B { override fn hi(&self) {} }",
    )
    .unwrap();
    // C overrides a method first declared virtual in A, not in B.
    assert!(out.contains("impl AVirtual for C {"));
    assert!(!out.contains("BVirtual"));
}

#[test]
fn braces_in_strings_do_not_confuse_the_parser() {
    let out = transpile(
        r##"class A { fn f(&self) -> String { let _c = '}'; format!("{{}}") + r#"}"# } }
        fn after() {}"##,
    )
    .unwrap();
    assert!(out.contains("fn after() {}"));
}

#[test]
fn generic_field_types_with_commas_are_kept_whole() {
    let out = transpile("class A { m: HashMap<String, Vec<u32>>, f: fn(i32) -> i32, n: u8, }").unwrap();
    assert!(out.contains("    m: HashMap<String, Vec<u32>>,"));
    assert!(out.contains("    f: fn(i32) -> i32,"));
    assert!(out.contains("    n: u8,"));
}

// ---- errors ----------------------------------------------------------

#[test]
fn override_without_virtual_ancestor_is_an_error() {
    let err = transpile("class A {} class B extends A { override fn f(&self) {} }").unwrap_err();
    assert!(err.message.contains("no ancestor declares"));
}

#[test]
fn override_without_extends_is_an_error() {
    let err = transpile("class A { override fn f(&self) {} }").unwrap_err();
    assert!(err.message.contains("no `extends`"));
}

#[test]
fn generic_classes_give_a_clear_error() {
    let err = transpile("class Stack<T> { items: Vec<T>, }").unwrap_err();
    assert!(err.message.contains("generic classes"));
}

#[test]
fn errors_carry_line_and_column() {
    let err = transpile("fn ok() {}\n\nclass A {\n    age u32,\n}").unwrap_err();
    assert_eq!(err.pos.line, 4);
}

#[test]
fn unclosed_class_is_reported_at_its_start() {
    let err = transpile("\nclass A {\n    x: i32,\n").unwrap_err();
    assert!(err.message.contains("never closed"));
    assert_eq!(err.pos.line, 2);
}
