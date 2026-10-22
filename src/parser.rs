//! parser.rs — Stage 3: turn tokens into `Item`s (see ast.rs).
//!
//! This is NOT a Rust parser. It walks the token list at the top level of
//! the file and does only two things:
//!
//!   * If it sees `[pub] class Name`, it parses the class (below).
//!   * Otherwise it copies the token into a `Raw` run, untouched.
//!
//! Rules that keep this safe:
//!   * `class` is only recognised at brace depth 0, so `let class = 5;`
//!     inside a function (or a field called `class` in a struct) still works.
//!   * Inside a class we only parse the *shape*: members are fields
//!     (`name: Type,`) or methods (`fn name(...) { ... }`). Types, signatures
//!     and bodies are collected as raw tokens by counting brackets/braces.
//!
//! Supported grammar:
//!
//! ```text
//! class      := [pub] "class" Ident [ "extends" Ident ] "{" member* "}"
//! member     := field | method
//! field      := [pub] Ident ":" <type tokens> ("," | before "}")
//! method     := [pub] [virtual | override] "fn" Ident <signature> <block>
//! ```
//!
//! Not supported yet (reported as clear errors): generic classes, abstract
//! methods without a body, and associated consts/types inside a class.

use crate::ast::*;
use crate::error::{Error, Pos, Result};
use crate::lexer::{Kind, Token};

