#import "@preview/parsely:0.1.0"

#let _default_grammar = (
  arg: (infix: $,$, assoc: true, prec: 4),
  add: (infix: $+$, prec: 1, assoc: true),
  sub: (infix: $-$, prec: 1),
  plus: (prefix: $+$, prec: 2),
  neg: (prefix: $-$, prec: 2),
  times: (infix: $times$, prec: 2),
  dot: (infix: $dot$, prec: 2),
  factorial: (postfix: $#parsely.tight !$, prec: 3),
  mul: (infix: $$, prec: 2.5, assoc: true),
  "()": (match: $(#parsely.slot("expr*"))$),
  pow: (match: $#parsely.slot("base")^#parsely.slot("exp")$),
  union: (infix: $union$, prec: 1),
  inter: (infix: $inter$, prec: 1),
  attach: math.attach,
  frac: math.frac,
  lr: math.lr,
  mat: math.mat,
  vec: math.vec,
  root: math.root,
  op-call: (match: $op(#parsely.slot("op"))(#parsely.slot("args*"))$),
  call: (match: $#parsely.slot("fn") #parsely.tight (#parsely.slot("body*"))$),
  op: math.op,
)
#let _typst_math = math

#let _leaf(value) = {
  if type(value) == str {
    value
  } else if type(value) == content {
    let fields = value.fields()
    if "text" in fields {
      fields.text
    } else if "children" in fields {
      fields.children.map(_leaf).join("")
    } else {
      repr(value)
    }
  } else {
    repr(value)
  }
}

#let _node_to_ast(node) = (
  head: node.head,
  args: node.args,
  slots: node.slots,
)

#let _is_space(value) = {
  if type(value) == str { return value.trim() == "" }
  if type(value) == content {
    if repr(value.func()) == "space" { return true }
    if repr(value.func()) == "symbol" and "text" in value.fields() {
      return value.fields().text.trim() == ""
    }
  }
  false
}

#let _content_positional_fields = (
  attach: ("base",),
  equation: ("body",),
  frac: ("num", "denom"),
  lr: ("body",),
  mat: (),
  vec: (),
  root: ("index", "radicand"),
)

#let _call_content(fn, fields) = {
  let kind = repr(fn)
  if kind == "sequence" and "children" in fields {
    return fields.children.join()
  }

  let pos = ()
  for field in _content_positional_fields.at(kind, default: ()) {
    if field in fields {
      pos.push(fields.remove(field))
    }
  }
  fn(..pos, ..fields)
}

#let _trim_math(value) = {
  let trim_array(values) = {
    let values = values.map(_trim_math)
    while values.len() > 0 and _is_space(values.first()) {
      values = values.slice(1)
    }
    while values.len() > 0 and _is_space(values.last()) {
      values = values.slice(0, values.len() - 1)
    }
    values
  }

  if type(value) == array { return trim_array(value) }
  if type(value) != content { return value }

  let kind = repr(value.func())
  if kind != "sequence" and kind not in _content_positional_fields {
    return value
  }

  let fields = (:)
  for (key, field) in value.fields() {
    fields.insert(key, _trim_math(field))
  }
  _call_content(value.func(), fields)
}

#let _namespace(engine, namespace) = if namespace == none { engine.namespace } else { namespace }
#let _namespace_bytes(engine, namespace: none) = cbor.encode(_namespace(engine, namespace))

#let _ast_bytes(eqn, grammar) = {
  let parsed = parsely.parse(_trim_math(eqn), grammar)
  let tree = parsely.walk(parsed.tree, post: _node_to_ast, leaf: _leaf)
  cbor.encode(tree)
}

#let _from_math(engine, eqn, grammar: none, namespace: none) = {
  let grammar = if grammar == none { engine.grammar } else { grammar }
  engine.plugin.from_ast(_ast_bytes(eqn, grammar), _namespace_bytes(engine, namespace: namespace))
}

#let _array_tree(engine, eqn, grammar: none) = {
  let grammar = if grammar == none { engine.grammar } else { grammar }
  let parsed = parsely.parse(_trim_math(eqn), grammar)
  parsely.walk(parsed.tree, post: it => (
    strong(raw(it.head)),
    ..it.args,
    ..it.slots.pairs().map(((slot, it)) => {
      (text(gray, 0.8em, raw(slot)), it)
    }),
  ), leaf: _typst_math.equation)
}

#let _var(engine, name, namespace: none) = engine.plugin.symbol(cbor.encode(name), _namespace_bytes(engine, namespace: namespace))
#let _symbol(engine, name, namespace: none) = _var(engine, name, namespace: namespace)
#let _wild(engine, name, level: 1, namespace: none) = {
  let suffix = ""
  for _ in range(level) { suffix += "_" }
  _var(engine, name + suffix, namespace: namespace)
}
#let _dummy(engine, name, level: 1, namespace: none) = _wild(engine, name, level: level, namespace: namespace)

