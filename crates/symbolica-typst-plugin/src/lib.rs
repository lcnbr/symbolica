use std::io::Cursor;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom::Error> {
    if dest.is_null() && len != 0 {
        return Err(getrandom::Error::new_custom(1));
    }

    let mut state = 0x9e3779b97f4a7c15u64 ^ len as u64;
    for i in 0..len {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        unsafe { dest.add(i).write((state >> 56) as u8) };
    }

    Ok(())
}

use ciborium::value::Value;
use symbolica::prelude::{Atom, AtomCore, AtomPrinter, Indeterminate, PrintOptions, State, Symbol};
use wasm_minimal_protocol::*;

initiate_protocol!();

fn decode_cbor(input: &[u8], label: &str) -> Result<Value, String> {
    ciborium::from_reader::<Value, _>(Cursor::new(input))
        .map_err(|err| format!("{label} must be CBOR-encoded: {err}"))
}

const ATOM_PAYLOAD_MAGIC: &[u8; 4] = b"SATP";
const ATOM_PAYLOAD_VERSION: u8 = 3;

fn read_u8(input: &mut &[u8], label: &str) -> Result<u8, String> {
    if input.is_empty() {
        return Err(format!("{label} is truncated"));
    }
    let value = input[0];
    *input = &input[1..];
    Ok(value)
}

fn read_u32(input: &mut &[u8], label: &str) -> Result<u32, String> {
    if input.len() < 4 {
        return Err(format!("{label} is truncated"));
    }
    let (value, rest) = input.split_at(4);
    *input = rest;
    Ok(u32::from_le_bytes(value.try_into().expect("u32 slice length")))
}

fn read_u64(input: &mut &[u8], label: &str) -> Result<u64, String> {
    if input.len() < 8 {
        return Err(format!("{label} is truncated"));
    }
    let (value, rest) = input.split_at(8);
    *input = rest;
    Ok(u64::from_le_bytes(value.try_into().expect("u64 slice length")))
}

fn read_bytes<'a>(input: &mut &'a [u8], len: usize, label: &str) -> Result<&'a [u8], String> {
    if input.len() < len {
        return Err(format!("{label} is truncated"));
    }
    let (value, rest) = input.split_at(len);
    *input = rest;
    Ok(value)
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn exported_atom(atom: &Atom) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    atom.export(&mut bytes)
        .map_err(|err| format!("failed to export Atom: {err}"))?;
    Ok(bytes)
}

fn decode_payload(input: &[u8], label: &str) -> Result<Atom, String> {
    let mut rest = input;
    let magic = read_bytes(&mut rest, ATOM_PAYLOAD_MAGIC.len(), label)?;
    if magic != ATOM_PAYLOAD_MAGIC {
        return Atom::import(&mut Cursor::new(input), None)
            .map_err(|err| format!("{label} must be exported Atom bytes: {err}"));
    }

    let version = read_u8(&mut rest, label)?;
    if version != ATOM_PAYLOAD_VERSION {
        return Err(format!("{label} has unsupported Atom payload version {version}"));
    }

    let generation = read_u64(&mut rest, label)?;
    let symbol_count = read_u64(&mut rest, label)?;
    let symbol_fingerprint = read_u64(&mut rest, label)?;
    let raw_len = read_u32(&mut rest, label)? as usize;
    let raw = read_bytes(&mut rest, raw_len, label)?;
    let export_len = read_u32(&mut rest, label)? as usize;
    let export = read_bytes(&mut rest, export_len, label)?;

    if State::raw_symbol_state_is_compatible(generation, symbol_count, symbol_fingerprint)
        && let Ok(atom) = Atom::try_from_raw(raw.to_vec())
    {
        return Ok(atom);
    }

    Atom::import(&mut Cursor::new(export), None)
        .map_err(|err| format!("{label} fallback Atom import failed: {err}"))
}

fn decode_atom(input: &[u8], label: &str) -> Result<Atom, String> {
    decode_payload(input, label)
}

