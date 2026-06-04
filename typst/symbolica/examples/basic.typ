#import "../lib.typ" as symbolica

#let input = symbolica.math($(f((y^x + 1)^2)+xi)/("some"+"thing")$)
#let expanded = symbolica.expand(input)
#let d = symbolica.derivative(input, "x")
#let combined = symbolica.add(expanded, symbolica.mul(3, "x"))
#let replaced = symbolica.replace-all(input, "y", "z")

Input: #symbolica.to-typst(input)

Expanded: #symbolica.to-typst(expanded)

Derivative: #symbolica.to-typst(d)

Combined: #symbolica.to-typst(combined)

Replaced: #symbolica.to-typst(replaced)

Symbolica: #symbolica.to-symbolica(input)
