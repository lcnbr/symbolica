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

#let _ast_bytes(eqn, grammar) = {
  let parsed = parsely.parse(_trim_math(eqn), grammar)
  let tree = parsely.walk(parsed.tree, post: _node_to_ast, leaf: _leaf)
  cbor.encode(tree)
}

#let _from_math(engine, eqn, grammar: none) = {
  let grammar = if grammar == none { engine.grammar } else { grammar }
  engine.plugin.from_ast(_ast_bytes(eqn, grammar))
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

#let _var(engine, name) = engine.plugin.symbol(cbor.encode(name))

#let _expr_bytes(engine, expr) = {
  if type(expr) == bytes {
    expr
  } else if type(expr) == content {
    _from_math(engine, expr)
  } else {
    engine.plugin.from_ast(cbor.encode(expr))
  }
}

#let _atom_array(engine, values) = cbor.encode(values.map(value => _expr_bytes(engine, value)))

#let _canonical(engine, expr) = str(engine.plugin.canonical(_expr_bytes(engine, expr)))
#let _to_typst_source(engine, expr) = str(engine.plugin.to_typst(_expr_bytes(engine, expr)))
#let _to_typst(engine, expr, block: false) = {
  let eqn = eval(_to_typst_source(engine, expr), mode: "math")
  if block { _typst_math.equation(eqn.body, block: true) } else { eqn }
}
#let _to_latex(engine, expr) = str(engine.plugin.to_latex(_expr_bytes(engine, expr)))

#let _simplify(engine, expr) = engine.plugin.simplify_expr(_expr_bytes(engine, expr))
#let _expand(engine, expr) = engine.plugin.expand(_expr_bytes(engine, expr))
#let _factor(engine, expr) = engine.plugin.factor(_expr_bytes(engine, expr))
#let _derivative(engine, expr, var) = {
  engine.plugin.derivative(_expr_bytes(engine, expr), _expr_bytes(engine, var))
}
#let _replace_all(engine, expr, pattern, replacement) = {
  engine.plugin.replace_all(
    _expr_bytes(engine, expr),
    _expr_bytes(engine, pattern),
    _expr_bytes(engine, replacement),
  )
}

#let _add(engine, ..terms) = engine.plugin.add(_atom_array(engine, terms.pos()))
#let _mul(engine, ..factors) = engine.plugin.mul(_atom_array(engine, factors.pos()))
#let _neg(engine, expr) = engine.plugin.neg(_expr_bytes(engine, expr))
#let _sub(engine, lhs, rhs) = engine.plugin.sub(_expr_bytes(engine, lhs), _expr_bytes(engine, rhs))
#let _div(engine, lhs, rhs) = engine.plugin.div(_expr_bytes(engine, lhs), _expr_bytes(engine, rhs))
#let _pow(engine, base, exp) = engine.plugin.power(_expr_bytes(engine, base), _expr_bytes(engine, exp))

#let init(source: "symbolica.wasm", grammar: _default_grammar) = {
  let engine = (
    plugin: plugin(source),
    grammar: grammar,
  )

  (
    math: eqn => _from_math(engine, eqn),
    atom: value => _expr_bytes(engine, value),
    var: name => _var(engine, name),
    from-math: (eqn, grammar: none) => _from_math(engine, eqn, grammar: grammar),
    array-tree: (eqn, grammar: none) => _array_tree(engine, eqn, grammar: grammar),
    canonical: expr => _canonical(engine, expr),
    to-symbolica: expr => _canonical(engine, expr),
    to-typst-source: expr => _to_typst_source(engine, expr),
    to-typst: (expr, block: false) => _to_typst(engine, expr, block: block),
    to-latex: expr => _to_latex(engine, expr),
    simplify: expr => _simplify(engine, expr),
    expand: expr => _expand(engine, expr),
    factor: expr => _factor(engine, expr),
    derivative: (expr, var) => _derivative(engine, expr, var),
    replace-all: (expr, pattern, replacement) => _replace_all(engine, expr, pattern, replacement),
    add: (..terms) => _add(engine, ..terms),
    mul: (..factors) => _mul(engine, ..factors),
    neg: expr => _neg(engine, expr),
    sub: (lhs, rhs) => _sub(engine, lhs, rhs),
    div: (lhs, rhs) => _div(engine, lhs, rhs),
    pow: (base, exp) => _pow(engine, base, exp),
  )
}
#let _default_engine = init()

#let math = _default_engine.math
#let atom = _default_engine.atom
#let var = _default_engine.var
#let from-math = _default_engine.from-math
#let array-tree = _default_engine.array-tree
#let canonical = _default_engine.canonical
#let to-symbolica = _default_engine.to-symbolica
#let to-typst-source = _default_engine.to-typst-source
#let to-typst = _default_engine.to-typst
#let to-latex = _default_engine.to-latex
#let simplify = _default_engine.simplify
#let expand = _default_engine.expand
#let factor = _default_engine.factor
#let derivative = _default_engine.derivative
#let replace-all = _default_engine.replace-all
#let add = _default_engine.add
#let mul = _default_engine.mul
#let neg = _default_engine.neg
#let sub = _default_engine.sub
#let div = _default_engine.div
#let pow = _default_engine.pow
