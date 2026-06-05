#import "@preview/tidy:0.4.3"
#import "lib.typ" as symbolica
#import "lib.typ": *

#set document(title: "Symbolica for Typst")
#set page(margin: 22mm)
#set text(font: "Libertinus Serif", size: 10.5pt)
#set heading(numbering: "1.1")

#let example-preamble = (
  "#let _sym = symbolica.init()",
  "#let init = symbolica.init",
  "#let math = _sym.math",
  "#let atom = _sym.atom",
  "#let symbol = _sym.symbol",
  "#let var = _sym.var",
  "#let wild = _sym.wild",
  "#let dummy = _sym.dummy",
  "#let from-math = _sym.from-math",
  "#let array-tree = _sym.array-tree",
  "#let canonical = _sym.canonical",
  "#let to-symbolica = _sym.to-symbolica",
  "#let to-typst-source = _sym.to-typst-source",
  "#let to-typst = _sym.to-typst",
  "#let to-latex = _sym.to-latex",
  "#let simplify = _sym.simplify",
  "#let expand = _sym.expand",
  "#let factor = _sym.factor",
  "#let derivative = _sym.derivative",
  "#let series = _sym.series",
  "#let rule = _sym.rule",
  "#let replace = _sym.replace",
  "#let replace-multiple = _sym.replace-multiple",
  "#let replace-wildcards = _sym.replace-wildcards",
  "#let replace-all = _sym.replace-all",
  "#let evaluate = _sym.evaluate",
  "#let solve-linear = _sym.solve-linear",
  "#let nsolve = _sym.nsolve",
  "#let nsolve-system = _sym.nsolve-system",
  "#let matrix = _sym.matrix",
  "#let vec = _sym.vec",
  "#let identity = _sym.identity",
  "#let eye = _sym.eye",
  "#let matrix-add = _sym.matrix-add",
  "#let matrix-sub = _sym.matrix-sub",
  "#let matrix-mul = _sym.matrix-mul",
  "#let matrix-div-scalar = _sym.matrix-div-scalar",
  "#let transpose = _sym.transpose",
  "#let det = _sym.det",
  "#let inv = _sym.inv",
  "#let matrix-solve = _sym.matrix-solve",
  "#let matrix-solve-any = _sym.matrix-solve-any",
  "#let row-reduce = _sym.row-reduce",
  "#let augment = _sym.augment",
  "#let split-col = _sym.split-col",
  "#let primitive-part = _sym.primitive-part",
  "#let content = _sym.content",
  "#let matrix-at = _sym.matrix-at",
  "#let matrix-shape = _sym.matrix-shape",
  "#let add = _sym.add",
  "#let mul = _sym.mul",
  "#let neg = _sym.neg",
  "#let sub = _sym.sub",
  "#let div = _sym.div",
  "#let pow = _sym.pow",
).join("\n") + "\n"

#let docs = tidy.parse-module(read("lib.typ"), name: "symbolica", scope: (symbolica: symbolica), preamble: example-preamble)
#let result(body) = block(stroke: luma(205), inset: 8pt, radius: 3pt, width: 100%, body)

= Symbolica for Typst

`symbolica` exposes Symbolica's exact algebra engine to Typst through a WebAssembly plugin. Expressions and matrices are passed around as opaque byte payloads; use `to-typst` for document output and `to-symbolica` for portable text.

== Setup

```typst
#import "@local/symbolica:0.1.0": *

#let expr = math($(x + 1)^2$)
#to-typst(expand(expr))
```

For a source checkout, import the local library directly:

```typst
#import "lib.typ": *
```

== Worked API Examples

Each example shows the Typst code first, then the rendered output produced by the current plugin.

=== Engine, Parsing, and Symbols

```typst
#let sym = init(namespace: "physics")
#let smath = sym.math
#let svar = sym.var

#let x = var("x")
#let y = symbol("y")
#let a-wild = wild("a")
#let rhs-only = dummy("fresh")
#let expr = math($(x + 1)^2 + y$)
#let same = from-math($(x + 1)^2 + y$)
#let literal = atom("z")
#let namespaced = add(svar("x"), x)

expr: #to-typst(expr)
same: #to-typst(same)
literal: #to-typst(literal)
wildcards: #raw(to-symbolica(a-wild)) and #raw(to-symbolica(rhs-only))
namespaces: #raw(to-symbolica(namespaced, namespaces: true))
```

