//! codegen.rs — Stage 4: turn the AST back into plain Rust source text.
//!
//! Raw items are printed exactly as they were read. Each `class` is
//! rewritten ("desugared") into ordinary Rust, so the real compiler, the
//! borrow checker and every crate on crates.io work without any changes.
//!
//! Translation rules:
//!
//! | Objective-Rust            | Generated Rust                                   |
//! |---------------------------|--------------------------------------------------|
//! | `class A { x: T }`        | `struct A { x: T }`                              |
//! | normal methods            | `impl A { ... }` (static dispatch)               |
//! | `extends B`               | field `base: B` + `Deref`/`DerefMut` to `B`      |
//! | `super: expr` in `Self {}`| `base: expr`                                     |
//! | `virtual fn f`            | `trait AVirtual { fn f(..); }` + `impl AVirtual for A` |
//! | `override fn f`           | `impl XVirtual for A` where `X` is the nearest   |
//! |                           | ancestor that declared `f` as `virtual`          |
//!
//! Because virtual methods become a trait, polymorphism works the Rust way:
//! `Box<dyn AVirtual>` / `&dyn AVirtual` can hold an `A` or any subclass.
//!
//! Each generated class is preceded by a `// orst: ...` comment that points
//! back to the line in the `.orst` source, which helps when reading compiler
//! errors that refer to the generated file.

use std::collections::HashMap;

use crate::ast::*;
use crate::error::{Error, Result};
use crate::lexer::join;

/// Lookup of every class declared in the file, by name.
type ClassTable<'a> = HashMap<&'a str, &'a ClassDecl>;

/// Generate Rust source for a whole file.
pub fn generate(items: &[Item]) -> Result<String> {
    let table: ClassTable = items
        .iter()
        .filter_map(|item| match item {
            Item::Class(c) => Some((c.name.as_str(), c)),
            Item::Raw(_) => None,
        })
        .collect();

    let mut out = String::new();
    for item in items {
        match item {
            Item::Raw(tokens) => out.push_str(&join(tokens)),
            Item::Class(c) => out.push_str(&gen_class(c, &table)?),
        }
    }
    Ok(out)
}

fn gen_class(c: &ClassDecl, table: &ClassTable) -> Result<String> {
    let vis = if c.is_pub { "pub " } else { "" };
    let mut o = format!("// orst: class `{}` (source line {})\n", c.name, c.pos.line);

    // 1) struct — inheritance is composition: the parent lives in `base`.
    o += &format!("{}struct {} {{\n", vis, c.name);
    if let Some(parent) = &c.parent {
        o += &format!("    base: {},\n", parent);
    }
    for f in &c.fields {
        let fvis = if f.is_pub { "pub " } else { "" };
        o += &format!("    {}{}: {},\n", fvis, f.name, join(&f.ty));
    }
    o += "}\n\n";

    let by_kind = |k: MethodKind| -> Vec<&Method> { c.methods.iter().filter(|m| m.kind == k).collect() };
    let normal = by_kind(MethodKind::Normal);
    let virtuals = by_kind(MethodKind::Virtual);
    let overrides = by_kind(MethodKind::Override);

    // 2) normal methods -> inherent impl
    if !normal.is_empty() {
        o += &format!("impl {} {{\n", c.name);
        for m in &normal {
            o += &gen_method(m, true);
        }
        o += "}\n\n";
    }

    // 3) virtual methods -> a new trait named <Class>Virtual
    if !virtuals.is_empty() {
        o += &format!("{}trait {}Virtual {{\n", vis, c.name);
        for m in &virtuals {
            o += &format!("    fn {}{};\n", m.name, join(&m.sig).trim_end());
        }
        o += "}\n";
        o += &format!("impl {0}Virtual for {0} {{\n", c.name);
        for m in &virtuals {
            o += &gen_method(m, false);
        }
        o += "}\n\n";
    }

    // 4) overrides -> impl of the ancestor's trait. Overrides may belong to
    //    different ancestors, so group them by the trait that owns them.
    let mut groups: Vec<(String, Vec<&Method>)> = Vec::new();
    for m in overrides {
        let owner = virtual_owner(c, m, table)?;
        match groups.iter_mut().find(|(name, _)| *name == owner) {
            Some((_, list)) => list.push(m),
            None => groups.push((owner, vec![m])),
        }
    }
    for (owner, list) in groups {
        o += &format!("impl {}Virtual for {} {{\n", owner, c.name);
        for m in list {
            o += &gen_method(m, false);
        }
        o += "}\n\n";
    }

    // 5) extends -> Deref/DerefMut so parent fields and methods are reachable
    if let Some(parent) = &c.parent {
        o += &format!(
            "impl std::ops::Deref for {0} {{\n    type Target = {1};\n    fn deref(&self) -> &{1} {{ &self.base }}\n}}\n",
            c.name, parent
        );
        o += &format!(
            "impl std::ops::DerefMut for {0} {{\n    fn deref_mut(&mut self) -> &mut {1} {{ &mut self.base }}\n}}\n\n",
            c.name, parent
        );
    }
    Ok(o)
}

/// Find which ancestor first declared `method` as `virtual`. The generated
/// trait for that ancestor is the one the override must implement.
fn virtual_owner<'a>(c: &'a ClassDecl, method: &Method, table: &ClassTable<'a>) -> Result<String> {
    let Some(first_parent) = &c.parent else {
        return Err(Error::new(
            format!("`override fn {}` in class `{}`, which has no `extends`", method.name, c.name),
            method.pos,
        ));
    };

    let mut visited: Vec<&str> = vec![c.name.as_str()];
    let mut current: &str = first_parent.as_str();

    loop {
        if visited.contains(&current) {
            return Err(Error::new(format!("inheritance cycle involving class `{}`", current), c.pos));
        }
        visited.push(current);

        let Some(ancestor) = table.get(current) else {
            // Declared outside this file (another module): trust that it
            // follows the same naming scheme.
            return Ok(current.to_string());
        };
        if ancestor
            .methods
            .iter()
            .any(|m| m.kind == MethodKind::Virtual && m.name == method.name)
        {
            return Ok(ancestor.name.clone());
        }
        match &ancestor.parent {
            Some(p) => current = p.as_str(),
            None => {
                return Err(Error::new(
                    format!(
                        "`override fn {}` in class `{}`: no ancestor declares `virtual fn {}`",
                        method.name, c.name, method.name
                    ),
                    method.pos,
                ));
            }
        }
    }
}

/// One method as Rust text. `keep_pub` is false inside trait impls, where
/// Rust forbids a visibility qualifier.
fn gen_method(m: &Method, keep_pub: bool) -> String {
    let vis = if keep_pub && m.is_pub { "pub " } else { "" };
    format!("    {}fn {}{}{}\n", vis, m.name, join(&m.sig), rewrite_body(&m.body))
}

/// Copy a method body, rewriting `super: value` (a struct-literal field
/// initialiser) to `base: value`. `super::path` is left alone.
fn rewrite_body(tokens: &[crate::lexer::Token]) -> String {
    let mut s = String::new();
    for (i, t) in tokens.iter().enumerate() {
        let is_super_init = t.text == "super"
            && tokens.get(i + 1).map_or(false, |n| n.text == ":")
            && tokens.get(i + 2).map_or(true, |n| n.text != ":");
        s.push_str(if is_super_init { "base" } else { t.text.as_str() });
    }
    s
}