fn encode_atom(atom: &Atom) -> Result<Vec<u8>, String> {
    let raw = atom.clone().into_raw();
    let exported = exported_atom(atom)?;
    let (generation, symbol_count, symbol_fingerprint) = State::raw_symbol_state();

    let mut bytes = Vec::with_capacity(raw.len() + exported.len() + 48);
    bytes.extend_from_slice(ATOM_PAYLOAD_MAGIC);
    bytes.push(ATOM_PAYLOAD_VERSION);
    write_u64(&mut bytes, generation);
    write_u64(&mut bytes, symbol_count);
    write_u64(&mut bytes, symbol_fingerprint);
    write_u32(&mut bytes, raw.len() as u32);
    bytes.extend_from_slice(&raw);
    write_u32(&mut bytes, exported.len() as u32);
    bytes.extend_from_slice(&exported);
    Ok(bytes)
}

fn decode_atom_array(input: &[u8], label: &str) -> Result<Vec<Atom>, String> {
    match decode_cbor(input, label)? {
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| match value {
                Value::Bytes(bytes) => decode_atom(bytes, &format!("{label}[{index}]")),
                other => Err(format!(
                    "{label}[{index}] must be Atom bytes, got {other:?}"
                )),
            })
            .collect(),
        other => Err(format!(
            "{label} must be an array of Atom bytes, got {other:?}"
        )),
    }
}

fn atom_from_ast(input: &[u8], label: &str) -> Result<Atom, String> {
    let value = decode_cbor(input, label)?;
    atom_from_value(&value)
}

fn atom_from_value(value: &Value) -> Result<Atom, String> {
    match value {
        Value::Integer(n) => {
            let n: i64 = (*n)
                .try_into()
                .map_err(|_| "integer literal is out of range".to_owned())?;
            Ok(Atom::num(n))
        }
        Value::Float(n) => Ok(Atom::num(*n)),
        Value::Text(text) => atom_from_leaf(text),
        Value::Map(map) => atom_from_node(map),
        other => Err(format!("unsupported Parsely AST value: {other:?}")),
    }
}

fn atom_from_leaf(text: &str) -> Result<Atom, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("empty math leaf".to_owned());
    }

    if let Ok(n) = text.parse::<i64>() {
        return Ok(Atom::num(n));
    }

    if text.contains('.') || text.contains('e') || text.contains('E') {
        if let Ok(n) = text.parse::<f64>() {
            return Ok(Atom::num(n));
        }
    }

    symbol_atom(text)
}

fn symbol_atom(name: &str) -> Result<Atom, String> {
    Symbol::parse(name.trim(), "typst").map(Atom::var)
}