#let _expr_bytes(engine, expr, namespace: none) = {
  if type(expr) == bytes {
    expr
  } else if type(expr) == content {
    _from_math(engine, expr, namespace: namespace)
  } else {
    engine.plugin.from_ast(cbor.encode(expr), _namespace_bytes(engine, namespace: namespace))
  }
}

#let _atom_array(engine, values, namespace: none) = cbor.encode(values.map(value => _expr_bytes(engine, value, namespace: namespace)))
#let _payload_bytes(engine, value, namespace: none) = if type(value) == bytes { value } else { _expr_bytes(engine, value, namespace: namespace) }

#let _canonical(engine, expr, namespaces: false) = str(engine.plugin.canonical(_payload_bytes(engine, expr), cbor.encode(namespaces)))
#let _to_typst_source(engine, expr) = str(engine.plugin.to_typst(_payload_bytes(engine, expr)))
#let _to_typst(engine, expr, block: false) = {
  let eqn = eval(_to_typst_source(engine, expr), mode: "math")
  if block { _typst_math.equation(eqn.body, block: true) } else { eqn }
}
#let _to_latex(engine, expr) = str(engine.plugin.to_latex(_payload_bytes(engine, expr)))

#let _simplify(engine, expr) = engine.plugin.simplify_expr(_expr_bytes(engine, expr))
#let _expand(engine, expr) = engine.plugin.expand(_expr_bytes(engine, expr))
#let _factor(engine, expr) = engine.plugin.factor(_expr_bytes(engine, expr))
#let _derivative(engine, expr, var) = engine.plugin.derivative(_expr_bytes(engine, expr), _expr_bytes(engine, var))
#let _series(engine, expr, var, expansion-point, depth, depth-denom: 1, depth-is-absolute: true) = {
  engine.plugin.series(cbor.encode((
    expr: _expr_bytes(engine, expr),
    var: _expr_bytes(engine, var),
    expansion-point: _expr_bytes(engine, expansion-point),
    depth: depth,
    depth-denom: depth-denom,
    depth-is-absolute: depth-is-absolute,
  )))
}

#let _replacement_options(engine,
  non-greedy-wildcards: (),
  min-level: 0,
  max-level: none,
  level-range: none,
  level-is-tree-depth: false,
  partial: true,
  allow-new-wildcards-on-rhs: false,
  rhs-cache-size: 100,
) = (
  non-greedy-wildcards: non-greedy-wildcards.map(w => _expr_bytes(engine, w)),
  min-level: min-level,
  max-level: max-level,
  level-range: level-range,
  level-is-tree-depth: level-is-tree-depth,
  partial: partial,
  allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs,
  rhs-cache-size: rhs-cache-size,
)

#let _rule(engine, pattern, rhs,
  non-greedy-wildcards: (),
  min-level: 0,
  max-level: none,
  level-range: none,
  level-is-tree-depth: false,
  partial: true,
  allow-new-wildcards-on-rhs: false,
  rhs-cache-size: 100,
) = (
  pattern: _expr_bytes(engine, pattern),
  rhs: _expr_bytes(engine, rhs),
  .._replacement_options(engine,
    non-greedy-wildcards: non-greedy-wildcards,
    min-level: min-level,
    max-level: max-level,
    level-range: level-range,
    level-is-tree-depth: level-is-tree-depth,
    partial: partial,
    allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs,
    rhs-cache-size: rhs-cache-size,
  ),
)

#let _replace(engine, expr, pattern, rhs,
  repeat: false,
  once: false,
  bottom-up: false,
  nested: false,
  non-greedy-wildcards: (),
  min-level: 0,
  max-level: none,
  level-range: none,
  level-is-tree-depth: false,
  partial: true,
  allow-new-wildcards-on-rhs: false,
  rhs-cache-size: 100,
) = {
  engine.plugin.replace(cbor.encode((
    expr: _expr_bytes(engine, expr),
    once: once,
    repeat: repeat,
    bottom-up: bottom-up,
    nested: nested,
    .._rule(engine, pattern, rhs,
      non-greedy-wildcards: non-greedy-wildcards,
      min-level: min-level,
      max-level: max-level,
      level-range: level-range,
      level-is-tree-depth: level-is-tree-depth,
      partial: partial,
      allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs,
      rhs-cache-size: rhs-cache-size,
    ),
  )))
}

#let _replace_multiple(engine, expr, rules, repeat: false, once: false, bottom-up: false, nested: false) = {
  engine.plugin.replace_multiple(cbor.encode((
    expr: _expr_bytes(engine, expr),
    rules: rules,
    once: once,
    repeat: repeat,
    bottom-up: bottom-up,
    nested: nested,
  )))
}

