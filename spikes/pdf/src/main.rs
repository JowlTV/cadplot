//! Spike M0 — gerador de PDF.
//!
//! Gera a MESMA folha de teste por dois caminhos:
//!   1. `printpdf.pdf`: PDF montado em Rust com printpdf (gerador próprio);
//!   2. `folha.html` / `folha-canvas.html`: página para o print-to-PDF do
//!      Chromium, que é o mesmo motor do WebView2 `PrintToPdf`.
//! O script `medir.py` compara os dois: tamanho da página, comprimentos
//! medidos no papel, espessuras de linha e se a saída é vetorial.
//!
//! Folha: A3 paisagem (420 × 297 mm), escala 1:100, desenho em milímetros.
//!   - linha de 10 m → deve medir 100,0 mm no papel;
//!   - linha de 5 m → 50,0 mm;
//!   - círculo r = 2 m → diâmetro 40,0 mm;
//!   - seis linhas com espessuras 0,13 / 0,18 / 0,25 / 0,35 / 0,50 / 0,70 mm.
//!
//! Uso: cargo run -p cadplot-pdf-spike -- <pasta de saída>

use std::fmt::Write as _;
use std::path::PathBuf;

use printpdf::*;

const PAPER_W_MM: f64 = 420.0;
const PAPER_H_MM: f64 = 297.0;
const SCALE: f64 = 1.0 / 100.0; // papel/desenho
const ORIGIN_MM: (f64, f64) = (30.0, 30.0); // posição do (0,0) do desenho no papel

/// Primitivas no espaço do desenho (mm de desenho), espessura em mm de papel.
enum Prim {
    Line { a: (f64, f64), b: (f64, f64), w_mm: f64 },
    Circle { c: (f64, f64), r: f64, w_mm: f64 },
    PaperRect { x: f64, y: f64, w: f64, h: f64, w_mm: f64 },
}

fn sheet() -> Vec<Prim> {
    let mut v = vec![
        // Moldura a 10 mm da borda (já em mm de papel).
        Prim::PaperRect { x: 10.0, y: 10.0, w: PAPER_W_MM - 20.0, h: PAPER_H_MM - 20.0, w_mm: 0.5 },
        // Régua: 10 m e 5 m.
        Prim::Line { a: (0.0, 0.0), b: (10_000.0, 0.0), w_mm: 0.35 },
        Prim::Line { a: (0.0, 0.0), b: (0.0, 5_000.0), w_mm: 0.35 },
        // Círculo r = 2 m.
        Prim::Circle { c: (16_000.0, 3_000.0), r: 2_000.0, w_mm: 0.25 },
    ];
    for (i, w) in [0.13, 0.18, 0.25, 0.35, 0.50, 0.70].iter().enumerate() {
        let y = 8_000.0 + i as f64 * 1_500.0;
        v.push(Prim::Line { a: (0.0, y), b: (10_000.0, y), w_mm: *w });
    }
    v
}

/// Mundo → papel (mm), origem do papel no canto inferior esquerdo.
fn to_paper(p: (f64, f64)) -> (f64, f64) {
    (ORIGIN_MM.0 + p.0 * SCALE, ORIGIN_MM.1 + p.1 * SCALE)
}

const K: f64 = 0.552_284_749_830_793_4; // 4/3·(√2−1): arco de 90° em Bézier cúbica

fn circle_beziers(c: (f64, f64), r: f64) -> Vec<[(f64, f64); 4]> {
    let k = K * r;
    let (x, y) = c;
    vec![
        [(x + r, y), (x + r, y + k), (x + k, y + r), (x, y + r)],
        [(x, y + r), (x - k, y + r), (x - r, y + k), (x - r, y)],
        [(x - r, y), (x - r, y - k), (x - k, y - r), (x, y - r)],
        [(x, y - r), (x + k, y - r), (x + r, y - k), (x + r, y)],
    ]
}

fn mm(v: f64) -> Mm {
    Mm(v as f32)
}

fn lp(p: (f64, f64), bezier: bool) -> LinePoint {
    LinePoint { p: Point::new(mm(p.0), mm(p.1)), bezier }
}

fn black() -> Color {
    Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None })
}

fn build_printpdf() -> Vec<u8> {
    let mut doc = PdfDocument::new("CadPlot — spike M0 (printpdf)");
    let mut ops = vec![
        Op::SetOutlineColor { col: black() },
        Op::SetLineCapStyle { cap: LineCapStyle::Butt },
        Op::SetLineJoinStyle { join: LineJoinStyle::Miter },
    ];
    for p in sheet() {
        match p {
            Prim::Line { a, b, w_mm } => {
                ops.push(Op::SetOutlineThickness { pt: Pt((w_mm * 72.0 / 25.4) as f32) });
                ops.push(Op::DrawLine {
                    line: Line { points: vec![lp(to_paper(a), false), lp(to_paper(b), false)], is_closed: false },
                });
            }
            Prim::PaperRect { x, y, w, h, w_mm } => {
                ops.push(Op::SetOutlineThickness { pt: Pt((w_mm * 72.0 / 25.4) as f32) });
                ops.push(Op::DrawLine {
                    line: Line {
                        points: vec![lp((x, y), false), lp((x + w, y), false), lp((x + w, y + h), false), lp((x, y + h), false)],
                        is_closed: true,
                    },
                });
            }
            Prim::Circle { c, r, w_mm } => {
                ops.push(Op::SetOutlineThickness { pt: Pt((w_mm * 72.0 / 25.4) as f32) });
                let pc = to_paper(c);
                let segs = circle_beziers(pc, r * SCALE);
                let mut pts = vec![lp(segs[0][0], false)];
                for s in &segs {
                    pts.push(lp(s[1], true));
                    pts.push(lp(s[2], true));
                    pts.push(lp(s[3], false));
                }
                ops.push(Op::DrawLine { line: Line { points: pts, is_closed: true } });
            }
        }
    }
    ops.extend_from_slice(&[
        Op::StartTextSection,
        Op::SetTextCursor { pos: Point::new(Mm(30.0), Mm(20.0)) },
        Op::SetFont { font: PdfFontHandle::Builtin(BuiltinFont::Helvetica), size: Pt(10.0) },
        Op::SetFillColor { col: black() },
        Op::ShowText { items: vec![TextItem::Text("CadPlot spike M0 - A3 1:100 - printpdf".into())] },
        Op::EndTextSection,
    ]);
    let page = PdfPage::new(mm(PAPER_W_MM), mm(PAPER_H_MM), ops);
    let mut warnings = Vec::new();
    doc.with_pages(vec![page]).save(&PdfSaveOptions::default(), &mut warnings)
}