fn atom_from_node(map: &[(Value, Value)]) -> Result<Atom, String> {
    let head = map_text(map, "head")?;
    let args = map_array(map, "args")?;
    let slots = map_map(map, "slots")?;

    match head {
        "add" => Ok(Atom::add_many(atoms_from_values(args)?)),
        "arg" => Ok(Symbol::ARG.call_args(atoms_from_values(args)?)),
        "mul" | "times" | "dot" => Ok(Atom::mul_many(atoms_from_values(args)?)),
        "sub" => Ok(atom_arg(args, 0, head)? - atom_arg(args, 1, head)?),
        "neg" => Ok(-atom_arg(args, 0, head)?),
        "plus" | "group" | "()" => slot_or_arg_atom(slots, "expr", args, 0, head),
        "lr" => slot_or_arg_atom(slots, "body", args, 0, head),
        "attach" => {
            let base = slot_or_arg_atom(slots, "base", args, 0, head)?;
            if let Some(exp) = slot_atom(slots, "t") {
                Ok(base.pow(exp?))
            } else {
                Ok(base)
            }
        }
        "factorial" => {
            let arg = atom_arg(args, 0, head)?;
            Ok(Symbol::parse("gamma", "symbolica")?.call(arg + Atom::num(1)))
        }
        "frac" => {
            let num = slot_or_arg_atom(slots, "num", args, 0, head)?;
            let denom = slot_or_arg_atom(slots, "denom", args, 1, head)?;
            Ok(num / denom)
        }
        "pow" => {
            let base = slot_or_arg_atom(slots, "base", args, 0, head)?;
            let exp = slot_or_arg_atom(slots, "exp", args, 1, head)?;
            Ok(base.pow(exp))
        }
        "sqrt" => {
            let radicand = slot_atom(slots, "radicand")
                .or_else(|| slot_atom(slots, "body"))
                .unwrap_or_else(|| atom_arg(args, 0, head))?;
            Ok(Symbol::SQRT.call(radicand))
        }
        "root" => {
            let radicand = slot_atom(slots, "radicand")
                .or_else(|| slot_atom(slots, "body"))
                .unwrap_or_else(|| atom_arg(args, 1, head).or_else(|_| atom_arg(args, 0, head)))?;
            if let Some(index) =
                slot_atom(slots, "index").or_else(|| args.first().map(atom_from_value))
            {
                Ok(radicand.pow(Atom::num(1) / index?))
            } else {
                Ok(Symbol::SQRT.call(radicand))
            }
        }
        "abs" | "norm" => {
            let body = slot_atom(slots, "body").unwrap_or_else(|| atom_arg(args, 0, head))?;
            Ok(Symbol::ABS.call(body))
        }
        "call" => {
            let symbol = slot_symbol(slots, "fn")?;
            let args = slot_atoms(slots, "body")?.unwrap_or_default();
            Ok(symbol.call_args(args))
        }
        "op-call" => {
            let symbol = slot_symbol(slots, "op")?;
            let args = slot_atoms(slots, "args")?.unwrap_or_default();
            Ok(symbol.call_args(args))
        }
        _ => {
            let symbol = Symbol::parse(head, "typst")?;
            Ok(symbol.call_args(atoms_from_values(args)?))
        }
    }
}

fn atoms_from_values(values: &[Value]) -> Result<Vec<Atom>, String> {
    values.iter().map(atom_from_value).collect()
}

fn atoms_from_arg_value(value: &Value) -> Result<Vec<Atom>, String> {
    if let Value::Map(map) = value {
        if map_text(map, "head").ok() == Some("arg") {
            return atoms_from_values(map_array(map, "args")?);
        }
    }

    Ok(vec![atom_from_value(value)?])
}

fn slot_atoms(slots: &[(Value, Value)], key: &str) -> Result<Option<Vec<Atom>>, String> {
    map_get(slots, key).map(atoms_from_arg_value).transpose()
}

fn symbol_from_value(value: &Value) -> Result<Symbol, String> {
    match value {
        Value::Text(text) => Symbol::parse(text.trim(), "typst"),
        Value::Map(map) if map_text(map, "head").ok() == Some("op") => {
            if let Some(Value::Text(text)) = map_get(map, "text") {
                Symbol::parse(text.trim(), "typst")
            } else if let Some(Value::Text(text)) = map_get(map_map(map, "slots")?, "text") {
                Symbol::parse(text.trim(), "typst")
            } else {
                Err("op missing text".to_owned())
            }
        }
        other => atom_from_value(other).and_then(|atom| {
            atom.get_symbol()
                .ok_or_else(|| "function head must be a symbol".to_owned())
        }),
    }
}

fn slot_symbol(slots: &[(Value, Value)], key: &str) -> Result<Symbol, String> {
    map_get(slots, key)
        .ok_or_else(|| format!("missing {key}"))
        .and_then(symbol_from_value)
}

fn atom_arg(args: &[Value], index: usize, head: &str) -> Result<Atom, String> {
    args.get(index)
        .ok_or_else(|| format!("{head} missing argument {index}"))
        .and_then(atom_from_value)
}

fn slot_or_arg_atom(
    slots: &[(Value, Value)],
    key: &str,
    args: &[Value],
    index: usize,
    head: &str,
) -> Result<Atom, String> {
    slot_atom(slots, key).unwrap_or_else(|| atom_arg(args, index, head))
}

