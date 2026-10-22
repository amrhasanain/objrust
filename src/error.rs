//! error.rs — Source positions and the error type shared by every stage.
//!
//! Every stage of the transpiler (lexer, parser, code generator) reports
//! problems with the same `Error` type, so the command line tool can print
//! one consistent message:
//!
//! ```text
//! animals.orst:12:5: expected `:` after field name `age`, found `}`
//! ```
//!
//! `Pos` is a 1-based line/column pair. It is attached to every token and to
//! every class and method in the AST, which is what makes good messages
//! possible.

use std::fmt;

/// A 1-based line and column in the `.orst` source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub fn new(line: usize, col: usize) -> Self {
        Pos { line, col }
    }
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// An error produced while transpiling, together with where it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub pos: Pos,
}

impl Error {
    pub fn new(message: impl Into<String>, pos: Pos) -> Self {
        Error { message: message.into(), pos }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.pos, self.message)
    }
}

impl std::error::Error for Error {}

/// Shorthand used by all stages.
pub type Result<T> = std::result::Result<T, Error>;
