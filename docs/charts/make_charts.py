"""Draws the README charts as SVG, in light and dark versions.

    python3 docs/charts/make_charts.py

Numbers come from the evaluation runs recorded in evals/cards/README.md: the synthetic mixed
and held-out corpora run through the earlier role pipeline ("before") and the card pipeline
("now") with the same harness, and text-recognition confidence measured on synthetic scans.
Colours were checked with a colour-blindness validator in both modes.
"""
from pathlib import Path
from xml.sax.saxutils import escape

OUT = Path(__file__).resolve().parents[1] / "images" / "charts"

THEMES = {
    "light": {
        "surface": "#fffdf9", "border": "#efe3d3", "ink": "#2b2118", "ink2": "#7d6a59",
        "muted": "#9a8a7a", "grid": "#f1e8dc", "axis": "#d9cbb8",
        "before": "#2a78d6", "now": "#eb6834", "good": "#0ca30c", "wait": "#fab219",
    },
    "dark": {
        "surface": "#28211b", "border": "#3d3229", "ink": "#f6ede3", "ink2": "#c9b8a6",
        "muted": "#9a8a7a", "grid": "#352c24", "axis": "#4a3d32",
        "before": "#3987e5", "now": "#d95926", "good": "#0ca30c", "wait": "#fab219",
    },
}
FONT = "system-ui, -apple-system, 'Segoe UI', sans-serif"
WIDTH = 720


def text(x, y, value, size=12, fill="ink", weight=400, anchor="start", t=None):
    return (f'<text x="{x}" y="{y}" font-size="{size}" font-weight="{weight}" '
            f'fill="{t[fill]}" text-anchor="{anchor}">{escape(str(value))}</text>')


def bar(x, y, w, h, color, round_end=True):
    """A bar anchored at x; only the data end (right) is rounded, radius 4."""
    if w <= 0:
        return ""
    r = min(4, w / 2, h / 2) if round_end else 0
    return (f'<path d="M{x},{y} H{x + w - r} Q{x + w},{y} {x + w},{y + r} V{y + h - r} '
            f'Q{x + w},{y + h} {x + w - r},{y + h} H{x} Z" fill="{color}"/>')


