use std::io::Cursor;

use ahash::HashMap;

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
use symbolica::domains::SelfRing;
use symbolica::prelude::{
    Atom, AtomCore, AtomPrinter, AtomView, Complex, F64, Indeterminate, IntegerRing, Matrix,
    PrintOptions, PrintState, Q, RationalPolynomial, RationalPolynomialField, ReplaceSettings,
    Replacement, Ring, SeriesDepth, SolveError, State, Symbol, Z,
};
use wasm_minimal_protocol::*;

initiate_protocol!();

type MatrixField = RationalPolynomialField<IntegerRing, u16>;
type MatrixEntry = RationalPolynomial<IntegerRing, u16>;
type PluginMatrix = Matrix<MatrixField>;

const ATOM_PAYLOAD_MAGIC: &[u8; 4] = b"SATP";
const ATOM_PAYLOAD_VERSION: u8 = 3;
const MATRIX_PAYLOAD_MAGIC: &[u8; 4] = b"SMTP";
const MATRIX_PAYLOAD_VERSION: u8 = 1;

fn decode_cbor(input: &[u8], label: &str) -> Result<Value, String> {
    ciborium::from_reader::<Value, _>(Cursor::new(input))
        .map_err(|err| format!("{label} must be CBOR-encoded: {err}"))
}

fn encode_cbor(value: Value) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    ciborium::into_writer(&value, &mut bytes)
        .map_err(|err| format!("failed to encode CBOR: {err}"))?;
    Ok(bytes)
}

fn cbor_bool(input: &[u8], label: &str) -> Result<bool, String> {
    match decode_cbor(input, label)? {
        Value::Bool(value) => Ok(value),
        other => Err(format!("{label} must be bool, got {other:?}")),
    }
}

fn cbor_usize(input: &[u8], label: &str) -> Result<usize, String> {
    value_usize(&decode_cbor(input, label)?, label)
}

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
    Ok(u32::from_le_bytes(
        value.try_into().expect("u32 slice length"),
    ))
}

fn read_u64(input: &mut &[u8], label: &str) -> Result<u64, String> {
    if input.len() < 8 {
        return Err(format!("{label} is truncated"));
    }
    let (value, rest) = input.split_at(8);
    *input = rest;
    Ok(u64::from_le_bytes(
        value.try_into().expect("u64 slice length"),
    ))
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
        return Err(format!(
            "{label} has unsupported Atom payload version {version}"
        ));
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

fn is_matrix_payload(input: &[u8]) -> bool {
    input.starts_with(MATRIX_PAYLOAD_MAGIC)
}

fn atom_to_matrix_entry(atom: &Atom) -> Result<MatrixEntry, String> {
    atom.try_to_rational_polynomial::<_, _, u16>(&Q, &Z, None)
        .map_err(|err| format!("matrix entry must be rational-polynomial compatible: {err}"))
}

fn atoms_to_matrix(atoms: Vec<Atom>, nrows: u32, ncols: u32) -> Result<PluginMatrix, String> {
    if nrows == 0 || ncols == 0 {
        return Err("matrix must have at least one row and one column".to_owned());
    }
    if atoms.len() != nrows as usize * ncols as usize {
        return Err(format!(
            "matrix has {} entries but shape is {nrows}x{ncols}",
            atoms.len()
        ));
    }

    let mut entries = atoms
        .iter()
        .map(atom_to_matrix_entry)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some((first, rest)) = entries.split_first_mut() {
        for _ in 0..2 {
            for entry in &mut *rest {
                first.unify_variables(entry);
            }
        }
    }

    Matrix::from_linear(entries, nrows, ncols, RationalPolynomialField::new(Z))
        .map_err(|err| format!("invalid matrix: {err}"))
}

fn matrix_entries_to_atoms(matrix: &PluginMatrix) -> Vec<Atom> {
    matrix
        .clone()
        .into_vec()
        .into_iter()
        .map(|entry| entry.to_expression())
        .collect()
}

fn encode_matrix(matrix: &PluginMatrix) -> Result<Vec<u8>, String> {
    let atoms = matrix_entries_to_atoms(matrix);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MATRIX_PAYLOAD_MAGIC);
    bytes.push(MATRIX_PAYLOAD_VERSION);
    write_u32(&mut bytes, matrix.nrows() as u32);
    write_u32(&mut bytes, matrix.ncols() as u32);
    write_u32(&mut bytes, atoms.len() as u32);
    for atom in atoms {
        let atom_bytes = encode_atom(&atom)?;
        write_u32(&mut bytes, atom_bytes.len() as u32);
        bytes.extend_from_slice(&atom_bytes);
    }
    Ok(bytes)
}

fn decode_matrix(input: &[u8], label: &str) -> Result<PluginMatrix, String> {
    let mut rest = input;
    let magic = read_bytes(&mut rest, MATRIX_PAYLOAD_MAGIC.len(), label)?;
    if magic != MATRIX_PAYLOAD_MAGIC {
        return Err(format!("{label} must be matrix bytes"));
    }
    let version = read_u8(&mut rest, label)?;
    if version != MATRIX_PAYLOAD_VERSION {
        return Err(format!(
            "{label} has unsupported matrix payload version {version}"
        ));
    }
    let nrows = read_u32(&mut rest, label)?;
    let ncols = read_u32(&mut rest, label)?;
    let len = read_u32(&mut rest, label)? as usize;
    let mut atoms = Vec::with_capacity(len);
    for index in 0..len {
        let atom_len = read_u32(&mut rest, label)? as usize;
        let atom = read_bytes(&mut rest, atom_len, label)?;
        atoms.push(decode_atom(atom, &format!("{label}[{index}]"))?);
    }
    atoms_to_matrix(atoms, nrows, ncols)
}