/// Parse a whole file's tokens into top-level items.
pub fn parse(tokens: Vec<Token>) -> Result<Vec<Item>> {
    Parser { toks: tokens, pos: 0 }.parse_file()
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    // ---- token helpers -------------------------------------------------

    /// Is token `i` exactly `text`?
    fn is(&self, i: usize, text: &str) -> bool {
        self.toks.get(i).map_or(false, |t| t.text == text)
    }

    /// Index of the first non-trivia token at or after `from`.
    fn peek_sig(&self, from: usize) -> Option<usize> {
        (from..self.toks.len()).find(|&i| !self.toks[i].is_trivia())
    }

    fn skip_trivia(&mut self) {
        while self.pos < self.toks.len() && self.toks[self.pos].is_trivia() {
            self.pos += 1;
        }
    }

    /// Position of the current token (or the last one at end of file).
    fn here(&self) -> Pos {
        self.toks
            .get(self.pos)
            .or(self.toks.last())
            .map_or(Pos::new(1, 1), |t| t.pos)
    }

    /// Human-readable description of the current token, for error messages.
    fn found(&self) -> String {
        self.toks
            .get(self.pos)
            .map_or("end of file".to_string(), |t| format!("`{}`", t.text))
    }

    /// Consume `text` if it comes next (skipping whitespace/comments).
    fn try_eat(&mut self, text: &str) -> bool {
        self.skip_trivia();
        if self.is(self.pos, text) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, text: &str) -> Result<()> {
        if self.try_eat(text) {
            Ok(())
        } else {
            Err(Error::new(format!("expected `{}`, found {}", text, self.found()), self.here()))
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        self.skip_trivia();
        match self.toks.get(self.pos) {
            Some(t) if t.kind == Kind::Ident => {
                let name = t.text.clone();
                self.pos += 1;
                Ok(name)
            }
            _ => Err(Error::new(format!("expected an identifier, found {}", self.found()), self.here())),
        }
    }

    // ---- top level -----------------------------------------------------

    /// Is there a `[pub] class Name` at the current position?
    /// Returns `Some(is_pub)` if so.
    fn class_start(&self) -> Option<bool> {
        let (is_pub, class_idx) = if self.is(self.pos, "pub") {
            (true, self.peek_sig(self.pos + 1)?)
        } else {
            (false, self.pos)
        };
        if !self.is(class_idx, "class") {
            return None;
        }
        let name_idx = self.peek_sig(class_idx + 1)?;
        if self.toks[name_idx].kind == Kind::Ident {
            Some(is_pub)
        } else {
            None
        }
    }

    fn parse_file(&mut self) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        let mut raw: Vec<Token> = Vec::new();
        let mut depth = 0i32; // brace depth of the plain Rust we are copying

        while self.pos < self.toks.len() {
            if depth == 0 {
                if let Some(is_pub) = self.class_start() {
                    if !raw.is_empty() {
                        items.push(Item::Raw(std::mem::take(&mut raw)));
                    }
                    items.push(Item::Class(self.parse_class(is_pub)?));
                    continue;
                }
            }
            let t = self.toks[self.pos].clone();
            match t.text.as_str() {
                "{" => depth += 1,
                "}" => depth -= 1,
                _ => {}
            }
            raw.push(t);
            self.pos += 1;
        }
        if !raw.is_empty() {
            items.push(Item::Raw(raw));
        }
        Ok(items)
    }

    // ---- class ---------------------------------------------------------

    fn parse_class(&mut self, is_pub: bool) -> Result<ClassDecl> {
        let pos = self.here();
        if is_pub {
            self.expect("pub")?;
        }
        self.expect("class")?;
        let name = self.expect_ident()?;

        self.skip_trivia();
        if self.is(self.pos, "<") {
            return Err(Error::new(
                format!("generic classes are not supported yet (class `{}`)", name),
                self.here(),
            ));
        }

        let parent = if self.try_eat("extends") { Some(self.expect_ident()?) } else { None };
        self.expect("{")?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        loop {
            self.skip_trivia();
            if self.pos >= self.toks.len() {
                return Err(Error::new(format!("class `{}` is never closed (missing `}}`)", name), pos));
            }
            if self.try_eat("}") {
                break;
            }

            let member_pos = self.here();
            let m_pub = self.try_eat("pub");
            let kind = if self.try_eat("virtual") {
                MethodKind::Virtual
            } else if self.try_eat("override") {
                MethodKind::Override
            } else {
                MethodKind::Normal
            };

            if self.try_eat("fn") {
                methods.push(self.parse_method(member_pos, m_pub, kind)?);
            } else if kind != MethodKind::Normal {
                return Err(Error::new("`virtual` and `override` can only be used on methods", member_pos));
            } else {
                fields.push(self.parse_field(m_pub)?);
            }
        }

        Ok(ClassDecl { name, parent, is_pub, fields, methods, pos })
    }

    /// `name: Type,`
    fn parse_field(&mut self, is_pub: bool) -> Result<Field> {
        let name = self.expect_ident()?;
        if !self.try_eat(":") {
            return Err(Error::new(
                format!(
                    "expected `:` after `{}` (class members are fields `name: Type,` or methods `fn ...`), found {}",
                    name,
                    self.found()
                ),
                self.here(),
            ));
        }
        self.skip_trivia();
        let ty = self.field_type()?;
        Ok(Field { name, is_pub, ty })
    }

    /// Collect the tokens of a type up to the next top-level `,` (consumed)
    /// or the class's closing `}` (not consumed). Angle brackets are counted
    /// so `HashMap<String, u32>` is not split at its inner comma.
    fn field_type(&mut self) -> Result<Vec<Token>> {
        let mut out: Vec<Token> = Vec::new();
        let mut depth = 0i32;

        while let Some(t) = self.toks.get(self.pos).cloned() {
            // `->` contains a `>` that is not a closing angle bracket
            let is_arrow = t.text == ">" && out.last().map_or(false, |p| p.text == "-");
            match t.text.as_str() {
                "(" | "[" | "<" => depth += 1,
                ")" | "]" => depth -= 1,
                ">" if !is_arrow => depth -= 1,
                "}" if depth == 0 => break,
                "," if depth == 0 => {
                    self.pos += 1;
                    break;
                }
                _ => {}
            }
            out.push(t);
            self.pos += 1;
        }

        while out.last().map_or(false, |t| t.is_trivia()) {
            out.pop();
        }
        if out.is_empty() {
            return Err(Error::new("missing field type", self.here()));
        }
        Ok(out)
    }

    // ---- methods -------------------------------------------------------

    /// After `fn`: `name <signature> { body }`
    fn parse_method(&mut self, pos: Pos, is_pub: bool, kind: MethodKind) -> Result<Method> {
        let name = self.expect_ident()?;

        // Signature: everything up to the `{` that opens the body.
        let mut sig = Vec::new();
        let mut depth = 0i32;
        loop {
            let Some(t) = self.toks.get(self.pos).cloned() else {
                return Err(Error::new(
                    format!("unexpected end of file in the signature of `{}`", name),
                    self.here(),
                ));
            };
            match t.text.as_str() {
                "(" | "[" => depth += 1,
                ")" | "]" => depth -= 1,
                "{" if depth == 0 => break,
                ";" if depth == 0 => {
                    return Err(Error::new(
                        format!("method `{}` needs a body (abstract methods are not supported yet)", name),
                        t.pos,
                    ));
                }
                _ => {}
            }
            sig.push(t);
            self.pos += 1;
        }

        let body = self.balanced_block()?;
        Ok(Method { name, is_pub, kind, sig, body, pos })
    }

    /// From a `{` through its matching `}` (inclusive).
    fn balanced_block(&mut self) -> Result<Vec<Token>> {
        let open = self.here();
        let mut out = Vec::new();
        let mut depth = 0i32;
        while let Some(t) = self.toks.get(self.pos).cloned() {
            self.pos += 1;
            match t.text.as_str() {
                "{" => depth += 1,
                "}" => depth -= 1,
                _ => {}
            }
            out.push(t);
            if depth == 0 {
                return Ok(out);
            }
        }
        Err(Error::new("this `{` is never closed", open))
    }
}
