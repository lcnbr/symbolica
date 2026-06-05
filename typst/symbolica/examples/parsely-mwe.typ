#import "@preview/parsely:0.1.0"

// Minimal Parsely parse example matching the Symbolica grammar shape.
#let grammar = (
  add: (infix: $+$, prec: 1, assoc: true),
  mul: (infix: $$, prec: 2.5, assoc: true),
  "()": (match: $(#parsely.slot("expr*"))$),
  pow: (match: $#parsely.slot("base")^#parsely.slot("exp")$),
  frac: math.frac,
)

#let input = $((y^x + 1)^2)/(1/a + "some" + "thing")$

#let parsed = parsely.parse(input, grammar)
#metadata(repr(parsed)) <parsed>