fn decode_atom_array(input: &[u8], label: &str) -> Result<Vec<Atom>, String> {
    match decode_cbor(input, label)? {
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| atom_from_cbor_value(value, &format!("{label}[{index}]")))
            .collect(),
        other => Err(format!(
            "{label} must be an array of Atom bytes, got {other:?}"
        )),
    }
}

fn atom_from_cbor_value(value: &Value, label: &str) -> Result<Atom, String> {
    match value {
        Value::Bytes(bytes) => decode_atom(bytes, label),
        Value::Integer(n) => {
            let n: i64 = (*n)
                .try_into()
                .map_err(|_| format!("{label} integer is out of range"))?;
            Ok(Atom::num(n))
        }
        Value::Float(n) => Ok(Atom::num(*n)),
        Value::Text(text) => symbol_atom(text, "typst"),
        other => Err(format!("{label} must be Atom bytes, got {other:?}")),
    }
}

fn atom_from_ast(input: &[u8], namespace: &str, label: &str) -> Result<Atom, String> {
    let value = decode_cbor(input, label)?;
    atom_from_value(&value, namespace)
}

fn atom_from_value(value: &Value, namespace: &str) -> Result<Atom, String> {
    match value {
        Value::Integer(n) => {
            let n: i64 = (*n)
                .try_into()
                .map_err(|_| "integer literal is out of range".to_owned())?;
            Ok(Atom::num(n))
        }
        Value::Float(n) => Ok(Atom::num(*n)),
        Value::Text(text) => atom_from_leaf(text, namespace),
        Value::Map(map) => atom_from_node(map, namespace),
        other => Err(format!("unsupported Parsely AST value: {other:?}")),
    }
}

fn atom_from_leaf(text: &str, namespace: &str) -> Result<Atom, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("empty math leaf".to_owned());
    }

    if let Ok(n) = text.parse::<i64>() {
        return Ok(Atom::num(n));
    }

    if (text.contains('.') || text.contains('e') || text.contains('E'))
        && let Ok(n) = text.parse::<f64>()
    {
        return Ok(Atom::num(n));
    }

    symbol_atom(text, namespace)
}

fn symbol_atom(name: &str, namespace: &str) -> Result<Atom, String> {
    Symbol::parse(name.trim(), namespace.to_owned()).map(Atom::var)
}

fn atom_from_node(map: &[(Value, Value)], namespace: &str) -> Result<Atom, String> {
    let head = map_text(map, "head")?;
    let args = map_array(map, "args")?;
    let slots = map_map(map, "slots")?;

    match head {
        "add" => Ok(Atom::add_many(atoms_from_values(args, namespace)?)),
        "arg" => Ok(Symbol::ARG.call_args(atoms_from_values(args, namespace)?)),
        "mul" | "times" | "dot" => Ok(Atom::mul_many(atoms_from_values(args, namespace)?)),
        "sub" => Ok(atom_arg(args, 0, head, namespace)? - atom_arg(args, 1, head, namespace)?),
        "neg" => Ok(-atom_arg(args, 0, head, namespace)?),
        "plus" | "group" | "()" => slot_or_arg_atom(slots, "expr", args, 0, head, namespace),
        "lr" => slot_or_arg_atom(slots, "body", args, 0, head, namespace),
        "attach" => {
            let base = slot_or_arg_atom(slots, "base", args, 0, head, namespace)?;
            if let Some(exp) = slot_atom(slots, "t", namespace) {
                Ok(base.pow(exp?))
            } else {
                Ok(base)
            }
        }
        "factorial" => {
            let arg = atom_arg(args, 0, head, namespace)?;
            Ok(Symbol::parse("gamma", "symbolica")?.call(arg + Atom::num(1)))
        }
        "frac" => {
            let num = slot_or_arg_atom(slots, "num", args, 0, head, namespace)?;
            let denom = slot_or_arg_atom(slots, "denom", args, 1, head, namespace)?;
            Ok(num / denom)
        }
        "pow" => {
            let base = slot_or_arg_atom(slots, "base", args, 0, head, namespace)?;
            let exp = slot_or_arg_atom(slots, "exp", args, 1, head, namespace)?;
            Ok(base.pow(exp))
        }
        "sqrt" => {
            let radicand = slot_atom(slots, "radicand", namespace)
                .or_else(|| slot_atom(slots, "body", namespace))
                .unwrap_or_else(|| atom_arg(args, 0, head, namespace))?;
            Ok(Symbol::SQRT.call(radicand))
        }
        "root" => {
            let radicand = slot_atom(slots, "radicand", namespace)
                .or_else(|| slot_atom(slots, "body", namespace))
                .unwrap_or_else(|| {
                    atom_arg(args, 1, head, namespace)
                        .or_else(|_| atom_arg(args, 0, head, namespace))
                })?;
            if let Some(index) = slot_atom(slots, "index", namespace)
                .or_else(|| args.first().map(|v| atom_from_value(v, namespace)))
            {
                Ok(radicand.pow(Atom::num(1) / index?))
            } else {
                Ok(Symbol::SQRT.call(radicand))
            }
        }
        "abs" | "norm" => {
            let body = slot_atom(slots, "body", namespace)
                .unwrap_or_else(|| atom_arg(args, 0, head, namespace))?;
            Ok(Symbol::ABS.call(body))
        }
        "call" => {
            let symbol = slot_symbol(slots, "fn", namespace)?;
            let args = slot_atoms(slots, "body", namespace)?.unwrap_or_default();
            Ok(symbol.call_args(args))
        }
        "op-call" => {
            let symbol = slot_symbol(slots, "op", namespace)?;
            let args = slot_atoms(slots, "args", namespace)?.unwrap_or_default();
            Ok(symbol.call_args(args))
        }
        "mat" | "vec" => Err(format!(
            "{head} is matrix-valued; use matrix(...) or vec(...)"
        )),
        _ => {
            let symbol = Symbol::parse(head, namespace.to_owned())?;
            Ok(symbol.call_args(atoms_from_values(args, namespace)?))
        }
    }
}

