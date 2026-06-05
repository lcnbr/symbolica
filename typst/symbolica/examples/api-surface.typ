#import "../lib.typ": init

#let sym = init()
#let parse = sym.math
#let symbol = sym.symbol
#let var = sym.var
#let wild = sym.wild
#let to-typst = sym.to-typst
#let to-symbolica = sym.to-symbolica
#let add = sym.add
#let mul = sym.mul
#let replace = sym.replace
#let replace-wildcards = sym.replace-wildcards
#let series = sym.series
#let evaluate = sym.evaluate
#let solve-linear = sym.solve-linear
#let nsolve = sym.nsolve
#let nsolve-system = sym.nsolve-system
#let matrix = sym.matrix
#let make-vec = sym.vec
#let identity = sym.identity
#let eye = sym.eye
#let matrix-solve = sym.matrix-solve
#let matrix-mul = sym.matrix-mul
#let det = sym.det
#let inv = sym.inv
#let transpose = sym.transpose
#let augment = sym.augment
#let row-reduce = sym.row-reduce
#let matrix-at = sym.matrix-at
#let matrix-shape = sym.matrix-shape

#let x = var("x")
#let y = var("y")
#let xp = symbol("x", namespace: "physics")
#let namespaced = add(xp, x)

#let value = evaluate(parse($x^2 + y$), values: ((x, 2.0), (y, 3.0)))

#let builtin = init(namespace: "symbolica")
#let bparse = builtin.math
#let bvar = builtin.var
#let bseries = builtin.series
#let bto-typst = builtin.to-typst
#let sx = bvar("x")
#let ser = bseries(bparse($cos(x)/(x + 1)$), sx, 0, 3)

#let expr = parse($f(x, y) + x$)
#let swapped = replace(expr, parse($f("a_", "b_")$), parse($g("b_", "a_")$))
#let wildcarded = replace-wildcards(parse($h("a_")$), ((wild("a"), parse($x + 1$)),))
#let rhs-only = replace(parse($f(x)$), parse($f("a_")$), parse($g("a_", "fresh_")$), allow-new-wildcards-on-rhs: true)

#let exact = solve-linear((parse($2 x + y - 5$), parse($x - y - 1$)), (x, y))
#let root = nsolve(parse($x^2 - 2$), x, 1.0)
#let roots = nsolve-system((parse($x^2 + y - 3$), parse($x - y$)), (x, y), (1.0, 1.0))

#let A = matrix($mat(2, 1; 1, -1)$)
#let b = make-vec($vec(5, 1)$)
#let B = matrix(((1, 2), (3, 4)))
#let solved = matrix-solve(A, b)
#let reduced = row-reduce(A)
#let augmented = augment(A, b)

Namespaces: #raw(to-symbolica(namespaced, namespaces: true))

Evaluate: #repr(value)

Series: #bto-typst(ser)

Replace: #to-typst(swapped)

Wildcard replace: #to-typst(wildcarded)

RHS-only wildcard: #to-typst(rhs-only)

Exact solve: #exact.map(to-typst).join[, ]

Numeric solve: #repr(root), #repr(roots)

Matrix A: #to-typst(A)

Matrix B: #to-typst(B)

A shape: #repr(matrix-shape(A)); A[0, 1]: #to-typst(matrix-at(A, 0, 1))

Solve A x = b: #to-typst(solved)

Det: #to-typst(det(A))

Inverse: #to-typst(inv(A))

Transpose: #to-typst(transpose(A))

A times I: #to-typst(matrix-mul(A, identity(2)))

Eye: #to-typst(eye((1, 2)))

Augment: #to-typst(augmented)

Row-reduce rank: #reduced.rank; matrix: #to-typst(reduced.matrix)