#let _replace_all(engine, expr, pattern, replacement) = _replace(engine, expr, pattern, replacement)
#let _replace_wildcards(engine, pattern, replacements) = {
  let pairs = replacements.map(pair => (
    _expr_bytes(engine, pair.at(0)),
    _expr_bytes(engine, pair.at(1)),
  ))
  engine.plugin.replace_wildcards(cbor.encode((pattern: _expr_bytes(engine, pattern), replacements: pairs)))
}

#let _eval_value(value) = {
  if type(value) == dictionary and "re" in value {
    (re: value.re, im: value.at("im", default: 0.0))
  } else {
    value
  }
}
#let _eval_values(engine, values) = values.map(pair => (_expr_bytes(engine, pair.at(0)), _eval_value(pair.at(1))))
#let _evaluate(engine, expr, values: ()) = cbor(engine.plugin.evaluate(cbor.encode((expr: _expr_bytes(engine, expr), values: _eval_values(engine, values)))))

#let _expr_array(engine, values) = {
  let value = values
  if type(value) == bytes { return value }
  if type(value) == content {
    let kind = repr(value.func())
    let fields = value.fields()
    if kind == "equation" { return _expr_array(engine, fields.body) }
    if kind == "vec" and "children" in fields {
      return fields.children.map(child => _expr_bytes(engine, child))
    }
  }
  if type(value) == array { return value.map(item => _expr_bytes(engine, item)) }
  ( _expr_bytes(engine, value), )
}

#let _solve_linear(engine, system, variables) = cbor(engine.plugin.solve_linear(cbor.encode((
  system: _expr_array(engine, system),
  variables: _expr_array(engine, variables),
))))
#let _nsolve(engine, expr, var, init, prec: 1e-4, max-iterations: 1000) = cbor(engine.plugin.nsolve(cbor.encode((
  expr: _expr_bytes(engine, expr),
  var: _expr_bytes(engine, var),
  init: init,
  prec: prec,
  max-iterations: max-iterations,
))))
#let _nsolve_system(engine, system, variables, init, prec: 1e-4, max-iterations: 1000) = cbor(engine.plugin.nsolve_system(cbor.encode((
  system: _expr_array(engine, system),
  variables: _expr_array(engine, variables),
  init: init,
  prec: prec,
  max-iterations: max-iterations,
))))

#let _matrix_rows(engine, value) = {
  if type(value) == content {
    let kind = repr(value.func())
    let fields = value.fields()
    if kind == "equation" { return _matrix_rows(engine, fields.body) }
    if kind == "mat" and "rows" in fields {
      return fields.rows.map(row => row.map(cell => _expr_bytes(engine, cell)))
    }
    if kind == "vec" and "children" in fields {
      return fields.children.map(cell => (_expr_bytes(engine, cell),))
    }
  }
  if type(value) == array {
    if value.len() == 0 { return () }
    if type(value.first()) == array {
      return value.map(row => row.map(cell => _expr_bytes(engine, cell)))
    }
    return value.map(cell => (_expr_bytes(engine, cell),))
  }
  ((_expr_bytes(engine, value),),)
}

#let _matrix(engine, value) = {
  if type(value) == bytes { value } else { engine.plugin.matrix_from_nested(cbor.encode(_matrix_rows(engine, value))) }
}
#let _matrix_source(value) = {
  if type(value) == bytes { return true }
  if type(value) == content {
    let kind = repr(value.func())
    let fields = value.fields()
    if kind == "equation" { return _matrix_source(fields.body) }
    return kind == "mat" or kind == "vec"
  }
  type(value) == array and value.len() > 0 and type(value.first()) == array
}
#let _vec(engine, values) = {
  if type(values) == bytes or type(values) == content { return _matrix(engine, values) }
  engine.plugin.matrix_vec(cbor.encode(values.map(value => _expr_bytes(engine, value))))
}
#let _identity(engine, n) = engine.plugin.matrix_identity(cbor.encode(n))
#let _eye(engine, diag) = engine.plugin.matrix_eye(_atom_array(engine, diag))
#let _matrix_add(engine, lhs, rhs) = engine.plugin.matrix_add(_matrix(engine, lhs), _matrix(engine, rhs))
#let _matrix_sub(engine, lhs, rhs) = engine.plugin.matrix_sub(_matrix(engine, lhs), _matrix(engine, rhs))
#let _matrix_mul(engine, lhs, rhs) = {
  let rhs = if _matrix_source(rhs) { _matrix(engine, rhs) } else { _expr_bytes(engine, rhs) }
  engine.plugin.matrix_mul(_matrix(engine, lhs), rhs)
}
#let _matrix_div_scalar(engine, lhs, rhs) = engine.plugin.matrix_div_scalar(_matrix(engine, lhs), _expr_bytes(engine, rhs))
#let _transpose(engine, matrix) = engine.plugin.transpose(_matrix(engine, matrix))
#let _det(engine, matrix) = engine.plugin.det(_matrix(engine, matrix))
#let _inv(engine, matrix) = engine.plugin.inv(_matrix(engine, matrix))
#let _matrix_solve(engine, A, b) = engine.plugin.matrix_solve(_matrix(engine, A), _matrix(engine, b))
#let _matrix_solve_any(engine, A, b) = engine.plugin.matrix_solve_any(_matrix(engine, A), _matrix(engine, b))
#let _row_reduce(engine, matrix, max-col: none) = {
  let request = (matrix: _matrix(engine, matrix))
  if max-col != none { request.insert("max-col", max-col) }
  cbor(engine.plugin.row_reduce(cbor.encode(request)))
}
#let _augment(engine, lhs, rhs) = engine.plugin.augment(_matrix(engine, lhs), _matrix(engine, rhs))
#let _split_col(engine, matrix, index) = cbor(engine.plugin.split_col(cbor.encode((matrix: _matrix(engine, matrix), index: index))))
#let _primitive_part(engine, matrix) = engine.plugin.primitive_part(_matrix(engine, matrix))
#let _content(engine, matrix) = engine.plugin.content(_matrix(engine, matrix))
#let _matrix_at(engine, matrix, row, col) = engine.plugin.matrix_at(cbor.encode((matrix: _matrix(engine, matrix), row: row, col: col)))
#let _matrix_shape(engine, matrix) = cbor(engine.plugin.matrix_shape(_matrix(engine, matrix)))