fn atoms_from_values(values: &[Value], namespace: &str) -> Result<Vec<Atom>, String> {
    values
        .iter()
        .map(|v| atom_from_value(v, namespace))
        .collect()
}

fn atoms_from_arg_value(value: &Value, namespace: &str) -> Result<Vec<Atom>, String> {
    if let Value::Map(map) = value
        && map_text(map, "head").ok() == Some("arg")
    {
        return atoms_from_values(map_array(map, "args")?, namespace);
    }

    Ok(vec![atom_from_value(value, namespace)?])
}

fn slot_atoms(
    slots: &[(Value, Value)],
    key: &str,
    namespace: &str,
) -> Result<Option<Vec<Atom>>, String> {
    map_get(slots, key)
        .map(|value| atoms_from_arg_value(value, namespace))
        .transpose()
}

fn symbol_from_value(value: &Value, namespace: &str) -> Result<Symbol, String> {
    match value {
        Value::Text(text) => Symbol::parse(text.trim(), namespace.to_owned()),
        Value::Map(map) if map_text(map, "head").ok() == Some("op") => {
            if let Some(Value::Text(text)) = map_get(map, "text") {
                Symbol::parse(text.trim(), namespace.to_owned())
            } else if let Some(Value::Text(text)) = map_get(map_map(map, "slots")?, "text") {
                Symbol::parse(text.trim(), namespace.to_owned())
            } else {
                Err("op missing text".to_owned())
            }
        }
        other => atom_from_value(other, namespace).and_then(|atom| {
            atom.get_symbol()
                .ok_or_else(|| "function head must be a symbol".to_owned())
        }),
    }
}

fn slot_symbol(slots: &[(Value, Value)], key: &str, namespace: &str) -> Result<Symbol, String> {
    map_get(slots, key)
        .ok_or_else(|| format!("missing {key}"))
        .and_then(|v| symbol_from_value(v, namespace))
}

fn atom_arg(args: &[Value], index: usize, head: &str, namespace: &str) -> Result<Atom, String> {
    args.get(index)
        .ok_or_else(|| format!("{head} missing argument {index}"))
        .and_then(|v| atom_from_value(v, namespace))
}

fn slot_or_arg_atom(
    slots: &[(Value, Value)],
    key: &str,
    args: &[Value],
    index: usize,
    head: &str,
    namespace: &str,
) -> Result<Atom, String> {
    slot_atom(slots, key, namespace).unwrap_or_else(|| atom_arg(args, index, head, namespace))
}

fn slot_atom(slots: &[(Value, Value)], key: &str, namespace: &str) -> Option<Result<Atom, String>> {
    map_get(slots, key).map(|v| atom_from_value(v, namespace))
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

fn map_bool(map: &[(Value, Value)], key: &str, default: bool) -> Result<bool, String> {
    match map_get(map, key) {
        Some(Value::Bool(value)) => Ok(*value),
        Some(other) => Err(format!("{key} must be bool, got {other:?}")),
        None => Ok(default),
    }
}

fn map_usize(map: &[(Value, Value)], key: &str, default: usize) -> Result<usize, String> {
    match map_get(map, key) {
        Some(value) => value_usize(value, key),
        None => Ok(default),
    }
}

fn map_optional_usize(map: &[(Value, Value)], key: &str) -> Result<Option<usize>, String> {
    match map_get(map, key) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => Ok(Some(value_usize(value, key)?)),
    }
}

fn map_i64(map: &[(Value, Value)], key: &str, default: i64) -> Result<i64, String> {
    match map_get(map, key) {
        Some(value) => value_i64(value, key),
        None => Ok(default),
    }
}

fn map_f64(map: &[(Value, Value)], key: &str, default: f64) -> Result<f64, String> {
    match map_get(map, key) {
        Some(value) => value_f64(value, key),
        None => Ok(default),
    }
}

fn map_bytes<'a>(map: &'a [(Value, Value)], key: &str) -> Result<&'a [u8], String> {
    match map_get(map, key) {
        Some(Value::Bytes(bytes)) => Ok(bytes),
        Some(other) => Err(format!("{key} must be bytes, got {other:?}")),
        None => Err(format!("missing {key}")),
    }
}

fn value_i64(value: &Value, label: &str) -> Result<i64, String> {
    match value {
        Value::Integer(n) => (*n)
            .try_into()
            .map_err(|_| format!("{label} integer is out of range")),
        other => Err(format!("{label} must be integer, got {other:?}")),
    }
}

fn value_usize(value: &Value, label: &str) -> Result<usize, String> {
    let n = value_i64(value, label)?;
    n.try_into()
        .map_err(|_| format!("{label} must be non-negative"))
}

