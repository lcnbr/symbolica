#import "@preview/parsely:0.1.0"

// Minimal reproducer for Parsely 0.1.0.
// This fails with: `type none has no method len` in parsely/src/parse.typ:159.
#let grammar = (
  add: (infix: $+$, prec: 1, assoc: true),
  mul: (infix: $$, prec: 2.5, assoc: true),
  "()": (match: $(#parsely.slot("expr*"))$),
  pow: (match: $#parsely.slot("base")^#parsely.slot("exp")$),
  frac: math.frac,
)

#let input = $((y^x + 1)^2)/(1/a + "some" + "thing" )$
// Removing the trailing space before the denominator's closing parenthesis works:
// #let input = $((y^x + 1)^2)/(1/a + "some" + "thing")$

#let parsed = parsely.parse(input, grammar)
#metadata(repr(parsed)) <parsed>
