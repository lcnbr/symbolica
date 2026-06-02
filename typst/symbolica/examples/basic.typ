#import "../lib.typ": init, from-math, expand, derivative, to-typst

#let sym = init(source: "../symbolica.wasm")
#let input = from-math($(x + 1)^2$)
#let expanded = expand(sym, input)

$ #to-typst(sym, expanded) $

#let d = derivative(sym, input, "x")

$ #to-typst(sym, d) $