#let sym = init(namespace: "physics")
#let smath = sym.math
#let svar = sym.var
#let x = var("x")
#let y = symbol("y")
#let z = var("z")
#let a-wild = wild("a")
#let rhs-only = dummy("fresh")
#let expr = math($(x + 1)^2 + y$)
#let same = from-math($(x + 1)^2 + y$)
#let literal = atom("z")
#let namespaced = add(svar("x"), x)

#result[
expr: #to-typst(expr)\
same: #to-typst(same)\
literal: #to-typst(literal)\
wildcards: #raw(to-symbolica(a-wild)) and #raw(to-symbolica(rhs-only))\
namespaces: #raw(to-symbolica(namespaced, namespaces: true))
]

=== Parse Tree Inspection

```typst
#array-tree($(x + 1)^2$)
```

#result[
#array-tree($(x + 1)^2$)
]

=== Rendering

```typst
#let expr = math($(x + 1)^2 + y$)

canonical: #raw(canonical(expr))
to-symbolica: #raw(to-symbolica(expr))
to-typst-source: #raw(to-typst-source(expr))
to-typst: #to-typst(expr)
to-latex: #raw(to-latex(expr))
```

#result[
canonical: #raw(canonical(expr))\
to-symbolica: #raw(to-symbolica(expr))\
to-typst-source: #raw(to-typst-source(expr))\
to-typst: #to-typst(expr)\
to-latex: #raw(to-latex(expr))
]

=== Algebra and Calculus

```typst
#let p = math($x^2 + 2 x + 1$)
#let q = math($(x + 1) (y + 2)$)

simplify: #to-typst(simplify(p))
expand: #to-typst(expand(q))
factor: #to-typst(factor(p))
derivative: #to-typst(derivative(q, x))
series: #builtin.to-typst(builtin.series(builtin.math($cos(x)/(x + 1)$), builtin.var("x"), 0, 3))
```

#let p = math($x^2 + 2 x + 1$)
#let q = math($(x + 1) (y + 2)$)
#let builtin = init(namespace: "symbolica")
#let bmath = builtin.math
#let bvar = builtin.var
#let bseries = builtin.series
#let bto-typst = builtin.to-typst
#let sx = bvar("x")
#let ser = bseries(bmath($cos(x)/(x + 1)$), sx, 0, 3)

#result[
simplify: #to-typst(simplify(p))\
expand: #to-typst(expand(q))\
factor: #to-typst(factor(p))\
derivative: #to-typst(derivative(q, x))\
series: #bto-typst(ser)
]

=== Scalar Constructors and Arithmetic

```typst
#let arith = add(
  mul(2, x, y),
  neg(z),
  sub(x, y),
  div(1, x),
  pow(x, 3),
)
#to-typst(arith)
```

#let arith = add(
  mul(2, x, y),
  neg(z),
  sub(x, y),
  div(1, x),
  pow(x, 3),
)

#result[#to-typst(arith)]

=== Replacement

```typst
#let src = math($f(x, y) + x$)
#let swapped = replace(src, math($f("a_", "b_")$), math($g("b_", "a_")$))
#let r1 = rule(math($f("a_")$), math($h("a_")$))
#let r2 = rule(x, z)
#let multiple = replace-multiple(math($f(x) + x$), (r1, r2))
#let wild-replaced = replace-wildcards(math($k("a_")$), ((wild("a"), math($x + 1$)),))
#let all = replace-all(math($x + y$), y, z)

replace: #to-typst(swapped)
replace-multiple: #to-typst(multiple)
replace-wildcards: #to-typst(wild-replaced)
replace-all: #to-typst(all)
```

#let src = math($f(x, y) + x$)
#let swapped = replace(src, math($f("a_", "b_")$), math($g("b_", "a_")$))
#let r1 = rule(math($f("a_")$), math($h("a_")$))
#let r2 = rule(x, z)
#let multiple = replace-multiple(math($f(x) + x$), (r1, r2))
#let wild-replaced = replace-wildcards(math($k("a_")$), ((wild("a"), math($x + 1$)),))
#let all = replace-all(math($x + y$), y, z)

#result[
replace: #to-typst(swapped)\
replace-multiple: #to-typst(multiple)\
replace-wildcards: #to-typst(wild-replaced)\
replace-all: #to-typst(all)
]

=== Evaluation and Solving

```typst
#let value = evaluate(math($x^2 + y$), values: ((x, 2.0), (y, 3.0)))
#let complex = evaluate(math($x^2$), values: ((x, (re: 2.0, im: 1.0)),))
#let exact = solve-linear((math($2 x + y - 5$), math($x - y - 1$)), (x, y))
#let root = nsolve(math($x^2 - 2$), x, 1.0)
#let roots = nsolve-system((math($x^2 + y - 3$), math($x - y$)), (x, y), (1.0, 1.0))

value: #repr(value)
complex: #repr(complex)
solve-linear: #exact.map(to-typst).join[, ]
nsolve: #repr(root)
nsolve-system: #repr(roots)
```