fn slot_atom(slots: &[(Value, Value)], key: &str) -> Option<Result<Atom, String>> {
    map_get(slots, key).map(atom_from_value)
}

fn map_get<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    map.iter().find_map(|(candidate, value)| match candidate {
        Value::Text(candidate) if candidate == key => Some(value),
        _ => None,
    })
}

fn map_text<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a str, String> {
    match map_get(map, key) {
        Some(Value::Text(text)) => Ok(text),
        Some(_) => Err(format!("{key} must be text")),
        None => Err(format!("missing {key}")),
    }
}

fn map_array<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a [Value], String> {
    match map_get(map, key) {
        Some(Value::Array(values)) => Ok(values),
        Some(_) => Err(format!("{key} must be an array")),
        None => Err(format!("missing {key}")),
    }
}

fn map_map<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a [(Value, Value)], String> {
    match map_get(map, key) {
        Some(Value::Map(values)) => Ok(values),
        Some(_) => Err(format!("{key} must be a dictionary")),
        None => Err(format!("missing {key}")),
    }
}

fn render(atom: &Atom, opts: PrintOptions) -> Vec<u8> {
    AtomPrinter::new_with_options(atom.as_view(), opts)
        .to_string()
        .into_bytes()
}

fn render_symbolica(atom: &Atom) -> Vec<u8> {
    render(atom, PrintOptions::file_no_namespace())
}

#[wasm_func]
pub fn from_ast(ast: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&atom_from_ast(ast, "ast")?)
}

#[wasm_func]
pub fn symbol(name: &[u8]) -> Result<Vec<u8>, String> {
    match decode_cbor(name, "name")? {
        Value::Text(name) => encode_atom(&symbol_atom(&name)?),
        other => Err(format!("name must be text, got {other:?}")),
    }
}

#[wasm_func]
pub fn canonical(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?;
    Ok(render_symbolica(&expr))
}

#[wasm_func]
pub fn to_typst(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?;
    Ok(render(&expr, PrintOptions::typst()))
}

#[wasm_func]
pub fn to_latex(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?;
    Ok(render(&expr, PrintOptions::latex()))
}

#[wasm_func]
pub fn simplify_expr(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?;
    encode_atom(&expr)
}

#[wasm_func]
pub fn expand(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?.expand_via_poly::<u16, Atom>(None);
    encode_atom(&expr)
}

#[wasm_func]
pub fn factor(expr: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?.factor();
    encode_atom(&expr)
}

#[wasm_func]
pub fn derivative(expr: &[u8], var: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?;
    let var = Indeterminate::try_from(decode_atom(var, "var")?)?;
    encode_atom(&expr.derivative(var))
}

#[wasm_func]
pub fn replace_all(expr: &[u8], pattern: &[u8], replacement: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?;
    let pattern = decode_atom(pattern, "pattern")?;
    let replacement = decode_atom(replacement, "replacement")?;
    encode_atom(&expr.replace(pattern).with(replacement))
}

#[wasm_func]
pub fn add(args: &[u8]) -> Result<Vec<u8>, String> {
    let args = decode_atom_array(args, "args")?;
    encode_atom(&Atom::add_many(args))
}

#[wasm_func]
pub fn mul(args: &[u8]) -> Result<Vec<u8>, String> {
    let args = decode_atom_array(args, "args")?;
    encode_atom(&Atom::mul_many(args))
}

#[wasm_func]
pub fn neg(expr: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&(-decode_atom(expr, "expr")?))
}

#[wasm_func]
pub fn sub(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&(decode_atom(lhs, "lhs")? - decode_atom(rhs, "rhs")?))
}

#[wasm_func]
pub fn div(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&(decode_atom(lhs, "lhs")? / decode_atom(rhs, "rhs")?))
}

#[wasm_func]
pub fn power(base: &[u8], exp: &[u8]) -> Result<Vec<u8>, String> {
    encode_atom(&decode_atom(base, "base")?.pow(decode_atom(exp, "exp")?))
}