fn value_f64(value: &Value, label: &str) -> Result<f64, String> {
    match value {
        Value::Float(n) => Ok(*n),
        Value::Integer(n) => {
            let n: i64 = (*n)
                .try_into()
                .map_err(|_| format!("{label} integer is out of range"))?;
            Ok(n as f64)
        }
        Value::Map(map) => {
            let re = map_f64(map, "re", 0.0)?;
            let im = map_f64(map, "im", 0.0)?;
            if im == 0.0 {
                Ok(re)
            } else {
                Err(format!("{label} must be real for this operation"))
            }
        }
        other => Err(format!("{label} must be number, got {other:?}")),
    }
}

fn value_complex(value: &Value, label: &str) -> Result<Complex<f64>, String> {
    match value {
        Value::Map(map) => Ok(Complex::new(
            map_f64(map, "re", 0.0)?,
            map_f64(map, "im", 0.0)?,
        )),
        _ => Ok(Complex::new(value_f64(value, label)?, 0.0)),
    }
}

fn value_atom_bytes<'a>(value: &'a Value, label: &str) -> Result<&'a [u8], String> {
    match value {
        Value::Bytes(bytes) => Ok(bytes),
        other => Err(format!("{label} must be bytes, got {other:?}")),
    }
}

fn symbol_from_atom_bytes(bytes: &[u8], label: &str) -> Result<Symbol, String> {
    match decode_atom(bytes, label)?.as_view() {
        AtomView::Var(var) => Ok(var.get_symbol()),
        _ => Err(format!("{label} must be a symbol")),
    }
}

fn indeterminate_from_bytes(bytes: &[u8], label: &str) -> Result<Indeterminate, String> {
    Indeterminate::try_from(decode_atom(bytes, label)?).map_err(|err| err.to_string())
}

fn replacement_options(
    map: &[(Value, Value)],
) -> Result<((usize, Option<usize>), bool, bool, bool, usize, Vec<Symbol>), String> {
    let min_level = map_usize(map, "min-level", 0)?;
    let max_level = map_optional_usize(map, "max-level")?;
    let level_range = if let Some(Value::Array(values)) = map_get(map, "level-range") {
        if values.len() != 2 {
            return Err("level-range must have two entries".to_owned());
        }
        let max = match &values[1] {
            Value::Null => None,
            value => Some(value_usize(value, "level-range[1]")?),
        };
        (value_usize(&values[0], "level-range[0]")?, max)
    } else {
        (min_level, max_level)
    };

    let non_greedy = if let Some(Value::Array(values)) = map_get(map, "non-greedy-wildcards") {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let bytes = value_atom_bytes(value, &format!("non-greedy-wildcards[{index}]"))?;
                symbol_from_atom_bytes(bytes, &format!("non-greedy-wildcards[{index}]"))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };

    Ok((
        level_range,
        map_bool(map, "level-is-tree-depth", false)?,
        map_bool(map, "partial", true)?,
        map_bool(map, "allow-new-wildcards-on-rhs", false)?,
        map_usize(map, "rhs-cache-size", 100)?,
        non_greedy,
    ))
}

fn build_replace_settings(map: &[(Value, Value)]) -> Result<ReplaceSettings, String> {
    Ok(ReplaceSettings::new()
        .once(map_bool(map, "once", false)?)
        .bottom_up(map_bool(map, "bottom-up", false)?)
        .nested(map_bool(map, "nested", false)?))
}

fn build_replacement(map: &[(Value, Value)]) -> Result<Replacement, String> {
    let pattern_atom = decode_atom(map_bytes(map, "pattern")?, "pattern")?;
    let rhs_atom = decode_atom(map_bytes(map, "rhs")?, "rhs")?;
    let pattern = pattern_atom.to_pattern();
    let rhs = rhs_atom.to_pattern();
    let (
        level_range,
        level_is_tree_depth,
        partial,
        allow_new_wildcards_on_rhs,
        rhs_cache_size,
        non_greedy,
    ) = replacement_options(map)?;

    if !allow_new_wildcards_on_rhs && let Some(wildcard) = pattern.find_new_wildcard(&rhs) {
        return Err(format!(
            "Wildcard {} does not appear in pattern",
            wildcard.get_name()
        ));
    }

    Ok(Replacement::new(pattern, rhs)
        .level_range(level_range)
        .level_is_tree_depth(level_is_tree_depth)
        .partial(partial)
        .allow_new_wildcards_on_rhs(allow_new_wildcards_on_rhs)
        .rhs_cache_size(rhs_cache_size)
        .non_greedy_wildcards(non_greedy))
}

fn repeat_replacements(
    expr: Atom,
    replacements: Vec<Replacement>,
    settings: ReplaceSettings,
    repeat: bool,
) -> Atom {
    let mut current = expr;
    loop {
        let next = current.replace_multiple_with_settings(&replacements, settings);
        if !repeat || next == current {
            return next;
        }
        current = next;
    }
}

fn symbolica_options(namespaces: bool) -> PrintOptions {
    if namespaces {
        PrintOptions::file()
    } else {
        PrintOptions::file_no_namespace()
    }
}

fn render_atom(atom: &Atom, opts: PrintOptions) -> Vec<u8> {
    AtomPrinter::new_with_options(atom.as_view(), opts)
        .to_string()
        .into_bytes()
}