/// Página HTML com SVG em mm: o caminho "vetorial" do print-to-PDF.
fn build_html_svg() -> String {
    let mut s = String::new();
    let _ = write!(
        s,
        r##"<!doctype html><html lang="pt-BR"><meta charset="utf-8"><title>CadPlot spike M0 (SVG)</title>
<style>@page {{ size: {w}mm {h}mm; margin: 0 }} html,body {{ margin:0; padding:0 }}
svg {{ display:block; width:{w}mm; height:{h}mm }}</style>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}mm" height="{h}mm">
<g fill="none" stroke="#000" stroke-linecap="butt" stroke-linejoin="miter">
"##,
        w = PAPER_W_MM,
        h = PAPER_H_MM
    );
    // SVG tem Y para baixo: y_svg = H - y_papel.
    let f = |p: (f64, f64)| (p.0, PAPER_H_MM - p.1);
    for p in sheet() {
        match p {
            Prim::Line { a, b, w_mm } => {
                let (a, b) = (f(to_paper(a)), f(to_paper(b)));
                let _ = writeln!(s, r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke-width="{}"/>"#, a.0, a.1, b.0, b.1, w_mm);
            }
            Prim::PaperRect { x, y, w, h, w_mm } => {
                let _ = writeln!(s, r#"<rect x="{}" y="{}" width="{}" height="{}" stroke-width="{}"/>"#, x, PAPER_H_MM - y - h, w, h, w_mm);
            }
            Prim::Circle { c, r, w_mm } => {
                let c = f(to_paper(c));
                let _ = writeln!(s, r#"<circle cx="{}" cy="{}" r="{}" stroke-width="{}"/>"#, c.0, c.1, r * SCALE, w_mm);
            }
        }
    }
    let _ = write!(
        s,
        r#"</g><text x="30" y="{}" font-family="Arial" font-size="3.528">CadPlot spike M0 — A3 1:100 — Chromium/SVG — ação</text></svg></html>"#,
        PAPER_H_MM - 20.0
    );
    s
}

/// Mesma folha desenhada num <canvas>, que é como o visualizador WebGL/Canvas renderiza.
fn build_html_canvas() -> String {
    let mut js = String::new();
    for p in sheet() {
        match p {
            Prim::Line { a, b, w_mm } => {
                let (a, b) = (to_paper(a), to_paper(b));
                let _ = writeln!(js, "ln({},{},{},{},{});", a.0, a.1, b.0, b.1, w_mm);
            }
            Prim::PaperRect { x, y, w, h, w_mm } => {
                let _ = writeln!(js, "ln({x},{y},{},{y},{w_mm});ln({},{y},{},{},{w_mm});ln({},{},{x},{},{w_mm});ln({x},{},{x},{y},{w_mm});", x + w, x + w, x + w, y + h, x + w, y + h, y + h, y + h);
            }
            Prim::Circle { c, r, w_mm } => {
                let c = to_paper(c);
                let _ = writeln!(js, "ci({},{},{},{});", c.0, c.1, r * SCALE, w_mm);
            }
        }
    }
    format!(
        r#"<!doctype html><html><meta charset="utf-8"><title>CadPlot spike M0 (canvas)</title>
<style>@page {{ size: {w}mm {h}mm; margin: 0 }} html,body {{ margin:0 }} canvas {{ display:block; width:{w}mm; height:{h}mm }}</style>
<canvas id="c"></canvas><script>
const W={w},H={h},DPI=300,PX=DPI/25.4,c=document.getElementById('c');c.width=Math.round(W*PX);c.height=Math.round(H*PX);
const g=c.getContext('2d');g.strokeStyle='#000';
function ln(x1,y1,x2,y2,w){{g.lineWidth=w*PX;g.beginPath();g.moveTo(x1*PX,(H-y1)*PX);g.lineTo(x2*PX,(H-y2)*PX);g.stroke();}}
function ci(x,y,r,w){{g.lineWidth=w*PX;g.beginPath();g.arc(x*PX,(H-y)*PX,r*PX,0,Math.PI*2);g.stroke();}}
{js}</script></html>"#,
        w = PAPER_W_MM,
        h = PAPER_H_MM
    )
}

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "spikes/pdf/out".into()));
    std::fs::create_dir_all(&out).expect("pasta de saída");
    std::fs::write(out.join("printpdf.pdf"), build_printpdf()).expect("printpdf.pdf");
    std::fs::write(out.join("folha.html"), build_html_svg()).expect("folha.html");
    std::fs::write(out.join("folha-canvas.html"), build_html_canvas()).expect("folha-canvas.html");
    println!("gerado em {}", out.display());
}
