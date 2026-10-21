# MathML in a custom frontend

`_repr_html_()` produces a static, bounded preview. To display an interactive
pager in a custom Pyodide application, retain a `MathMLViewer` in the worker and
connect its messages to the supplied JavaScript module. These APIs do not import
anywidget, Jupyter, IPython, or marimo.

```python
from symbolica import E, MathMLViewer

expression = E('(1+x)^100').expand()
viewer = expression.mathml_viewer(page_size=25)
initial_page = viewer.page()
javascript = MathMLViewer.javascript()
```

`expression.formatted(page_size=10).mathml_viewer()` returns the viewer with those
formatting options. Plain `FormattedOutput` objects without a MathML provider
return `None`.

Import `javascript` as an ES module and mount it in an HTML element:

```javascript
const url = URL.createObjectURL(new Blob([javascript], {type: 'text/javascript'}));
const {mount} = await import(url);
URL.revokeObjectURL(url);
const pager = mount({
  el: document.querySelector('#expression'),
  page: initial_page,
  send: message => workerRequest('message', {viewerId, message}),
});
```

The application supplies `workerRequest`. For each request, call
`viewer.handle_message(message)` on the retained Python viewer and return the
result. It is either `None` or `(field, payload)`; JSON converts the tuple to an
array. The pager handles navigation, popups, inspection, expansion, width fitting,
and keyboard zoom through this connection. Requests are serialized and transport
errors appear in the pager. Transports that push responses separately can call
`pager.update(response)` instead of returning the response from `send`.

No expression object or PyProxy should cross the worker boundary. Only bounded
page dictionaries and JSON messages are transferred. A worker should retain
viewers under its own identifiers, allowing independent displays to coexist.
The initial page includes bounded responsive alternatives, not a hidden complete
expression.

Products keep factor boundaries even when they contain only a few factors.
The browser repacks product rows and sum rows inside large factors using their
rendered widths, the loaded font and the available container width. Resizing,
font loading, previews and restored subtrees use the same packing policy without
additional kernel requests. Static MathML retains bounded fallback rows; source
expressions and omission paths are unchanged.

Root sums use an HTML flex container with one MathML root per term in both the
viewer and static HTML previews, so they wrap in Chromium, Firefox and WebKit.
Term wrappers are replaced rather than duplicated. The viewer converts the
existing MathML rows locally; page payloads and standalone MathML formatting are
unchanged. Static HTML uses a small shared CSS rule for the term roots.

Width fitting retains the chosen DOM, avoids a duplicate initial resize pass,
and only responds to math-font loading. Open popups keep their state and anchor
through refitting, document/ancestor scrolling and viewport resizing without
additional kernel requests.

When removing a display, first `await pager.close()` in JavaScript, then call
`viewer.close()` and release its worker entry. The JavaScript cleanup sends a
disposal message, disconnects observers and removes event handlers; the Python
close releases the retained expression and cached pages.

The same JavaScript module retains its default `render` export for the optional
notebook adapter. Custom applications use `mount` and need no widget library.