#let _add(engine, ..terms) = engine.plugin.add(_atom_array(engine, terms.pos()))
#let _mul(engine, ..factors) = engine.plugin.mul(_atom_array(engine, factors.pos()))
#let _neg(engine, expr) = engine.plugin.neg(_expr_bytes(engine, expr))
#let _sub(engine, lhs, rhs) = engine.plugin.sub(_expr_bytes(engine, lhs), _expr_bytes(engine, rhs))
#let _div(engine, lhs, rhs) = engine.plugin.div(_expr_bytes(engine, lhs), _expr_bytes(engine, rhs))
#let _pow(engine, base, exp) = engine.plugin.power(_expr_bytes(engine, base), _expr_bytes(engine, exp))

/// Create a Symbolica engine record backed by the WebAssembly plugin.
///
/// The returned dictionary contains the full API. The top-level functions below
/// are convenience wrappers around `init()` with default settings.
///
/// ```example
/// #let sym = init(namespace: "physics")
/// #let v = sym.var
/// #let render = sym.to-symbolica
/// #raw(render(v("x"), namespaces: true))
/// ```
///
/// -> dictionary
#let init(
  /// Default namespace used for symbols parsed from Typst math or strings.
  /// -> str
  namespace: "typst",
  /// WebAssembly plugin path passed to Typst's `plugin` constructor.
  /// -> str
  source: "symbolica.wasm",
  /// Parsely grammar used by `math` and `from-math`.
  /// -> dictionary
  grammar: _default_grammar,
) = {
  let engine = (
    plugin: plugin(source),
    grammar: grammar,
    namespace: namespace,
  )

  (
    math: (eqn, grammar: none, namespace: none) => _from_math(engine, eqn, grammar: grammar, namespace: namespace),
    atom: value => _expr_bytes(engine, value),
    symbol: (name, namespace: none) => _symbol(engine, name, namespace: namespace),
    var: (name, namespace: none) => _var(engine, name, namespace: namespace),
    wild: (name, level: 1, namespace: none) => _wild(engine, name, level: level, namespace: namespace),
    dummy: (name, level: 1, namespace: none) => _dummy(engine, name, level: level, namespace: namespace),
    from-math: (eqn, grammar: none, namespace: none) => _from_math(engine, eqn, grammar: grammar, namespace: namespace),
    array-tree: (eqn, grammar: none) => _array_tree(engine, eqn, grammar: grammar),
    canonical: (expr, namespaces: false) => _canonical(engine, expr, namespaces: namespaces),
    to-symbolica: (expr, namespaces: false) => _canonical(engine, expr, namespaces: namespaces),
    to-typst-source: expr => _to_typst_source(engine, expr),
    to-typst: (expr, block: false) => _to_typst(engine, expr, block: block),
    to-latex: expr => _to_latex(engine, expr),
    simplify: expr => _simplify(engine, expr),
    expand: expr => _expand(engine, expr),
    factor: expr => _factor(engine, expr),
    derivative: (expr, var) => _derivative(engine, expr, var),
    series: (expr, var, expansion-point, depth, depth-denom: 1, depth-is-absolute: true) => _series(engine, expr, var, expansion-point, depth, depth-denom: depth-denom, depth-is-absolute: depth-is-absolute),
    rule: (pattern, rhs, non-greedy-wildcards: (), min-level: 0, max-level: none, level-range: none, level-is-tree-depth: false, partial: true, allow-new-wildcards-on-rhs: false, rhs-cache-size: 100) => _rule(engine, pattern, rhs, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size),
    replace: (expr, pattern, rhs, repeat: false, once: false, bottom-up: false, nested: false, non-greedy-wildcards: (), min-level: 0, max-level: none, level-range: none, level-is-tree-depth: false, partial: true, allow-new-wildcards-on-rhs: false, rhs-cache-size: 100) => _replace(engine, expr, pattern, rhs, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size),
    replace-multiple: (expr, rules, repeat: false, once: false, bottom-up: false, nested: false) => _replace_multiple(engine, expr, rules, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested),
    replace-wildcards: (pattern, replacements) => _replace_wildcards(engine, pattern, replacements),
    replace-all: (expr, pattern, replacement) => _replace_all(engine, expr, pattern, replacement),
    evaluate: (expr, values: ()) => _evaluate(engine, expr, values: values),
    solve-linear: (system, variables) => _solve_linear(engine, system, variables),
    nsolve: (expr, var, init, prec: 1e-4, max-iterations: 1000) => _nsolve(engine, expr, var, init, prec: prec, max-iterations: max-iterations),
    nsolve-system: (system, variables, init, prec: 1e-4, max-iterations: 1000) => _nsolve_system(engine, system, variables, init, prec: prec, max-iterations: max-iterations),
    matrix: value => _matrix(engine, value),
    vec: values => _vec(engine, values),
    identity: n => _identity(engine, n),
    eye: diag => _eye(engine, diag),
    matrix-add: (lhs, rhs) => _matrix_add(engine, lhs, rhs),
    matrix-sub: (lhs, rhs) => _matrix_sub(engine, lhs, rhs),
    matrix-mul: (lhs, rhs) => _matrix_mul(engine, lhs, rhs),
    matrix-div-scalar: (lhs, rhs) => _matrix_div_scalar(engine, lhs, rhs),
    transpose: matrix => _transpose(engine, matrix),
    det: matrix => _det(engine, matrix),
    inv: matrix => _inv(engine, matrix),
    matrix-solve: (A, b) => _matrix_solve(engine, A, b),
    matrix-solve-any: (A, b) => _matrix_solve_any(engine, A, b),
    row-reduce: (matrix, max-col: none) => _row_reduce(engine, matrix, max-col: max-col),
    augment: (lhs, rhs) => _augment(engine, lhs, rhs),
    split-col: (matrix, index) => _split_col(engine, matrix, index),
    primitive-part: matrix => _primitive_part(engine, matrix),
    content: matrix => _content(engine, matrix),
    matrix-at: (matrix, row, col) => _matrix_at(engine, matrix, row, col),
    matrix-shape: matrix => _matrix_shape(engine, matrix),
    add: (..terms) => _add(engine, ..terms),
    mul: (..factors) => _mul(engine, ..factors),
    neg: expr => _neg(engine, expr),
    sub: (lhs, rhs) => _sub(engine, lhs, rhs),
    div: (lhs, rhs) => _div(engine, lhs, rhs),
    pow: (base, exp) => _pow(engine, base, exp),
  )
}
#let _default_engine = init()