fn render_payload_symbolica(input: &[u8], namespaces: bool) -> Result<Vec<u8>, String> {
    if is_matrix_payload(input) {
        let matrix = decode_matrix(input, "matrix")?;
        Ok(matrix
            .format_string(&symbolica_options(namespaces), PrintState::new())
            .into_bytes())
    } else {
        let expr = decode_atom(input, "expr")?;
        Ok(render_atom(&expr, symbolica_options(namespaces)))
    }
}

fn render_matrix_typst(matrix: &PluginMatrix) -> Result<Vec<u8>, String> {
    let entries = matrix_entries_to_atoms(matrix);
    let nrows = matrix.nrows();
    let ncols = matrix.ncols();
    let mut out = String::from("mat(");
    for row in 0..nrows {
        if row > 0 {
            out.push_str("; ");
        }
        for col in 0..ncols {
            if col > 0 {
                out.push_str(", ");
            }
            let atom = &entries[row * ncols + col];
            out.push_str(
                &String::from_utf8(render_atom(atom, PrintOptions::typst()))
                    .map_err(|e| e.to_string())?,
            );
        }
    }
    out.push(')');
    Ok(out.into_bytes())
}

fn render_payload_typst(input: &[u8]) -> Result<Vec<u8>, String> {
    if is_matrix_payload(input) {
        render_matrix_typst(&decode_matrix(input, "matrix")?)
    } else {
        let expr = decode_atom(input, "expr")?;
        Ok(render_atom(&expr, PrintOptions::typst()))
    }
}

fn decode_nested_matrix(input: &[u8]) -> Result<PluginMatrix, String> {
    match decode_cbor(input, "matrix")? {
        Value::Array(rows) => {
            if rows.is_empty() {
                return Err("matrix must have at least one row".to_owned());
            }
            let nrows = rows.len() as u32;
            let mut ncols = None;
            let mut atoms = Vec::new();
            for (row_index, row) in rows.iter().enumerate() {
                let Value::Array(cols) = row else {
                    return Err(format!("matrix row {row_index} must be an array"));
                };
                if cols.is_empty() {
                    return Err(format!("matrix row {row_index} is empty"));
                }
                if let Some(expected) = ncols {
                    if expected != cols.len() {
                        return Err("matrix is not rectangular".to_owned());
                    }
                } else {
                    ncols = Some(cols.len());
                }
                for (col_index, value) in cols.iter().enumerate() {
                    atoms.push(atom_from_cbor_value(
                        value,
                        &format!("matrix[{row_index}][{col_index}]"),
                    )?);
                }
            }
            atoms_to_matrix(atoms, nrows, ncols.unwrap() as u32)
        }
        other => Err(format!("matrix must be nested array, got {other:?}")),
    }
}

fn matrix_vec_from_values(input: &[u8]) -> Result<PluginMatrix, String> {
    let atoms = decode_atom_array(input, "values")?;
    atoms_to_matrix(atoms, cbor_len(input, "values")? as u32, 1)
}

fn cbor_len(input: &[u8], label: &str) -> Result<usize, String> {
    match decode_cbor(input, label)? {
        Value::Array(values) => Ok(values.len()),
        other => Err(format!("{label} must be array, got {other:?}")),
    }
}

fn matrix_from_diag(input: &[u8]) -> Result<PluginMatrix, String> {
    let atoms = decode_atom_array(input, "diag")?;
    if atoms.is_empty() {
        return Err("diagonal must not be empty".to_owned());
    }
    let mut entries = atoms
        .iter()
        .map(atom_to_matrix_entry)
        .collect::<Result<Vec<_>, _>>()?;
    if let Some((first, rest)) = entries.split_first_mut() {
        for _ in 0..2 {
            for entry in &mut *rest {
                first.unify_variables(entry);
            }
        }
    }
    Ok(Matrix::eye(&entries, RationalPolynomialField::new(Z)))
}

fn unify_matrices(lhs: &PluginMatrix, rhs: &PluginMatrix) -> (PluginMatrix, PluginMatrix) {
    let mut zero = lhs.field().zero();
    let mut lhs_data = lhs.clone().into_vec();
    let mut rhs_data = rhs.clone().into_vec();
    for entry in &mut lhs_data {
        zero.unify_variables(entry);
    }
    for entry in &mut rhs_data {
        zero.unify_variables(entry);
    }
    (
        Matrix::from_linear(
            lhs_data,
            lhs.nrows() as u32,
            lhs.ncols() as u32,
            RationalPolynomialField::new(Z),
        )
        .expect("existing lhs matrix shape"),
        Matrix::from_linear(
            rhs_data,
            rhs.nrows() as u32,
            rhs.ncols() as u32,
            RationalPolynomialField::new(Z),
        )
        .expect("existing rhs matrix shape"),
    )
}

fn unify_matrix_scalar(matrix: &PluginMatrix, scalar: MatrixEntry) -> (PluginMatrix, MatrixEntry) {
    let mut zero = matrix.field().zero();
    let mut data = matrix.clone().into_vec();
    for entry in &mut data {
        zero.unify_variables(entry);
    }
    let mut scalar = scalar;
    zero.unify_variables(&mut scalar);
    (
        Matrix::from_linear(
            data,
            matrix.nrows() as u32,
            matrix.ncols() as u32,
            RationalPolynomialField::new(Z),
        )
        .expect("existing matrix shape"),
        scalar,
    )
}

fn cbor_atom_array(atoms: Vec<Atom>) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Array(
        atoms
            .iter()
            .map(|atom| encode_atom(atom).map(Value::Bytes))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn cbor_f64(value: f64) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Float(value))
}

