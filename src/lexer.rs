//! lexer.rs — Stage 1: turn source text into a flat list of tokens.
//!
//! The lexer does NOT try to understand Rust. It only needs to be accurate
//! enough that the parser can trust two things:
//!
//!   1. A `{` or `}` token is a real brace, never part of a string, char
//!      literal or comment.
//!   2. Joining all token texts back together gives the original source
//!      exactly (whitespace and comments are kept as `Trivia` tokens), so
//!      ordinary Rust code passes through the transpiler untouched.
//!
//! Things it handles on purpose, because they are the classic traps:
//!   * raw strings:            r"..."   r#"..."#   br#"..."#
//!   * byte and normal strings with escapes: "a\"b"   b"bytes"
//!   * char literals vs. lifetimes:          'a'   '\n'   'static
//!   * nested block comments:  /* outer /* inner */ still a comment */
//!
//! Unterminated strings, chars, raw strings and block comments are reported
//! as errors with the position where they started.

use crate::error::{Error, Pos, Result};

/// The coarse category of a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An identifier or keyword: `fn`, `class`, `Animal`, `_x`
    Ident,
    /// A single punctuation character: `{`, `:`, `<` ...
    Punct,
    /// A string, char, lifetime or number
    Literal,
    /// Whitespace and comments (kept so output can be reproduced exactly)
    Trivia,
}

/// One lexical token with its exact source text and start position.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Kind,
    pub text: String,
    pub pos: Pos,
}

impl Token {
    pub fn is_trivia(&self) -> bool {
        self.kind == Kind::Trivia
    }
}

/// Concatenate the text of tokens (used to print pass-through code and types).
pub fn join(tokens: &[Token]) -> String {
    tokens.iter().map(|t| t.text.as_str()).collect()
}