/// Parse Typst math content into an opaque Symbolica atom payload.
///
/// ```example
/// #let expr = math($x + 1$)
/// #to-typst(expr)
/// ```
///
/// -> bytes
#let math(
  /// Math content, normally written as `$...$`.
  /// -> content
  eqn,
  /// Optional Parsely grammar override.
  /// -> dictionary | none
  grammar: none,
  /// Optional namespace override for parsed symbols.
  /// -> str | none
  namespace: none,
) = (_default_engine.math)(eqn, grammar: grammar, namespace: namespace)

/// Convert a Typst value or math expression into an atom payload.
///
/// ```example
/// #to-typst(atom("x"))
/// ```
///
/// -> bytes
#let atom(value) = (_default_engine.atom)(value)

/// Construct a symbol atom with an optional namespace override.
///
/// ```example
/// #to-typst(symbol("x"))
/// ```
///
/// -> bytes
#let symbol(name, namespace: none) = (_default_engine.symbol)(name, namespace: namespace)

/// Alias for `symbol` for variables used in expressions, solves, and derivatives.
///
/// ```example
/// #to-typst(var("x"))
/// ```
///
/// -> bytes
#let var(name, namespace: none) = (_default_engine.var)(name, namespace: namespace)

/// Construct a wildcard symbol by appending underscore levels to `name`.
///
/// ```example
/// #raw(to-symbolica(wild("a")))
/// ```
///
/// -> bytes
#let wild(name, level: 1, namespace: none) = (_default_engine.wild)(name, level: level, namespace: namespace)

