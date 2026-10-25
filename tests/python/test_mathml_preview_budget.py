"""Run with `python tests/python/test_mathml_preview_budget.py`.

Minimal notebook reproduction (only Symbolica is needed)::

    from symbolica import E
    s = "+".join(f"x{i}" for i in range(400))
    p = f"({s})*({s}+a)*({s}+b)"
    expression = E(f"{p}+c*({p})")
    expression  # Used to display only clickable [1] + [2].

The expression stays factorized; no expansion or tensor package is involved.
"""

import math
import unittest
from xml.etree import ElementTree

from symbolica import E, S
from test_mathml_display import mathml_value


class MathMLPreviewBudgetTests(unittest.TestCase):
    def test_oversized_first_item_keeps_visible_math_and_navigable_omissions(self):
        s = "+".join(f"x{i}" for i in range(400))
        p = f"({s})*({s}+a)*({s}+b)"
        for text in (f"{p}+c*({p})", f"1/(({p}+c)*({p}+d))"):
            with self.subTest(text=text[:40]):
                expression = E(text)
                original = str(expression)
                viewer = expression.mathml_viewer()
                try:
                    page = viewer.page()
                    self.assertEqual((page['start'], page['end'], page['total']), (0, 1, 2))
                    self.assertIsNotNone(ElementTree.fromstring(page['html']).find('.//{*}mi'))
                    self.assertTrue(any(len(path) > 1 for path, _ in page['markers']))
                    # Open each omission at its own continuation offset. It must
                    # resolve and progress, rather than reopen the same marker.
                    for (path, _), start in zip(page['markers'], page['marker_starts']):
                        _, opened = viewer.handle_message({'action': 'preview', 'popup': True, 'value': path, 'start': start})
                        self.assertGreater(opened['end'], opened['start'])
                        self.assertIsNotNone(ElementTree.fromstring(opened['html']).find('.//{*}mi'))
                    self.assertEqual(str(expression), original)
                finally:
                    viewer.close()

    def test_partial_previews_preserve_signs_and_fraction_meaning(self):
        s = "+".join(f"x{i}" for i in range(400))
        p = f"({s})*({s}+a)*({s}+b)"
        values = {f'x{i}': 1 for i in range(400)} | dict(a=2, b=3, c=4, d=5)
        constants = {S(name): value for name, value in values.items()}
        for text in (f"-({p})+c*({p})", f"-1/(({p}+c)*({p}+d))"):
            expression = E(text)
            viewer = expression.mathml_viewer()
            try:
                page = viewer.page()
                tree = ElementTree.fromstring(page['html'])
                for node in tree.iter():
                    if 'data-symbolica-marker' not in node.attrib:
                        continue
                    index = int(node.attrib['data-symbolica-marker'])
                    path, label = page['markers'][index]
                    target = expression
                    for child in path:
                        target = target[child]
                    start = page['marker_starts'][index]
                    if start:
                        children = target[start:]
                        target = (sum(children, E('0')) if 'sum' in label
                                  else math.prod(children, start=E('1')))
                    node.tag = '{http://www.w3.org/1998/Math/MathML}mn'
                    node.text = str(target.evaluate(constants).real)
                rendered = float(mathml_value(ElementTree.tostring(tree, encoding='unicode'), values))
                expected = expression.evaluate(constants).real
                self.assertTrue(math.isclose(rendered, expected, rel_tol=1e-12), (rendered, expected))
            finally:
                viewer.close()


if __name__ == '__main__':
    unittest.main()
