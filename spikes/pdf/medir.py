#!/usr/bin/env python3
"""Mede a geometria vetorial de um PDF (spike M0 e base do teste golden).

Para cada página: tamanho exato (MediaBox) em mm, segmentos traçados com
comprimento e espessura efetiva em mm (aplicando a CTM, inclusive dentro de
Form XObjects), caixas de curvas e quantidade de imagens raster.

Uso: python3 -I medir.py arquivo.pdf [mais.pdf …] [--json saida.json]
"""
import json
import math
import sys

import pikepdf

PT_MM = 25.4 / 72.0


def mul(a, b):
    # matrizes PDF [a b c d e f]; resultado = a × b
    return [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]


def apply(m, x, y):
    return (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])


def lw_scale(m):
    return math.sqrt(abs(m[0] * m[3] - m[1] * m[2]))


class Meter:
    def __init__(self):
        self.segments = []  # (x1,y1,x2,y2,width) em mm
        self.curves = []  # bbox de subpaths com curvas, em mm
        self.fills = 0
        self.images = 0
        self.text_ops = 0

    def run(self, content_obj, resources, ctm):
        stack = []
        lw = 1.0
        path = []  # lista de subpaths; cada um: lista de ("l"|"c", pontos)
        cur = None
        start = None
        for operands, op in pikepdf.parse_content_stream(content_obj):
            op = str(op)
            v = operands
            if op == "q":
                stack.append((ctm, lw))
            elif op == "Q":
                ctm, lw = stack.pop()
            elif op == "cm":
                ctm = mul([float(x) for x in operands], ctm)
            elif op == "w":
                lw = float(operands[0])
            elif op == "gs":
                gs = resources.get("/ExtGState", {}).get(operands[0])
                if gs is not None and "/LW" in gs:
                    lw = float(gs["/LW"])
            elif op == "m":
                cur = apply(ctm, float(v[0]), float(v[1]))
                start = cur
                path.append([("m", [cur])])
            elif op == "l":
                p = apply(ctm, float(v[0]), float(v[1]))
                path[-1].append(("l", [cur, p]))
                cur = p
            elif op == "c":
                pts = [apply(ctm, float(v[i]), float(v[i + 1])) for i in (0, 2, 4)]
                path[-1].append(("c", [cur] + pts))
                cur = pts[-1]
            elif op in ("v", "y"):
                pts = [apply(ctm, float(v[i]), float(v[i + 1])) for i in (0, 2)]
                path[-1].append(("c", [cur] + pts))
                cur = pts[-1]
            elif op == "re":
                x, y, w, h = (float(t) for t in operands)
                corners = [apply(ctm, x, y), apply(ctm, x + w, y), apply(ctm, x + w, y + h), apply(ctm, x, y + h)]
                sp = [("m", [corners[0]])]
                for i in range(4):
                    sp.append(("l", [corners[i], corners[(i + 1) % 4]]))
                path.append(sp)
            elif op == "h":
                if path and start and cur != start:
                    path[-1].append(("l", [cur, start]))
                cur = start
            elif op in ("S", "s", "B", "B*", "b", "b*", "f", "F", "f*", "n"):
                stroked = op in ("S", "s", "B", "B*", "b", "b*")
                if op in ("s", "b", "b*") and path and start and cur != start:
                    path[-1].append(("l", [cur, start]))
                if op in ("f", "F", "f*", "B", "B*", "b", "b*"):
                    self.fills += 1
                if stroked:
                    width = lw * lw_scale(ctm) * PT_MM
                    for sp in path:
                        has_curve = any(k == "c" for k, _ in sp)
                        if has_curve:
                            xs = [p[0] for _, pts in sp for p in pts]
                            ys = [p[1] for _, pts in sp for p in pts]
                            self.curves.append(
                                {"w_mm": width, "bbox_mm": [min(xs) * PT_MM, min(ys) * PT_MM, max(xs) * PT_MM, max(ys) * PT_MM]}
                            )
                        else:
                            for k, pts in sp:
                                if k == "l":
                                    (x1, y1), (x2, y2) = pts
                                    self.segments.append((x1 * PT_MM, y1 * PT_MM, x2 * PT_MM, y2 * PT_MM, width))
                path = []
            elif op in ("BT",):
                self.text_ops += 1
            elif op == "Do":
                name = operands[0]
                xobj = resources.get("/XObject", {}).get(name)
                if xobj is None:
                    continue
                sub = xobj.get("/Subtype")
                if sub == "/Image":
                    self.images += 1
                elif sub == "/Form":
                    m = [float(x) for x in xobj.get("/Matrix", [1, 0, 0, 1, 0, 0])]
                    res = xobj.get("/Resources", resources)
                    self.run(xobj, res, mul(m, ctm))


def measure(path):
    pdf = pikepdf.open(path)
    pages = []
    for page in pdf.pages:
        mb = [float(x) for x in page.mediabox]
        m = Meter()
        m.run(page.obj, page.resources, [1, 0, 0, 1, -mb[0], -mb[1]])
        pages.append(
            {
                "mediabox_pt": mb,
                "width_mm": (mb[2] - mb[0]) * PT_MM,
                "height_mm": (mb[3] - mb[1]) * PT_MM,
                "segments": [
                    {"len_mm": math.hypot(s[2] - s[0], s[3] - s[1]), "w_mm": s[4], "p1": [s[0], s[1]], "p2": [s[2], s[3]]}
                    for s in m.segments
                ],
                "curves": m.curves,
                "fills": m.fills,
                "images": m.images,
                "text_blocks": m.text_ops,
            }
        )
    return {"file": path, "producer": str(pdf.docinfo.get("/Producer", "")), "pages": pages}


def summarize(r):
    p = r["pages"][0]
    print(f"\n== {r['file']}  ({r['producer']})")
    print(f"   página: {p['width_mm']:.4f} × {p['height_mm']:.4f} mm  (MediaBox {p['mediabox_pt']})")
    print(f"   erro do tamanho: {p['width_mm'] - 420:+.4f} mm, {p['height_mm'] - 297:+.4f} mm")
    print(f"   segmentos traçados: {len(p['segments'])}, curvas: {len(p['curves'])}, imagens raster: {p['images']}, blocos de texto: {p['text_blocks']}")

    def find(length):
        c = [s for s in p["segments"] if abs(s["len_mm"] - length) < 1.0]
        return c

    for target in (100.0, 50.0):
        for s in find(target)[:8]:
            print(f"   linha alvo {target:>5.1f} mm → {s['len_mm']:.4f} mm (erro {s['len_mm'] - target:+.4f}), espessura {s['w_mm']:.4f} mm")
    for c in p["curves"]:
        b = c["bbox_mm"]
        print(f"   curva: diâmetro {b[2] - b[0]:.4f} × {b[3] - b[1]:.4f} mm (alvo 40), espessura {c['w_mm']:.4f} mm")


if __name__ == "__main__":
    args = sys.argv[1:]
    out = None
    if "--json" in args:
        i = args.index("--json")
        out = args[i + 1]
        del args[i : i + 2]
    results = [measure(a) for a in args]
    for r in results:
        summarize(r)
    if out:
        with open(out, "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=1)