/// Construct an RHS-only wildcard marker. This is wildcard syntax, not a fresh symbol generator.
///
/// ```example
/// #raw(to-symbolica(dummy("fresh")))
/// ```
///
/// -> bytes
#let dummy(name, level: 1, namespace: none) = (_default_engine.dummy)(name, level: level, namespace: namespace)

/// Parse Typst math content with an optional grammar and namespace override.
///
/// ```example
/// #to-typst(from-math($(x + 1)^2$))
/// ```
///
/// -> bytes
#let from-math(eqn, grammar: none, namespace: none) = (_default_engine.from-math)(eqn, grammar: grammar, namespace: namespace)

/// Render the Parsely tree used for parsing a math expression.
///
/// ```example
/// #array-tree($(x + 1)^2$)
/// ```
///
/// -> content
#let array-tree(eqn, grammar: none) = (_default_engine.array-tree)(eqn, grammar: grammar)

/// Render an atom or matrix payload as Symbolica text.
///
/// ```example
/// #raw(canonical(math($x + 1$)))
/// ```
///
/// -> str
#let canonical(expr, namespaces: false) = (_default_engine.canonical)(expr, namespaces: namespaces)

/// Alias for `canonical`.
///
/// ```example
/// #raw(to-symbolica(math($x + 1$)))
/// ```
///
/// -> str
#let to-symbolica(expr, namespaces: false) = (_default_engine.to-symbolica)(expr, namespaces: namespaces)

/// Render an atom or matrix payload as Typst math source.
///
/// ```example
/// #raw(to-typst-source(math($x + 1$)))
/// ```
///
/// -> str
#let to-typst-source(expr) = (_default_engine.to-typst-source)(expr)

/// Render an atom or matrix payload as Typst math content.
///
/// ```example
/// #to-typst(math($x + 1$))
/// ```
///
/// -> content
#let to-typst(expr, block: false) = (_default_engine.to-typst)(expr, block: block)

/// Render an atom or matrix payload as LaTeX source.
///
/// ```example
/// #raw(to-latex(math($x^2 + 1$)))
/// ```
///
/// -> str
#let to-latex(expr) = (_default_engine.to-latex)(expr)

/// Normalize an expression payload without changing its mathematical value.
///
/// ```example
/// #to-typst(simplify(math($x + 0$)))
/// ```
///
/// -> bytes
#let simplify(expr) = (_default_engine.simplify)(expr)

/// Expand products and powers through Symbolica's polynomial expansion.
///
/// ```example
/// #to-typst(expand(math($(x + 1)^2$)))
/// ```
///
/// -> bytes
#let expand(expr) = (_default_engine.expand)(expr)

/// Factor an expression payload.
///
/// ```example
/// #to-typst(factor(math($x^2 + 2 x + 1$)))
/// ```
///
/// -> bytes
#let factor(expr) = (_default_engine.factor)(expr)

/// Differentiate `expr` with respect to `var`.
///
/// ```example
/// #let x = var("x")
/// #to-typst(derivative(math($(x + 1)^2$), x))
/// ```
///
/// -> bytes
#let derivative(expr, var) = (_default_engine.derivative)(expr, var)

/// Compute a series expansion around `expansion-point`.
///
/// ```example
/// #let sym = init(namespace: "symbolica")
/// #let m = sym.math
/// #let v = sym.var
/// #let ser = sym.series
/// #let render = sym.to-typst
/// #render(ser(m($cos(x)/(x + 1)$), v("x"), 0, 3))
/// ```
///
/// -> bytes
#let series(expr, var, expansion-point, depth, depth-denom: 1, depth-is-absolute: true) = (
  _default_engine.series)(expr, var, expansion-point, depth, depth-denom: depth-denom, depth-is-absolute: depth-is-absolute)

/// Build a reusable replacement rule for `replace-multiple`.
///
/// ```example
/// #let r = rule(math($f("a_")$), math($g("a_")$))
/// #to-typst(replace-multiple(math($f(x)$), (r,)))
/// ```
///
/// -> dictionary
#let rule(pattern, rhs, non-greedy-wildcards: (), min-level: 0, max-level: none, level-range: none, level-is-tree-depth: false, partial: true, allow-new-wildcards-on-rhs: false, rhs-cache-size: 100) = (
  _default_engine.rule)(pattern, rhs, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size)

/// Replace subexpressions matching `pattern` with `rhs`.
///
/// ```example
/// #to-typst(replace(math($f(x, y)$), math($f("a_", "b_")$), math($g("b_", "a_")$)))
/// ```
///
/// -> bytes
#let replace(expr, pattern, rhs, repeat: false, once: false, bottom-up: false, nested: false, non-greedy-wildcards: (), min-level: 0, max-level: none, level-range: none, level-is-tree-depth: false, partial: true, allow-new-wildcards-on-rhs: false, rhs-cache-size: 100) = (
  _default_engine.replace)(expr, pattern, rhs, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size)