fn cbor_f64_array(values: Vec<f64>) -> Result<Vec<u8>, String> {
    encode_cbor(Value::Array(values.into_iter().map(Value::Float).collect()))
}

#[wasm_func]
pub fn from_ast(ast: &[u8], namespace: &[u8]) -> Result<Vec<u8>, String> {
    let namespace = match decode_cbor(namespace, "namespace")? {
        Value::Text(namespace) => namespace,
        other => return Err(format!("namespace must be text, got {other:?}")),
    };
    encode_atom(&atom_from_ast(ast, &namespace, "ast")?)
}

#[wasm_func]
pub fn symbol(name: &[u8], namespace: &[u8]) -> Result<Vec<u8>, String> {
    let name = match decode_cbor(name, "name")? {
        Value::Text(name) => name,
        other => return Err(format!("name must be text, got {other:?}")),
    };
    let namespace = match decode_cbor(namespace, "namespace")? {
        Value::Text(namespace) => namespace,
        other => return Err(format!("namespace must be text, got {other:?}")),
    };
    encode_atom(&symbol_atom(&name, &namespace)?)
}

#[wasm_func]
pub fn canonical(payload: &[u8], namespaces: &[u8]) -> Result<Vec<u8>, String> {
    render_payload_symbolica(payload, cbor_bool(namespaces, "namespaces")?)
}

#[wasm_func]
pub fn to_typst(payload: &[u8]) -> Result<Vec<u8>, String> {
    render_payload_typst(payload)
}