def frame(t, height, title, subtitle, body, label):
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" viewBox="0 0 {WIDTH} {height}" role="img" aria-label="{escape(label)}" font-family="{FONT}">
<title>{escape(title)}</title><desc>{escape(label)}</desc>
<rect x="0.5" y="0.5" width="{WIDTH - 1}" height="{height - 1}" rx="16" fill="{t['surface']}" stroke="{t['border']}"/>
{text(28, 40, title, 16, 'ink', 650, t=t)}
{text(28, 62, subtitle, 12.5, 'ink2', t=t)}
{body}
</svg>
'''


def legend(t, x, y, items):
    parts, cx = [], x
    for label, color in items:
        parts.append(f'<rect x="{cx}" y="{y - 9}" width="10" height="10" rx="2" fill="{t[color]}"/>')
        parts.append(text(cx + 16, y, label, 12, "ink2", t=t))
        cx += 26 + len(label) * 6.6
    return "\n".join(parts)


def gridlines(t, x0, x1, y0, y1, ticks, maximum, fmt=lambda v: str(v)):
    parts = []
    for v in ticks:
        x = x0 + (x1 - x0) * v / maximum
        parts.append(f'<line x1="{x}" y1="{y0}" x2="{x}" y2="{y1}" stroke="{t["grid"]}" stroke-width="1"/>')
        parts.append(text(x, y1 + 16, fmt(v), 11, "muted", anchor="middle", t=t))
    parts.append(f'<line x1="{x0}" y1="{y0}" x2="{x0}" y2="{y1}" stroke="{t["axis"]}" stroke-width="1"/>')
    return "\n".join(parts)


def placed(t):
    """Stacked bars: filing outcome for the same 35 documents."""
    rows = [("Before", 29, 6), ("Now", 35, 0)]
    x0, x1, top, h, gap = 110, 560, 104, 30, 22
    body = [legend(t, 28, 88, [("Filed correctly", "good"), ("Waited for you", "wait")])]
    bottom = top + len(rows) * (h + gap) - gap
    body.append(gridlines(t, x0, x1, top - 8, bottom + 8, [0, 10, 20, 30, 35], 35))
    for i, (name, filed, waited) in enumerate(rows):
        y = top + i * (h + gap)
        body.append(text(x0 - 14, y + h / 2 + 4, name, 13, "ink", 600, "end", t))
        wf = (x1 - x0) * filed / 35
        ww = (x1 - x0) * waited / 35
        body.append(bar(x0, y, wf - (2 if waited else 0), h, t["good"], round_end=not waited))
        if waited:
            body.append(bar(x0 + wf, y, ww, h, t["wait"]))
        label = f"{filed} filed" + (f" · {waited} waited" if waited else "")
        body.append(text(x1 + 12, y + h / 2 + 4, label, 12.5, "ink", 600, t=t))
    height = bottom + 52
    return frame(t, height, "Every file placed",
                 "Filing the same 35 synthetic documents (long, scanned, Swedish, German). 0 filed wrong in either.",
                 "\n".join(body),
                 "Before: 29 of 35 documents filed correctly, 6 waited for review. "
                 "Now: 35 of 35 filed correctly, none waited. No document was filed wrongly in either run.")


def faster(t):
    """Grouped bars on one shared scale: median seconds to file a document, per corpus."""
    corpora = [("Mixed corpus (35)", 3.1, 0.8), ("Held-out corpus (29)", 3.2, 0.6)]
    x0, x1, top, h, inner, gap, maximum = 190, 620, 112, 24, 4, 26, 4
    body = [legend(t, 28, 88, [("Before (role pipeline)", "before"), ("Now (card pipeline)", "now")])]
    group = 2 * h + inner
    bottom = top + len(corpora) * (group + gap) - gap
    body.append(gridlines(t, x0, x1, top - 8, bottom + 8, [0, 1, 2, 3, 4], maximum,
                          lambda v: f"{v:g} s"))
    for i, (name, before, now) in enumerate(corpora):
        y = top + i * (group + gap)
        body.append(text(x0 - 14, y + group / 2 + 4, name, 12.5, "ink", 600, "end", t))
        for j, (value, color) in enumerate([(before, "before"), (now, "now")]):
            by = y + j * (h + inner)
            w = (x1 - x0) * value / maximum
            body.append(bar(x0, by, w, h, t[color]))
            body.append(text(x0 + w + 8, by + h / 2 + 4, f"{value:g} s", 12.5, "ink", 600, t=t))
    height = bottom + 52
    return frame(t, height, "About four times faster",
                 "Median seconds to file one document, Apple Intelligence on device. Same harness, same documents.",
                 "\n".join(body),
                 "Mixed corpus: 3.1 seconds per document before, 0.8 now. "
                 "Held-out corpus: 3.2 seconds before, 0.6 now.")


def confidence(t):
    """Dot plot: text-recognition confidence per scan, with the 0.6 cut-off."""
    scans = [("Swedish bank statement", 1.00), ("Arabic bank statement", 1.00),
             ("Hindi bank statement", 1.00), ("Invoice with QR code", 1.00),
             ("Passport page", 0.75), ("Tamil bank statement", 0.42),
             ("Sinhala bank statement", 0.30)]
    x0, x1, top, step = 200, 640, 112, 26
    body = [legend(t, 28, 88, [("Read and filed", "good"), ("Waits for you", "wait")])]
    bottom = top + (len(scans) - 1) * step
    body.append(gridlines(t, x0, x1, top - 14, bottom + 14, [0, 0.2, 0.4, 0.6, 0.8, 1.0], 1.0,
                          lambda v: f"{v:.1f}"))
    cut = x0 + (x1 - x0) * 0.6
    body.append(f'<line x1="{cut}" y1="{top - 14}" x2="{cut}" y2="{bottom + 14}" '
                f'stroke="{t["ink2"]}" stroke-width="1.5" stroke-dasharray="4 4"/>')
    body.append(text(cut - 6, top - 20, "cut-off 0.6", 11.5, "ink2", 600, "end", t))
    for i, (name, value) in enumerate(scans):
        y = top + i * step
        color = t["good"] if value >= 0.6 else t["wait"]
        body.append(f'<line x1="{x0}" y1="{y}" x2="{x1}" y2="{y}" stroke="{t["grid"]}" stroke-width="1"/>')
        body.append(text(x0 - 14, y + 4, name, 12.5, "ink", anchor="end", t=t))
        cx = x0 + (x1 - x0) * value
        body.append(f'<circle cx="{cx}" cy="{y}" r="6" fill="{color}" stroke="{t["surface"]}" stroke-width="2"/>')
        if value < 0.6:
            body.append(text(cx + 12, y + 4, f"{value:.2f}", 12, "ink", 600, t=t))
    height = bottom + 52
    return frame(t, height, "Spotting scans Tuck can't read",
                 "Text-recognition confidence on synthetic scans. Apple's OCR does not support Sinhala or Tamil.",
                 "\n".join(body),
                 "Readable scans score 0.75 to 1.0: Swedish, Arabic and Hindi bank statements, an invoice and a "
                 "passport page. Tamil scores 0.42 and Sinhala 0.30, below the 0.6 cut-off, so those files wait "
                 "for the user instead of being guessed.")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, draw in [("placed", placed), ("faster", faster), ("confidence", confidence)]:
        for mode, t in THEMES.items():
            (OUT / f"{name}-{mode}.svg").write_text(draw(t))
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