/// Apply multiple replacement rules in one pass.
///
/// ```example
/// #let r1 = rule(math($f("a_")$), math($h("a_")$))
/// #let r2 = rule(var("x"), var("z"))
/// #to-typst(replace-multiple(math($f(x) + x$), (r1, r2)))
/// ```
///
/// -> bytes
#let replace-multiple(expr, rules, repeat: false, once: false, bottom-up: false, nested: false) = (
  _default_engine.replace-multiple)(expr, rules, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested)

/// Replace wildcard placeholders inside a pattern.
///
/// ```example
/// #to-typst(replace-wildcards(math($k("a_")$), ((wild("a"), math($x + 1$)),)))
/// ```
///
/// -> bytes
#let replace-wildcards(pattern, replacements) = (_default_engine.replace-wildcards)(pattern, replacements)

/// Compatibility alias for `replace` with default replacement settings.
///
/// ```example
/// #to-typst(replace-all(math($x + y$), var("y"), var("z")))
/// ```
///
/// -> bytes
#let replace-all(expr, pattern, replacement) = (_default_engine.replace-all)(expr, pattern, replacement)

/// Evaluate an expression with optional real or complex values.
///
/// ```example
/// #let x = var("x")
/// #let y = var("y")
/// #repr(evaluate(math($x^2 + y$), values: ((x, 2.0), (y, 3.0))))
/// ```
///
/// -> dictionary
#let evaluate(expr, values: ()) = (_default_engine.evaluate)(expr, values: values)

/// Solve a linear system exactly for `variables`.
///
/// ```example
/// #let x = var("x")
/// #let y = var("y")
/// #let sol = solve-linear((math($2 x + y - 5$), math($x - y - 1$)), (x, y))
/// #sol.map(to-typst).join[, ]
/// ```
///
/// -> array
#let solve-linear(system, variables) = (_default_engine.solve-linear)(system, variables)

/// Numerically solve a univariate expression near `init`.
///
/// ```example
/// #let x = var("x")
/// #repr(nsolve(math($x^2 - 2$), x, 1.0))
/// ```
///
/// -> float
#let nsolve(expr, var, init, prec: 1e-4, max-iterations: 1000) = (
  _default_engine.nsolve)(expr, var, init, prec: prec, max-iterations: max-iterations)

/// Numerically solve a system of expressions near `init`.
///
/// ```example
/// #let x = var("x")
/// #let y = var("y")
/// #repr(nsolve-system((math($x^2 + y - 3$), math($x - y$)), (x, y), (1.0, 1.0)))
/// ```
///
/// -> array
#let nsolve-system(system, variables, init, prec: 1e-4, max-iterations: 1000) = (
  _default_engine.nsolve-system)(system, variables, init, prec: prec, max-iterations: max-iterations)

/// Convert Typst math `mat(...)`, `vec(...)`, or nested arrays into a matrix payload.
///
/// ```example
/// #to-typst(matrix($mat(1, 2; 3, 4)$))
/// ```
///
/// -> bytes
#let matrix(value) = (_default_engine.matrix)(value)

/// Convert values or Typst math `vec(...)` into a column-vector matrix payload.
///
/// ```example
/// #to-typst(vec((1, 2)))
/// ```
///
/// -> bytes
#let vec(values) = (_default_engine.vec)(values)

/// Create an `n` by `n` identity matrix payload.
///
/// ```example
/// #to-typst(identity(2))
/// ```
///
/// -> bytes
#let identity(n) = (_default_engine.identity)(n)

/// Create a diagonal matrix payload from diagonal entries.
///
/// ```example
/// #to-typst(eye((1, 2)))
/// ```
///
/// -> bytes
#let eye(diag) = (_default_engine.eye)(diag)

/// Add two matrix payloads.
///
/// ```example
/// #to-typst(matrix-add(matrix(((1, 2), (3, 4))), identity(2)))
/// ```
///
/// -> bytes
#let matrix-add(lhs, rhs) = (_default_engine.matrix-add)(lhs, rhs)

/// Subtract two matrix payloads.
///
/// ```example
/// #to-typst(matrix-sub(matrix(((1, 2), (3, 4))), identity(2)))
/// ```
///
/// -> bytes
#let matrix-sub(lhs, rhs) = (_default_engine.matrix-sub)(lhs, rhs)

/// Multiply matrices, or multiply a matrix by a scalar expression.
///
/// ```example
/// #to-typst(matrix-mul(matrix(((1, 2), (3, 4))), identity(2)))
/// ```
///
/// -> bytes
#let matrix-mul(lhs, rhs) = (_default_engine.matrix-mul)(lhs, rhs)

/// Divide a matrix by a scalar expression.
///
/// ```example
/// #to-typst(matrix-div-scalar(matrix(((2, 4), (6, 8))), 2))
/// ```
///
/// -> bytes
#let matrix-div-scalar(lhs, rhs) = (_default_engine.matrix-div-scalar)(lhs, rhs)

