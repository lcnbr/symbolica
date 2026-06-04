import marimo

__generated_with = "0.23.8"
app = marimo.App(width="medium")


@app.cell
def _():
    import marimo as mo
    import symbolica

    return


@app.cell
def _():
    from symbolica import E,S

    x = S("x")
    e =x**(2*x + 1)


    return (e,)


@app.cell
def _(e):
    e
    return


@app.cell
def _():
    return


if __name__ == "__main__":
    app.run()
