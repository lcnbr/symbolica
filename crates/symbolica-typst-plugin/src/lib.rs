use std::str;

use symbolica::prelude::{Atom, AtomCore, AtomPrinter, Indeterminate, ParseSettings, PrintOptions};
use wasm_minimal_protocol::*;

initiate_protocol!();

fn parse_utf8<'a>(input: &'a [u8], label: &str) -> Result<&'a str, String> {
    str::from_utf8(input).map_err(|_| format!("{label} must be UTF-8"))
}

fn parse_atom(input: &[u8], label: &str) -> Result<Atom, String> {
    Atom::parse(parse_utf8(input, label)?, "typst", ParseSettings::default())
}

fn render(atom: &Atom, opts: PrintOptions) -> Vec<u8> {
    AtomPrinter::new_with_options(atom.as_view(), opts)
        .to_string()
        .into_bytes()
}

fn render_symbolica(atom: &Atom) -> Vec<u8> {
    atom.to_canonical_string().into_bytes()
}

#[wasm_func]
pub fn canonical(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = parse_atom(expr, "expr")?;
    Ok(render_symbolica(&expr))
}

#[wasm_func]
pub fn to_typst(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = parse_atom(expr, "expr")?;
    Ok(render(&expr, PrintOptions::typst()))
}

#[wasm_func]
pub fn to_latex(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = parse_atom(expr, "expr")?;
    Ok(render(&expr, PrintOptions::latex()))
}

#[wasm_func]
pub fn simplify_expr(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = parse_atom(expr, "expr")?;
    Ok(render_symbolica(&expr))
}

#[wasm_func]
pub fn expand(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = parse_atom(expr, "expr")?.expand();
    Ok(render_symbolica(&expr))
}

#[wasm_func]
pub fn factor(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = parse_atom(expr, "expr")?.factor();
    Ok(render_symbolica(&expr))
}

#[wasm_func]
pub fn derivative(expr: &[u8], var: &[u8]) -> Result<Vec<u8>, String> {
    let expr = parse_atom(expr, "expr")?;
    let var = Indeterminate::try_from(parse_atom(var, "var")?)?;
    Ok(render_symbolica(&expr.derivative(var)))
}

#[wasm_func]
pub fn replace_all(expr: &[u8], pattern: &[u8], replacement: &[u8]) -> Result<Vec<u8>, String> {
    let expr = parse_atom(expr, "expr")?;
    let pattern = parse_atom(pattern, "pattern")?;
    let replacement = parse_atom(replacement, "replacement")?;
    Ok(render_symbolica(&expr.replace(pattern).with(replacement)))
}