/// Transpose a matrix payload.
///
/// ```example
/// #to-typst(transpose(matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> bytes
#let transpose(matrix) = (_default_engine.transpose)(matrix)

/// Compute the determinant of a matrix as an atom payload.
///
/// ```example
/// #to-typst(det(matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> bytes
#let det(matrix) = (_default_engine.det)(matrix)

/// Compute the inverse of a matrix payload.
///
/// ```example
/// #to-typst(inv(matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> bytes
#let inv(matrix) = (_default_engine.inv)(matrix)

/// Solve `A x = b` exactly.
///
/// ```example
/// #let A = matrix($mat(2, 1; 1, -1)$)
/// #let b = vec((5, 1))
/// #to-typst(matrix-solve(A, b))
/// ```
///
/// -> bytes
#let matrix-solve(A, b) = (_default_engine.matrix-solve)(A, b)

/// Solve `A x = b` exactly, allowing any solution for underdetermined systems.
///
/// ```example
/// #let A = matrix($mat(2, 1; 1, -1)$)
/// #let b = vec((5, 1))
/// #to-typst(matrix-solve-any(A, b))
/// ```
///
/// -> bytes
#let matrix-solve-any(A, b) = (_default_engine.matrix-solve-any)(A, b)

/// Row-reduce a matrix and return `(matrix: ..., rank: ...)`.
///
/// ```example
/// #let rr = row-reduce(matrix(((1, 2), (3, 4))))
/// rank #rr.rank: #to-typst(rr.matrix)
/// ```
///
/// -> dictionary
#let row-reduce(matrix, max-col: none) = (_default_engine.row-reduce)(matrix, max-col: max-col)

/// Horizontally augment two matrices.
///
/// ```example
/// #to-typst(augment(identity(2), matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> bytes
#let augment(lhs, rhs) = (_default_engine.augment)(lhs, rhs)

/// Split a matrix at a column index.
///
/// ```example
/// #let aug = augment(identity(2), matrix(((1, 2), (3, 4))))
/// #let parts = split-col(aug, 2)
/// #to-typst(parts.at(0)) | #to-typst(parts.at(1))
/// ```
///
/// -> array
#let split-col(matrix, index) = (_default_engine.split-col)(matrix, index)

/// Compute the primitive part of a matrix payload.
///
/// ```example
/// #let x = var("x")
/// #let P = matrix(((mul(2, x), mul(4, x)), (mul(6, x), mul(8, x))))
/// #to-typst(primitive-part(P))
/// ```
///
/// -> bytes
#let primitive-part(matrix) = (_default_engine.primitive-part)(matrix)

/// Compute the content of a matrix as an atom payload.
///
/// ```example
/// #let x = var("x")
/// #let P = matrix(((mul(2, x), mul(4, x)), (mul(6, x), mul(8, x))))
/// #to-typst(content(P))
/// ```
///
/// -> bytes
#let content(matrix) = (_default_engine.content)(matrix)

/// Read a matrix entry as an atom payload using zero-based indices.
///
/// ```example
/// #let A = matrix(((1, 2), (3, 4)))
/// #to-typst(matrix-at(A, 0, 1))
/// ```
///
/// -> bytes
#let matrix-at(matrix, row, col) = (_default_engine.matrix-at)(matrix, row, col)

/// Return the matrix shape as `(rows, columns)`.
///
/// ```example
/// #repr(matrix-shape(matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> array
#let matrix-shape(matrix) = (_default_engine.matrix-shape)(matrix)

/// Add one or more expression payloads or Typst values.
///
/// ```example
/// #to-typst(add("x", 1, "y"))
/// ```
///
/// -> bytes
#let add(..terms) = (_default_engine.add)(..terms)

/// Multiply one or more expression payloads or Typst values.
///
/// ```example
/// #to-typst(mul(2, "x", "y"))
/// ```
///
/// -> bytes
#let mul(..factors) = (_default_engine.mul)(..factors)

/// Negate an expression payload.
///
/// ```example
/// #to-typst(neg("x"))
/// ```
///
/// -> bytes
#let neg(expr) = (_default_engine.neg)(expr)

/// Subtract two expression payloads or Typst values.
///
/// ```example
/// #to-typst(sub("x", "y"))
/// ```
///
/// -> bytes
#let sub(lhs, rhs) = (_default_engine.sub)(lhs, rhs)

/// Divide two expression payloads or Typst values.
///
/// ```example
/// #to-typst(div(1, "x"))
/// ```
///
/// -> bytes
#let div(lhs, rhs) = (_default_engine.div)(lhs, rhs)

/// Raise `base` to `exp`.
///
/// ```example
/// #to-typst(pow("x", 3))
/// ```
///
/// -> bytes
#let pow(base, exp) = (_default_engine.pow)(base, exp)