#let value = evaluate(math($x^2 + y$), values: ((x, 2.0), (y, 3.0)))
#let complex = evaluate(math($x^2$), values: ((x, (re: 2.0, im: 1.0)),))
#let exact = solve-linear((math($2 x + y - 5$), math($x - y - 1$)), (x, y))
#let root = nsolve(math($x^2 - 2$), x, 1.0)
#let roots = nsolve-system((math($x^2 + y - 3$), math($x - y$)), (x, y), (1.0, 1.0))

#result[
value: #repr(value)\
complex: #repr(complex)\
solve-linear: #exact.map(to-typst).join[, ]\
nsolve: #repr(root)\
nsolve-system: #repr(roots)
]

=== Matrix Construction and Rendering

```typst
#let A = matrix($mat(2, 1; 1, -1)$)
#let B = matrix(((1, 2), (3, 4)))
#let b = vec((5, 1))
#let I = identity(2)
#let D = eye((1, 2))

A: #to-typst(A)
B: #to-typst(B)
b: #to-typst(b)
I: #to-typst(I)
D: #to-typst(D)
```

#let A = matrix($mat(2, 1; 1, -1)$)
#let B = matrix(((1, 2), (3, 4)))
#let b = vec((5, 1))
#let I = identity(2)
#let D = eye((1, 2))

#result[
A: #to-typst(A)\
B: #to-typst(B)\
b: #to-typst(b)\
I: #to-typst(I)\
D: #to-typst(D)
]

=== Matrix Operations

```typst
#let solved = matrix-solve(A, b)
#let any = matrix-solve-any(A, b)
#let reduced = row-reduce(B)
#let aug = augment(A, B)
#let parts = split-col(aug, 2)

add: #to-typst(matrix-add(A, B))
sub: #to-typst(matrix-sub(B, A))
mul: #to-typst(matrix-mul(A, I))
scalar mul: #to-typst(matrix-mul(A, 3))
scalar div: #to-typst(matrix-div-scalar(B, 2))
transpose: #to-typst(transpose(A))
det: #to-typst(det(A))
inv: #to-typst(inv(A))
solve: #to-typst(solved)
solve-any: #to-typst(any)
row-reduce: rank #reduced.rank, #to-typst(reduced.matrix)
augment: #to-typst(aug)
split-col: #to-typst(parts.at(0)) | #to-typst(parts.at(1))
```

#let solved = matrix-solve(A, b)
#let any = matrix-solve-any(A, b)
#let reduced = row-reduce(B)
#let aug = augment(A, B)
#let parts = split-col(aug, 2)

#result[
add: #to-typst(matrix-add(A, B))\
sub: #to-typst(matrix-sub(B, A))\
mul: #to-typst(matrix-mul(A, I))\
scalar mul: #to-typst(matrix-mul(A, 3))\
scalar div: #to-typst(matrix-div-scalar(B, 2))\
transpose: #to-typst(transpose(A))\
det: #to-typst(det(A))\
inv: #to-typst(inv(A))\
solve: #to-typst(solved)\
solve-any: #to-typst(any)\
row-reduce: rank #reduced.rank, #to-typst(reduced.matrix)\
augment: #to-typst(aug)\
split-col: #to-typst(parts.at(0)) | #to-typst(parts.at(1))
]

=== Matrix Entries and Polynomial Content

```typst
#let P = matrix(((mul(2, x), mul(4, x)), (mul(6, x), mul(8, x))))

primitive-part: #to-typst(primitive-part(P))
content: #to-typst(content(P))
matrix-at: #to-typst(matrix-at(A, 0, 1))
matrix-shape: #repr(matrix-shape(A))
```

#let P = matrix(((mul(2, x), mul(4, x)), (mul(6, x), mul(8, x))))

#result[
primitive-part: #to-typst(primitive-part(P))\
content: #to-typst(content(P))\
matrix-at: #to-typst(matrix-at(A, 0, 1))\
matrix-shape: #repr(matrix-shape(A))
]

== API Reference

The reference below is generated from the doc comments in `lib.typ` with `tidy`. Its examples use a preamble equivalent to importing the package with `: *`.

#tidy.show-module(
  docs,
  style: tidy.styles.default,
  omit-private-definitions: true,
  first-heading-level: 2,
)
