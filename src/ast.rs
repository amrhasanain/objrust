//! ast.rs — Stage 2 data model: what the parser produces.
//!
//! The AST is deliberately tiny. Objective-Rust only adds ONE new construct
//! (`class`), so that is the only thing modelled. Everything else in a file
//! is ordinary Rust that we never need to understand, so it is kept as a raw
//! run of tokens (`Item::Raw`) and copied to the output unchanged.
//!
//! The same trick is used inside a class: field types, method signatures
//! and method bodies are stored as token lists, not parsed. The transpiler
//! only has to know WHERE they start and end, not what they mean.
//!
//! Source language (example):
//!
//! ```text
//! pub class Dog extends Animal {
//!     breed: String,
//!     pub override fn speak(&self) { ... }
//! }
//! ```

use crate::error::Pos;
use crate::lexer::Token;

/// One top-level piece of an `.orst` file.
#[derive(Debug)]
pub enum Item {
    /// Plain Rust (functions, structs, `use` lines, comments, whitespace...)
    /// copied to the output exactly as written.
    Raw(Vec<Token>),
    /// A `class` declaration that must be rewritten.
    Class(ClassDecl),
}

/// `[pub] class Name [extends Parent] { fields and methods }`
#[derive(Debug)]
pub struct ClassDecl {
    pub name: String,
    /// The single parent class named after `extends`, if any.
    pub parent: Option<String>,
    pub is_pub: bool,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    /// Where the declaration starts (used in errors and output comments).
    pub pos: Pos,
}

/// `[pub] name: Type,`
#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub is_pub: bool,
    /// The type, as unparsed tokens (e.g. `Vec<String>`).
    pub ty: Vec<Token>,
}

/// How a method takes part in dynamic dispatch.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MethodKind {
    /// No keyword: a normal method, resolved at compile time.
    Normal,
    /// `virtual`: introduces a method that subclasses may override.
    Virtual,
    /// `override`: replaces a `virtual` method from an ancestor class.
    Override,
}

/// `[pub] [virtual|override] fn name(<signature>) { <body> }`
#[derive(Debug)]
pub struct Method {
    pub name: String,
    pub is_pub: bool,
    pub kind: MethodKind,
    /// Everything between the name and the body, e.g. `(&self, x: i32) -> i32 `.
    pub sig: Vec<Token>,
    /// The body including its braces: `{ ... }`.
    pub body: Vec<Token>,
    pub pos: Pos,
}