#[wasm_func]
pub fn to_latex(expr: &[u8]) -> Result<Vec<u8>, String> {
    if is_matrix_payload(expr) {
        let matrix = decode_matrix(expr, "matrix")?;
        Ok(matrix
            .format_string(&PrintOptions::latex(), PrintState::new())
            .into_bytes())
    } else {
        let expr = decode_atom(expr, "expr")?;
        Ok(render_atom(&expr, PrintOptions::latex()))
    }
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
pub fn series(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "series request")? else {
        return Err("series request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let var = indeterminate_from_bytes(map_bytes(&map, "var")?, "var")?;
    let expansion_point = decode_atom(map_bytes(&map, "expansion-point")?, "expansion-point")?;
    let depth = map_i64(&map, "depth", 0)?;
    let depth_denom = map_i64(&map, "depth-denom", 1)?;
    let depth = if map_bool(&map, "depth-is-absolute", true)? {
        SeriesDepth::absolute((depth, depth_denom))
    } else {
        SeriesDepth::relative((depth, depth_denom))
    };
    encode_atom(
        &expr
            .series(var, expansion_point.as_view(), depth)
            .map_err(|err| err.to_string())?
            .to_atom(),
    )
}

#[wasm_func]
pub fn replace_all(expr: &[u8], pattern: &[u8], replacement: &[u8]) -> Result<Vec<u8>, String> {
    let expr = decode_atom(expr, "expr")?;
    let pattern = decode_atom(pattern, "pattern")?;
    let replacement = decode_atom(replacement, "replacement")?;
    encode_atom(&expr.replace(pattern).with(replacement))
}

#[wasm_func]
pub fn replace(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "replace request")? else {
        return Err("replace request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let replacement = build_replacement(&map)?;
    let settings = build_replace_settings(&map)?;
    let repeat = map_bool(&map, "repeat", false)?;
    encode_atom(&repeat_replacements(
        expr,
        vec![replacement],
        settings,
        repeat,
    ))
}

#[wasm_func]
pub fn replace_multiple(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "replace-multiple request")? else {
        return Err("replace-multiple request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let Value::Array(rules) = map_get(&map, "rules").ok_or_else(|| "missing rules".to_owned())?
    else {
        return Err("rules must be array".to_owned());
    };
    let replacements = rules
        .iter()
        .enumerate()
        .map(|(index, rule)| {
            let Value::Map(rule) = rule else {
                return Err(format!("rules[{index}] must be dictionary"));
            };
            build_replacement(rule)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let settings = build_replace_settings(&map)?;
    let repeat = map_bool(&map, "repeat", false)?;
    encode_atom(&repeat_replacements(expr, replacements, settings, repeat))
}

#[wasm_func]
pub fn replace_wildcards(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "replace-wildcards request")? else {
        return Err("replace-wildcards request must be dictionary".to_owned());
    };
    let pattern = decode_atom(map_bytes(&map, "pattern")?, "pattern")?.to_pattern();
    let Value::Array(replacements) =
        map_get(&map, "replacements").ok_or_else(|| "missing replacements".to_owned())?
    else {
        return Err("replacements must be array".to_owned());
    };
    let mut map = HashMap::default();
    for (index, replacement) in replacements.iter().enumerate() {
        let Value::Array(pair) = replacement else {
            return Err(format!("replacements[{index}] must be pair"));
        };
        if pair.len() != 2 {
            return Err(format!("replacements[{index}] must have two entries"));
        }
        let wildcard = symbol_from_atom_bytes(value_atom_bytes(&pair[0], "wildcard")?, "wildcard")?;
        if wildcard.get_wildcard_level() == 0 {
            return Err("only wildcards can be replaced".to_owned());
        }
        map.insert(
            wildcard,
            decode_atom(value_atom_bytes(&pair[1], "replacement")?, "replacement")?,
        );
    }
    encode_atom(
        &pattern
            .replace_wildcards(&map)
            .map_err(|err| err.to_string())?,
    )
}

#[wasm_func]
pub fn evaluate(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "evaluate request")? else {
        return Err("evaluate request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let mut constants = HashMap::default();
    if let Some(Value::Array(values)) = map_get(&map, "values") {
        for (index, value) in values.iter().enumerate() {
            let Value::Array(pair) = value else {
                return Err(format!("values[{index}] must be pair"));
            };
            if pair.len() != 2 {
                return Err(format!("values[{index}] must have two entries"));
            }
            constants.insert(
                decode_atom(value_atom_bytes(&pair[0], "value key")?, "value key")?,
                value_complex(&pair[1], "value")?,
            );
        }
    }
    let result = expr
        .evaluate(&constants)
        .map_err(|err| format!("could not evaluate expression: {err}"))?;
    encode_cbor(Value::Map(vec![
        (Value::Text("re".to_owned()), Value::Float(result.re)),
        (Value::Text("im".to_owned()), Value::Float(result.im)),
    ]))
}

#[wasm_func]
pub fn solve_linear(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "solve-linear request")? else {
        return Err("solve-linear request must be dictionary".to_owned());
    };
    let system = match map_get(&map, "system") {
        Some(Value::Array(values)) => values
            .iter()
            .enumerate()
            .map(|(index, value)| atom_from_cbor_value(value, &format!("system[{index}]")))
            .collect::<Result<Vec<_>, _>>()?,
        Some(Value::Bytes(bytes)) if is_matrix_payload(bytes) => {
            let matrix = decode_matrix(bytes, "system")?;
            matrix_entries_to_atoms(&matrix)
        }
        Some(other) => {
            return Err(format!(
                "system must be array or vector matrix, got {other:?}"
            ));
        }
        None => return Err("missing system".to_owned()),
    };
    let vars = match map_get(&map, "variables") {
        Some(Value::Array(values)) => values
            .iter()
            .enumerate()
            .map(|(index, value)| atom_from_cbor_value(value, &format!("variables[{index}]")))
            .collect::<Result<Vec<_>, _>>()?,
        Some(other) => return Err(format!("variables must be array, got {other:?}")),
        None => return Err("missing variables".to_owned()),
    };
    let result = match AtomView::solve_linear_system::<u16, _, Atom>(&system, &vars) {
        Ok(result) => result,
        Err(SolveError::Underdetermined {
            partial_solution, ..
        }) => partial_solution,
        Err(err) => return Err(err.to_string()),
    };
    cbor_atom_array(result)
}

#[wasm_func]
pub fn nsolve(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "nsolve request")? else {
        return Err("nsolve request must be dictionary".to_owned());
    };
    let expr = decode_atom(map_bytes(&map, "expr")?, "expr")?;
    let var = indeterminate_from_bytes(map_bytes(&map, "var")?, "var")?;
    let init = map_f64(&map, "init", 1.0)?;
    let prec = map_f64(&map, "prec", 1e-4)?;
    let max_iterations = map_usize(&map, "max-iterations", 1000)?;
    let result: F64 = expr
        .nsolve::<F64, _>(var, init.into(), prec.into(), max_iterations)
        .map_err(|err| format!("could not solve expression: {err}"))?;
    cbor_f64(result.into_inner())
}

#[wasm_func]
pub fn nsolve_system(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "nsolve-system request")? else {
        return Err("nsolve-system request must be dictionary".to_owned());
    };
    let Value::Array(system_values) =
        map_get(&map, "system").ok_or_else(|| "missing system".to_owned())?
    else {
        return Err("system must be array".to_owned());
    };
    let system = system_values
        .iter()
        .enumerate()
        .map(|(index, value)| atom_from_cbor_value(value, &format!("system[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let system_views = system.iter().map(|atom| atom.as_view()).collect::<Vec<_>>();

    let Value::Array(variable_values) =
        map_get(&map, "variables").ok_or_else(|| "missing variables".to_owned())?
    else {
        return Err("variables must be array".to_owned());
    };
    let vars = variable_values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            indeterminate_from_bytes(
                value_atom_bytes(value, "variable")?,
                &format!("variables[{index}]"),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let Value::Array(init_values) =
        map_get(&map, "init").ok_or_else(|| "missing init".to_owned())?
    else {
        return Err("init must be array".to_owned());
    };
    let init = init_values
        .iter()
        .enumerate()
        .map(|(index, value)| value_f64(value, &format!("init[{index}]")).map(F64::from))
        .collect::<Result<Vec<_>, _>>()?;

    let prec = map_f64(&map, "prec", 1e-4)?;
    let max_iterations = map_usize(&map, "max-iterations", 1000)?;
    let result: Vec<F64> =
        AtomView::nsolve_system(&system_views, &vars, &init, prec.into(), max_iterations)
            .map_err(|err| format!("could not solve system: {err}"))?;
    cbor_f64_array(result.into_iter().map(F64::into_inner).collect())
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

#[wasm_func]
pub fn matrix_from_nested(input: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(&decode_nested_matrix(input)?)
}

#[wasm_func]
pub fn matrix_vec(values: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(&matrix_vec_from_values(values)?)
}

#[wasm_func]
pub fn matrix_identity(n: &[u8]) -> Result<Vec<u8>, String> {
    let n = cbor_usize(n, "n")?;
    if n == 0 {
        return Err("identity matrix must be non-empty".to_owned());
    }
    encode_matrix(&Matrix::identity(n as u32, RationalPolynomialField::new(Z)))
}

#[wasm_func]
pub fn matrix_eye(diag: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(&matrix_from_diag(diag)?)
}

#[wasm_func]
pub fn matrix_add(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs) = unify_matrices(&decode_matrix(lhs, "lhs")?, &decode_matrix(rhs, "rhs")?);
    encode_matrix(&(&lhs + &rhs))
}

#[wasm_func]
pub fn matrix_sub(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs) = unify_matrices(&decode_matrix(lhs, "lhs")?, &decode_matrix(rhs, "rhs")?);
    encode_matrix(&(&lhs - &rhs))
}

#[wasm_func]
pub fn matrix_mul(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let lhs = decode_matrix(lhs, "lhs")?;
    if is_matrix_payload(rhs) {
        let (lhs, rhs) = unify_matrices(&lhs, &decode_matrix(rhs, "rhs")?);
        encode_matrix(&(&lhs * &rhs))
    } else {
        let scalar = atom_to_matrix_entry(&decode_atom(rhs, "rhs")?)?;
        let (lhs, scalar) = unify_matrix_scalar(&lhs, scalar);
        encode_matrix(&lhs.mul_scalar(&scalar))
    }
}

#[wasm_func]
pub fn matrix_div_scalar(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let lhs = decode_matrix(lhs, "lhs")?;
    let scalar = atom_to_matrix_entry(&decode_atom(rhs, "rhs")?)?;
    let (lhs, scalar) = unify_matrix_scalar(&lhs, scalar);
    if scalar.is_zero() {
        return Err("cannot divide a matrix by zero".to_owned());
    }
    encode_matrix(&lhs.div_scalar(&scalar))
}

#[wasm_func]
pub fn transpose(matrix: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(&decode_matrix(matrix, "matrix")?.transpose())
}

#[wasm_func]
pub fn det(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let det = decode_matrix(matrix, "matrix")?
        .det()
        .map_err(|err| err.to_string())?;
    encode_atom(&det.to_expression())
}

#[wasm_func]
pub fn inv(matrix: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(
        &decode_matrix(matrix, "matrix")?
            .inv()
            .map_err(|err| err.to_string())?,
    )
}

#[wasm_func]
pub fn matrix_solve(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs) = unify_matrices(&decode_matrix(lhs, "lhs")?, &decode_matrix(rhs, "rhs")?);
    encode_matrix(&lhs.solve(&rhs).map_err(|err| err.to_string())?)
}

#[wasm_func]
pub fn matrix_solve_any(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs) = unify_matrices(&decode_matrix(lhs, "lhs")?, &decode_matrix(rhs, "rhs")?);
    encode_matrix(&lhs.solve_any(&rhs).map_err(|err| err.to_string())?)
}

#[wasm_func]
pub fn row_reduce(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "row-reduce request")? else {
        return Err("row-reduce request must be dictionary".to_owned());
    };
    let mut matrix = decode_matrix(map_bytes(&map, "matrix")?, "matrix")?;
    let max_col = map_usize(&map, "max-col", matrix.ncols())? as u32;
    let rank = matrix.row_reduce(max_col);
    encode_cbor(Value::Map(vec![
        (
            Value::Text("matrix".to_owned()),
            Value::Bytes(encode_matrix(&matrix)?),
        ),
        (
            Value::Text("rank".to_owned()),
            Value::Integer((rank as i64).into()),
        ),
    ]))
}

#[wasm_func]
pub fn augment(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, String> {
    let (lhs, rhs) = unify_matrices(&decode_matrix(lhs, "lhs")?, &decode_matrix(rhs, "rhs")?);
    encode_matrix(&lhs.augment(&rhs).map_err(|err| err.to_string())?)
}

#[wasm_func]
pub fn split_col(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "split-col request")? else {
        return Err("split-col request must be dictionary".to_owned());
    };
    let matrix = decode_matrix(map_bytes(&map, "matrix")?, "matrix")?;
    let index = map_usize(&map, "index", 0)? as u32;
    let (lhs, rhs) = matrix.split_col(index).map_err(|err| err.to_string())?;
    encode_cbor(Value::Array(vec![
        Value::Bytes(encode_matrix(&lhs)?),
        Value::Bytes(encode_matrix(&rhs)?),
    ]))
}

#[wasm_func]
pub fn primitive_part(matrix: &[u8]) -> Result<Vec<u8>, String> {
    encode_matrix(&decode_matrix(matrix, "matrix")?.primitive_part())
}

#[wasm_func]
pub fn content(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let content = decode_matrix(matrix, "matrix")?.content();
    encode_atom(&content.to_expression())
}

#[wasm_func]
pub fn matrix_at(request: &[u8]) -> Result<Vec<u8>, String> {
    let Value::Map(map) = decode_cbor(request, "matrix-at request")? else {
        return Err("matrix-at request must be dictionary".to_owned());
    };
    let matrix = decode_matrix(map_bytes(&map, "matrix")?, "matrix")?;
    let row = map_usize(&map, "row", 0)?;
    let col = map_usize(&map, "col", 0)?;
    if row >= matrix.nrows() || col >= matrix.ncols() {
        return Err("matrix index out of bounds".to_owned());
    }
    encode_atom(&matrix[(row as u32, col as u32)].clone().to_expression())
}

#[wasm_func]
pub fn matrix_shape(matrix: &[u8]) -> Result<Vec<u8>, String> {
    let matrix = decode_matrix(matrix, "matrix")?;
    encode_cbor(Value::Array(vec![
        Value::Integer((matrix.nrows() as i64).into()),
        Value::Integer((matrix.ncols() as i64).into()),
    ]))
}
