#import "@preview/parsely:0.1.0"

#let default-grammar = parsely.common.arithmetic

#let init(source: "symbolica.wasm", grammar: default-grammar) = (
  plugin: plugin(source),
  grammar: grammar,
)

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

#let _node-to-symbolica(node) = {
  let (head, args, slots) = node
  if head == "add" {
    args.join("+")
  } else if head == "sub" {
    "(" + args.at(0) + ")-(" + args.at(1) + ")"
  } else if head == "mul" {
    args.join("*")
  } else if head == "frac" {
    "(" + slots.num + ")/(" + slots.denom + ")"
  } else if head == "pow" {
    let base = if "base" in slots { slots.base } else { args.at(0) }
    let exp = if "exp" in slots { slots.exp } else { args.at(1) }
    "(" + base + ")^(" + exp + ")"
  } else if head == "neg" {
    "-(" + args.at(0) + ")"
  } else if head == "group" {
    "(" + slots.expr + ")"
  } else if head == "sqrt" {
    "sqrt(" + slots.radicand + ")"
  } else {
    head + "(" + args.join(",") + ")"
  }
}

#let from-math(eqn, grammar: default-grammar) = {
  let parsed = parsely.parse(eqn, grammar)
  parsely.walk(parsed.tree, post: _node-to-symbolica, leaf: _leaf)
}

#let canonical(engine, expr) = str(engine.plugin.canonical(bytes(expr)))
#let simplify(engine, expr) = str(engine.plugin.simplify_expr(bytes(expr)))
#let expand(engine, expr) = str(engine.plugin.expand(bytes(expr)))
#let factor(engine, expr) = str(engine.plugin.factor(bytes(expr)))
#let derivative(engine, expr, var) = str(engine.plugin.derivative(bytes(expr), bytes(var)))
#let replace-all(engine, expr, pattern, replacement) = {
  str(engine.plugin.replace_all(bytes(expr), bytes(pattern), bytes(replacement)))
}
#let to-typst(engine, expr) = str(engine.plugin.to_typst(bytes(expr)))
#let to-latex(engine, expr) = str(engine.plugin.to_latex(bytes(expr)))

#let math(engine, eqn) = from-math(eqn, grammar: engine.grammar)