/// Split `src` into tokens.
pub fn lex(src: &str) -> Result<Vec<Token>> {
    Lexer { chars: src.chars().collect(), pos: 0, line: 1, col: 1 }.run()
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    // ---- driver -------------------------------------------------------

    fn run(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        while self.pos < self.chars.len() {
            let start = self.pos;
            let at = self.here();
            let kind = self.next_kind(at)?;
            tokens.push(Token {
                kind,
                text: self.chars[start..self.pos].iter().collect(),
                pos: at,
            });
        }
        Ok(tokens)
    }

    /// Consume exactly one token and say what kind it was.
    fn next_kind(&mut self, at: Pos) -> Result<Kind> {
        let ch = self.chars[self.pos];

        if ch.is_whitespace() {
            self.eat_while(char::is_whitespace);
            return Ok(Kind::Trivia);
        }
        if ch == '/' && self.peek(1) == Some('/') {
            self.eat_while(|c| c != '\n');
            return Ok(Kind::Trivia);
        }
        if ch == '/' && self.peek(1) == Some('*') {
            self.block_comment(at)?;
            return Ok(Kind::Trivia);
        }
        if let Some((open_len, hashes)) = self.raw_string_start() {
            self.raw_string(open_len, hashes, at)?;
            return Ok(Kind::Literal);
        }
        if ch == '"' || (ch == 'b' && self.peek(1) == Some('"')) {
            self.string(at)?;
            return Ok(Kind::Literal);
        }
        if ch == '\'' {
            self.char_or_lifetime(at)?;
            return Ok(Kind::Literal);
        }
        if is_ident_start(ch) {
            self.eat_while(is_ident_continue);
            return Ok(Kind::Ident);
        }
        if ch.is_ascii_digit() {
            self.eat_while(is_ident_continue); // 42, 1_000u32, 0xFF
            return Ok(Kind::Literal);
        }
        self.bump();
        Ok(Kind::Punct)
    }

    // ---- low-level helpers -------------------------------------------

    fn here(&self) -> Pos {
        Pos::new(self.line, self.col)
    }

    fn peek(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    /// Consume one char, keeping line/column up to date.
    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn eat_while(&mut self, pred: impl Fn(char) -> bool) {
        while let Some(c) = self.peek(0) {
            if !pred(c) {
                break;
            }
            self.bump();
        }
    }

    // ---- literals and comments ---------------------------------------

    /// `/* ... */`, which may nest.
    fn block_comment(&mut self, at: Pos) -> Result<()> {
        let mut depth = 0usize;
        loop {
            match (self.peek(0), self.peek(1)) {
                (Some('/'), Some('*')) => {
                    self.bump();
                    self.bump();
                    depth += 1;
                }
                (Some('*'), Some('/')) => {
                    self.bump();
                    self.bump();
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                (Some(_), _) => {
                    self.bump();
                }
                (None, _) => return Err(Error::new("unterminated block comment", at)),
            }
        }
    }

    /// If a raw string starts here, return (length of the opening such as
    /// `r#"`, number of `#`).
    fn raw_string_start(&self) -> Option<(usize, usize)> {
        let mut n = 0;
        if self.peek(0) == Some('b') {
            n += 1;
        }
        if self.peek(n) != Some('r') {
            return None;
        }
        n += 1;
        let mut hashes = 0;
        while self.peek(n + hashes) == Some('#') {
            hashes += 1;
        }
        if self.peek(n + hashes) == Some('"') {
            Some((n + hashes + 1, hashes))
        } else {
            None
        }
    }

    fn raw_string(&mut self, open_len: usize, hashes: usize, at: Pos) -> Result<()> {
        for _ in 0..open_len {
            self.bump();
        }
        loop {
            match self.bump() {
                None => return Err(Error::new("unterminated raw string", at)),
                Some('"') if (0..hashes).all(|k| self.peek(k) == Some('#')) => {
                    for _ in 0..hashes {
                        self.bump();
                    }
                    return Ok(());
                }
                Some(_) => {}
            }
        }
    }

    /// `"..."` or `b"..."`, honouring backslash escapes.
    fn string(&mut self, at: Pos) -> Result<()> {
        if self.peek(0) == Some('b') {
            self.bump();
        }
        self.bump(); // opening quote
        loop {
            match self.bump() {
                None => return Err(Error::new("unterminated string literal", at)),
                Some('\\') => {
                    self.bump();
                }
                Some('"') => return Ok(()),
                Some(_) => {}
            }
        }
    }

    /// `'a'` / `'\n'` are chars, `'a` / `'static` are lifetimes.
    fn char_or_lifetime(&mut self, at: Pos) -> Result<()> {
        let is_char = self.peek(1) == Some('\\') || self.peek(2) == Some('\'');
        self.bump(); // the opening `'`
        if !is_char {
            self.eat_while(is_ident_continue);
            return Ok(());
        }
        loop {
            match self.bump() {
                None => return Err(Error::new("unterminated character literal", at)),
                Some('\\') => {
                    self.bump();
                }
                Some('\'') => return Ok(()),
                Some(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<(Kind, String)> {
        lex(src)
            .unwrap()
            .into_iter()
            .filter(|t| !t.is_trivia())
            .map(|t| (t.kind, t.text))
            .collect()
    }

    #[test]
    fn joining_tokens_reproduces_the_source() {
        let src = "fn main() { let s = r#\"a \"} b\"#; /* x /* y */ z */ 'a' 'b }\n";
        assert_eq!(join(&lex(src).unwrap()), src);
    }

    #[test]
    fn braces_inside_strings_and_chars_are_not_punctuation() {
        let toks = kinds("\"{\" '{' r#\"}\"# x");
        assert!(toks.iter().all(|(k, _)| *k != Kind::Punct));
    }

    #[test]
    fn lifetimes_are_not_chars() {
        let toks = kinds("&'a str 'x'");
        assert!(toks.contains(&(Kind::Literal, "'a".to_string())));
        assert!(toks.contains(&(Kind::Literal, "'x'".to_string())));
    }

    #[test]
    fn positions_are_tracked() {
        let toks = lex("a\n  b").unwrap();
        let b = toks.iter().find(|t| t.text == "b").unwrap();
        assert_eq!(b.pos, Pos::new(2, 3));
    }

    #[test]
    fn unterminated_things_are_errors() {
        assert!(lex("\"abc").is_err());
        assert!(lex("/* abc").is_err());
        assert!(lex("r#\"abc\"").is_err());
    }
}
