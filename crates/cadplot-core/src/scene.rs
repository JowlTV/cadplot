//! Cena de visualização: o espaço modelo convertido em lotes de geometria
//! prontos para a GPU, sem perder rastreio do que não foi desenhado.
//!
//! - Toda a conta é feita em `f64`. Só no final os vértices são gravados em
//!   `f32` **relativos a uma origem** (centro da extensão), para a GPU não
//!   perder precisão em coordenadas grandes. Medidas e plotagem não usam esses
//!   `f32`.
//! - Blocos são expandidos recursivamente (INSERT aninhado, MINSERT, escala,
//!   rotação, OCS/normal), com as regras do AutoCAD para camada "0" e cor
//!   PorBloco.
//! - Cada entidade entra em `RenderStats` como desenhada, aproximada ou não
//!   desenhada, com o motivo. Nada some em silêncio.

use std::collections::{BTreeMap, HashMap};

use acadrust::entities::hatch::BoundaryEdge;
use acadrust::{CadDocument, Color, EntityType, Vector3};
use serde::{Deserialize, Serialize};

use crate::naming::entity_type_name;

// ---------------------------------------------------------------- tipos públicos

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimKind {
    Lines,
    Triangles,
    Points,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneLayer {
    pub name: String,
    pub color: [u8; 3],
    pub color_label: String,
    pub linetype: String,
    pub lineweight: String,
    pub on: bool,
    pub frozen: bool,
    pub locked: bool,
    pub plottable: bool,
    /// Entidades (já expandidas dos blocos) que ficaram nesta camada.
    pub primitives: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMeta {
    pub layer: u32,
    /// Índice em `host_sets`: camadas dos INSERT ancestrais (congelar uma delas esconde o lote).
    pub hosts: u32,
    pub color: [u8; 3],
    pub kind: PrimKind,
    /// Deslocamento e quantidade em `f32` no buffer de vértices (x, y intercalados).
    pub offset: u32,
    pub len: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneText {
    pub layer: u32,
    pub hosts: u32,
    pub color: [u8; 3],
    /// Posição relativa à origem da cena.
    pub x: f64,
    pub y: f64,
    pub height: f64,
    pub rotation: f64,
    pub width_factor: f64,
    /// Âncora: 0 = esquerda/base, 0,5 = centro, 1 = direita/topo.
    pub ax: f64,
    pub ay: f64,
    /// `true` para MTEXT (âncora vertical em relação ao bloco de linhas).
    pub multiline: bool,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeStats {
    pub drawn: usize,
    pub approximated: usize,
    pub skipped: usize,
    /// Motivo → quantidade.
    pub notes: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenderStats {
    /// Por tipo de entidade (contando instâncias expandidas dos blocos).
    pub by_type: BTreeMap<String, TypeStats>,
    pub inserts_expanded: usize,
    pub missing_blocks: BTreeMap<String, usize>,
    pub max_depth_reached: usize,
    pub segments: usize,
    pub triangles: usize,
    pub texts: usize,
    /// Limitações gerais desta versão do visualizador.
    pub global_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneMeta {
    pub schema: u32,
    pub space: String,
    pub origin: [f64; 2],
    /// Extensão da geometria desenhada (coordenadas absolutas).
    pub bounds: Option<[f64; 4]>,
    pub layers: Vec<SceneLayer>,
    pub host_sets: Vec<Vec<u32>>,
    pub batches: Vec<BatchMeta>,
    pub texts: Vec<SceneText>,
    pub stats: RenderStats,
}

pub struct Scene {
    pub meta: SceneMeta,
    pub vertices: Vec<f32>,
}

pub const SCENE_MAGIC: &[u8; 4] = b"CPS1";

impl Scene {
    /// Serializa: "CPS1" | u32 tamanho do JSON | JSON | preenchimento até múltiplo de 4 | f32 LE.
    pub fn to_bytes(&self, extra_json: Option<&serde_json::Value>) -> Vec<u8> {
        let mut meta = serde_json::to_value(&self.meta).expect("cena serializável");
        if let (Some(extra), Some(obj)) = (extra_json, meta.as_object_mut()) {
            obj.insert("report".into(), extra.clone());
        }
        let json = serde_json::to_vec(&meta).expect("json");
        let mut out = Vec::with_capacity(12 + json.len() + self.vertices.len() * 4);
        out.extend_from_slice(SCENE_MAGIC);
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(&json);
        while out.len() % 4 != 0 {
            out.push(b' ');
        }
        for v in &self.vertices {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }
}

// ------------------------------------------------------------- transformações

/// Afim 2D: x' = a·x + c·y + e ; y' = b·x + d·y + f.
#[derive(Debug, Clone, Copy)]
pub struct Xf {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Xf {
    pub const ID: Xf = Xf { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    fn apply(&self, p: (f64, f64)) -> (f64, f64) {
        (self.a * p.0 + self.c * p.1 + self.e, self.b * p.0 + self.d * p.1 + self.f)
    }
    /// self ∘ other (aplica `other` primeiro).
    fn then_after(&self, o: &Xf) -> Xf {
        Xf {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            e: self.a * o.e + self.c * o.f + self.e,
            f: self.b * o.e + self.d * o.f + self.f,
        }
    }
    fn translate(x: f64, y: f64) -> Xf {
        Xf { e: x, f: y, ..Xf::ID }
    }
    fn rotate(t: f64) -> Xf {
        let (s, c) = t.sin_cos();
        Xf { a: c, b: s, c: -s, d: c, e: 0.0, f: 0.0 }
    }
    fn scale(sx: f64, sy: f64) -> Xf {
        Xf { a: sx, d: sy, ..Xf::ID }
    }
    /// Escala média (para alturas de texto e tolerâncias).
    fn mean_scale(&self) -> f64 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }
    fn angle(&self) -> f64 {
        self.b.atan2(self.a)
    }
    fn is_mirrored(&self) -> bool {
        self.a * self.d - self.b * self.c < 0.0
    }
    /// OCS → WCS projetado no plano XY, pelo algoritmo de eixo arbitrário.
    fn ocs(normal: &Vector3) -> Xf {
        let (nx, ny, nz) = (normal.x, normal.y, normal.z);
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        if !(len > 0.0) || ((nx / len).abs() < 1e-12 && (ny / len).abs() < 1e-12 && nz > 0.0) {
            return Xf::ID;
        }
        let n = (nx / len, ny / len, nz / len);
        let ax = if n.0.abs() < 1.0 / 64.0 && n.1.abs() < 1.0 / 64.0 {
            cross((0.0, 1.0, 0.0), n)
        } else {
            cross((0.0, 0.0, 1.0), n)
        };
        let ax = norm(ax);
        let ay = norm(cross(n, ax));
        Xf { a: ax.0, b: ax.1, c: ay.0, d: ay.1, e: 0.0, f: 0.0 }
    }
}

fn cross(a: (f64, f64, f64), b: (f64, f64, f64)) -> (f64, f64, f64) {
    (a.1 * b.2 - a.2 * b.1, a.2 * b.0 - a.0 * b.2, a.0 * b.1 - a.1 * b.0)
}
fn norm(v: (f64, f64, f64)) -> (f64, f64, f64) {
    let l = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
    if l > 0.0 {
        (v.0 / l, v.1 / l, v.2 / l)
    } else {
        v
    }
}

// ------------------------------------------------------------ tesselação

/// Número de segmentos para um arco de raio `r` (já em unidades do mundo) e varredura `sweep`.
fn arc_segments(r: f64, sweep: f64) -> usize {
    if !(r > 0.0) || !sweep.is_finite() {
        return 1;
    }
    // Erro de corda ≈ 0,1 % do raio; entre 4 e 512 segmentos.
    let step = 2.0 * (1.0 - 0.001_f64).acos();
    ((sweep.abs() / step).ceil() as usize).clamp(4, 512)
}

fn arc_points(c: (f64, f64), r: f64, a0: f64, a1: f64, scale: f64) -> Vec<(f64, f64)> {
    let mut sweep = a1 - a0;
    while sweep <= 0.0 {
        sweep += std::f64::consts::TAU;
    }
    let n = arc_segments(r * scale, sweep);
    (0..=n)
        .map(|i| {
            let t = a0 + sweep * i as f64 / n as f64;
            (c.0 + r * t.cos(), c.1 + r * t.sin())
        })
        .collect()
}

/// Pontos de um trecho com bulge (sem o ponto inicial).
fn bulge_points(p0: (f64, f64), p1: (f64, f64), bulge: f64, scale: f64, out: &mut Vec<(f64, f64)>) {
    if bulge.abs() < 1e-12 {
        out.push(p1);
        return;
    }
    let (dx, dy) = (p1.0 - p0.0, p1.1 - p0.1);
    let chord = (dx * dx + dy * dy).sqrt();
    if chord < 1e-15 {
        out.push(p1);
        return;
    }
    let theta = 4.0 * bulge.atan(); // varredura com sinal
    let r = chord / (2.0 * (theta / 2.0).sin()).abs();
    // Centro: ponto médio deslocado na normal da corda.
    let (mx, my) = ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0);
    let h = r * (theta / 2.0).cos().abs();
    let (nx, ny) = (-dy / chord, dx / chord);
    let side = if (theta.abs() < std::f64::consts::PI) == (bulge > 0.0) { 1.0 } else { -1.0 };
    let c = (mx + nx * h * side, my + ny * h * side);
    let a0 = (p0.1 - c.1).atan2(p0.0 - c.0);
    let n = arc_segments(r * scale, theta);
    for i in 1..=n {
        let t = a0 + theta * i as f64 / n as f64;
        out.push((c.0 + r * t.cos(), c.1 + r * t.sin()));
    }
    // Garante o ponto final exato.
    if let Some(last) = out.last_mut() {
        *last = p1;
    }
}

fn bulge_polyline(verts: &[((f64, f64), f64)], closed: bool, scale: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(verts.len() * 2);
    if verts.is_empty() {
        return out;
    }
    out.push(verts[0].0);
    let n = verts.len();
    let segs = if closed { n } else { n - 1 };
    for i in 0..segs {
        let (p0, b) = verts[i];
        let p1 = verts[(i + 1) % n].0;
        bulge_points(p0, p1, b, scale, &mut out);
    }
    out
}

/// Avalia uma B-spline (racional se houver pesos) pelo algoritmo de De Boor.
fn spline_points(degree: usize, knots: &[f64], ctrl: &[(f64, f64)], weights: &[f64], samples: usize) -> Option<Vec<(f64, f64)>> {
    let n = ctrl.len();
    if n < 2 || degree == 0 || knots.len() != n + degree + 1 {
        return None;
    }
    let w: Vec<f64> = if weights.len() == n { weights.to_vec() } else { vec![1.0; n] };
    let (u0, u1) = (knots[degree], knots[n]);
    if !(u1 > u0) {
        return None;
    }
    let mut out = Vec::with_capacity(samples + 1);
    for s in 0..=samples {
        let u = if s == samples { u1 } else { u0 + (u1 - u0) * s as f64 / samples as f64 };
        // Intervalo k com knots[k] <= u < knots[k+1].
        let mut k = degree;
        while k < n - 1 && u >= knots[k + 1] {
            k += 1;
        }
        let mut d: Vec<(f64, f64, f64)> = (0..=degree)
            .map(|j| {
                let i = k - degree + j;
                (ctrl[i].0 * w[i], ctrl[i].1 * w[i], w[i])
            })
            .collect();
        for r in 1..=degree {
            for j in (r..=degree).rev() {
                let i = k - degree + j;
                let denom = knots[i + degree + 1 - r] - knots[i];
                let alpha = if denom.abs() < 1e-300 { 0.0 } else { (u - knots[i]) / denom };
                d[j] = (
                    (1.0 - alpha) * d[j - 1].0 + alpha * d[j].0,
                    (1.0 - alpha) * d[j - 1].1 + alpha * d[j].1,
                    (1.0 - alpha) * d[j - 1].2 + alpha * d[j].2,
                );
            }
        }
        let p = d[degree];
        if p.2.abs() < 1e-300 {
            return None;
        }
        out.push((p.0 / p.2, p.1 / p.2));
    }
    Some(out)
}

fn ellipse_points(
    c: (f64, f64),
    major: (f64, f64),
    ratio: f64,
    p0: f64,
    p1: f64,
    scale: f64,
) -> Vec<(f64, f64)> {
    let minor = (-major.1 * ratio, major.0 * ratio);
    let mut sweep = p1 - p0;
    while sweep <= 1e-12 {
        sweep += std::f64::consts::TAU;
    }
    let r = (major.0 * major.0 + major.1 * major.1).sqrt();
    let n = arc_segments(r * scale, sweep);
    (0..=n)
        .map(|i| {
            let t = p0 + sweep * i as f64 / n as f64;
            let (ct, st) = (t.cos(), t.sin());
            (c.0 + major.0 * ct + minor.0 * st, c.1 + major.1 * ct + minor.1 * st)
        })
        .collect()
}

// ------------------------------------------------------------------ texto

/// Remove códigos de formatação do MTEXT (formatação básica) e substitui %%c/%%d/%%p.
pub fn plain_mtext(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.next() {
                Some('P') | Some('X') => out.push('\n'),
                Some('~') => out.push('\u{a0}'),
                Some('\\') => out.push('\\'),
                Some('{') => out.push('{'),
                Some('}') => out.push('}'),
                Some('L') | Some('l') | Some('O') | Some('o') | Some('K') | Some('k') => {}
                Some('S') => {
                    // \Snum^den; ou \Snum/den; ou \Snum#den;
                    let mut frac = String::new();
                    for c in chars.by_ref() {
                        if c == ';' {
                            break;
                        }
                        frac.push(c);
                    }
                    out.push_str(&frac.replace(['^', '#'], "/"));
                }
                Some('U') if chars.peek() == Some(&'+') => {
                    chars.next();
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Some(c) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        out.push(c);
                    }
                }
                Some(_) => {
                    // \f…; \H…; \C…; \W…; \Q…; \T…; \A…; \p…; → até o ';'
                    for c in chars.by_ref() {
                        if c == ';' {
                            break;
                        }
                    }
                }
                None => {}
            },
            '{' | '}' => {}
            _ => out.push(ch),
        }
    }
    percent_codes(&out)
}

pub fn percent_codes(s: &str) -> String {
    if !s.contains("%%") {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '%' && i + 2 < b.len() && b[i + 1] == '%' {
            match b[i + 2].to_ascii_lowercase() {
                'c' => out.push('Ø'),
                'd' => out.push('°'),
                'p' => out.push('±'),
                '%' => out.push('%'),
                'u' | 'o' => {}
                _ => {
                    // %%nnn: código de caractere
                    let digits: String = b[i + 2..].iter().take(3).take_while(|c| c.is_ascii_digit()).collect();
                    if digits.len() == 3 {
                        if let Some(c) = digits.parse::<u32>().ok().and_then(char::from_u32) {
                            out.push(c);
                        }
                        i += 5;
                        continue;
                    }
                    out.push_str("%%");
                    i += 2;
                    continue;
                }
            }
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

fn text_anchor(h: &str, v: &str) -> (f64, f64, bool) {
    // (ax, ay, usa ponto de alinhamento)
    let ax = match h {
        "Center" | "Middle" => 0.5,
        "Right" => 1.0,
        "Aligned" | "Fit" => 0.0,
        _ => 0.0,
    };
    let ay = match (h, v) {
        ("Middle", _) => 0.5,
        (_, "Top") => 1.0,
        (_, "Middle") => 0.5,
        (_, "Bottom") => -0.25,
        _ => 0.0,
    };
    let uses_alignment = !(h == "Left" && v == "Baseline") && h != "Aligned" && h != "Fit";
    (ax, ay, uses_alignment)
}

// -------------------------------------------------------------- construção

/// Contexto herdado de um INSERT.
#[derive(Clone)]
struct Ctx {
    xf: Xf,
    /// Camada efetiva para entidades na camada "0".
    layer0: Option<u32>,
    /// Cor resolvida do INSERT, para entidades PorBloco.
    byblock: [u8; 3],
    hosts: u32,
    depth: usize,
}

const MAX_DEPTH: usize = 32;

struct Builder<'a> {
    doc: &'a CadDocument,
    layer_index: HashMap<String, u32>,
    layers: Vec<SceneLayer>,
    host_sets: Vec<Vec<u32>>,
    host_index: HashMap<Vec<u32>, u32>,
    batches: HashMap<(u32, u32, [u8; 3], PrimKind), Vec<(f64, f64)>>,
    texts: Vec<SceneText>,
    stats: RenderStats,
    block_cache: HashMap<String, Option<(acadrust::Vector3, Vec<acadrust::Handle>)>>,
    bounds: [f64; 4],
    bg_dark: bool,
}

fn rgb_of(c: &Color) -> Option<[u8; 3]> {
    c.rgb().map(|(r, g, b)| [r, g, b])
}

impl<'a> Builder<'a> {
    fn new(doc: &'a CadDocument) -> Self {
        let mut b = Builder {
            doc,
            layer_index: HashMap::new(),
            layers: Vec::new(),
            host_sets: vec![vec![]],
            host_index: HashMap::from([(vec![], 0)]),
            batches: HashMap::new(),
            texts: Vec::new(),
            stats: RenderStats::default(),
            block_cache: HashMap::new(),
            bounds: [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY],
            bg_dark: true,
        };
        for l in doc.layers.iter() {
            b.add_layer(l);
        }
        b
    }

    fn add_layer(&mut self, l: &acadrust::Layer) -> u32 {
        let idx = self.layers.len() as u32;
        self.layers.push(SceneLayer {
            name: l.name.clone(),
            color: rgb_of(&l.color).unwrap_or([255, 255, 255]),
            color_label: crate::naming::color_label(&l.color),
            linetype: l.line_type.clone(),
            lineweight: crate::naming::lineweight_label(&l.line_weight),
            on: !l.flags.off,
            frozen: l.flags.frozen,
            locked: l.flags.locked,
            plottable: l.is_plottable,
            primitives: 0,
        });
        self.layer_index.insert(l.name.to_lowercase(), idx);
        idx
    }

    fn layer_of(&mut self, name: &str) -> u32 {
        if let Some(i) = self.layer_index.get(&name.to_lowercase()) {
            return *i;
        }
        // Camada usada mas ausente da tabela: cria com padrão (o relatório já avisa).
        let l = acadrust::Layer::new(name);
        self.add_layer(&l)
    }

    fn host_set(&mut self, parent: u32, add: u32) -> u32 {
        let mut v = self.host_sets[parent as usize].clone();
        if !v.contains(&add) {
            v.push(add);
            v.sort_unstable();
        }
        if let Some(i) = self.host_index.get(&v) {
            return *i;
        }
        let i = self.host_sets.len() as u32;
        self.host_sets.push(v.clone());
        self.host_index.insert(v, i);
        i
    }

    /// (camada efetiva, cor) de uma entidade dentro do contexto.
    fn resolve(&mut self, e: &EntityType, ctx: &Ctx) -> (u32, [u8; 3]) {
        let c = e.common();
        let layer = match ctx.layer0 {
            Some(l0) if c.layer == "0" => l0,
            _ => self.layer_of(&c.layer),
        };
        let color = match c.color {
            Color::ByLayer => self.layers[layer as usize].color,
            Color::ByBlock => ctx.byblock,
            Color::None => self.layers[layer as usize].color,
            ref other => rgb_of(other).unwrap_or([255, 255, 255]),
        };
        (layer, color)
    }

    fn note(&mut self, t: &str, f: impl FnOnce(&mut TypeStats)) {
        f(self.stats.by_type.entry(t.to_string()).or_default());
    }

    fn drawn(&mut self, t: &str) {
        self.note(t, |s| s.drawn += 1);
    }
    fn approx(&mut self, t: &str, why: &str) {
        self.note(t, |s| {
            s.approximated += 1;
            *s.notes.entry(why.to_string()).or_default() += 1;
        });
    }
    fn skip(&mut self, t: &str, why: &str) {
        self.note(t, |s| {
            s.skipped += 1;
            *s.notes.entry(why.to_string()).or_default() += 1;
        });
    }

    fn grow(&mut self, p: (f64, f64)) {
        if p.0.is_finite() && p.1.is_finite() {
            self.bounds[0] = self.bounds[0].min(p.0);
            self.bounds[1] = self.bounds[1].min(p.1);
            self.bounds[2] = self.bounds[2].max(p.0);
            self.bounds[3] = self.bounds[3].max(p.1);
        }
    }

    fn polyline(&mut self, layer: u32, hosts: u32, color: [u8; 3], pts: &[(f64, f64)], xf: &Xf) {
        if pts.len() < 2 {
            return;
        }
        let w: Vec<(f64, f64)> = pts.iter().map(|p| xf.apply(*p)).collect();
        for p in &w {
            self.grow(*p);
        }
        let v = self.batches.entry((layer, hosts, color, PrimKind::Lines)).or_default();
        for s in w.windows(2) {
            v.push(s[0]);
            v.push(s[1]);
        }
        self.stats.segments += w.len() - 1;
        self.layers[layer as usize].primitives += 1;
    }

    fn triangles(&mut self, layer: u32, hosts: u32, color: [u8; 3], tris: &[(f64, f64)], xf: &Xf) {
        if tris.len() < 3 {
            return;
        }
        let w: Vec<(f64, f64)> = tris.iter().map(|p| xf.apply(*p)).collect();
        for p in &w {
            self.grow(*p);
        }
        self.stats.triangles += w.len() / 3;
        self.batches.entry((layer, hosts, color, PrimKind::Triangles)).or_default().extend(w);
        self.layers[layer as usize].primitives += 1;
    }

    fn point(&mut self, layer: u32, hosts: u32, color: [u8; 3], p: (f64, f64), xf: &Xf) {
        let w = xf.apply(p);
        self.grow(w);
        self.batches.entry((layer, hosts, color, PrimKind::Points)).or_default().push(w);
        self.layers[layer as usize].primitives += 1;
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        layer: u32,
        hosts: u32,
        color: [u8; 3],
        pos: (f64, f64),
        height: f64,
        rotation: f64,
        width_factor: f64,
        ax: f64,
        ay: f64,
        multiline: bool,
        text: String,
        xf: &Xf,
    ) {
        if text.trim().is_empty() || !(height > 0.0) {
            return;
        }
        let w = xf.apply(pos);
        self.grow(w);
        let mut rot = rotation + xf.angle();
        let mut ax = ax;
        if xf.is_mirrored() {
            // MIRRTEXT=0: texto espelhado continua legível.
            rot = std::f64::consts::PI - rotation + xf.angle();
            ax = 1.0 - ax;
        }
        self.texts.push(SceneText {
            layer,
            hosts,
            color,
            x: w.0,
            y: w.1,
            height: height * xf.mean_scale(),
            rotation: rot,
            width_factor: if width_factor > 0.0 { width_factor } else { 1.0 },
            ax,
            ay,
            multiline,
            text,
        });
        self.stats.texts += 1;
        self.layers[layer as usize].primitives += 1;
    }

    fn block(&mut self, name: &str) -> Option<(acadrust::Vector3, Vec<acadrust::Handle>)> {
        if let Some(c) = self.block_cache.get(name) {
            return c.clone();
        }
        let r = self
            .doc
            .block_records
            .get(name)
            .map(|br| (br.base_point, br.entity_handles.clone()));
        self.block_cache.insert(name.to_string(), r.clone());
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn expand_block(&mut self, name: &str, local: Xf, e: &EntityType, ctx: &Ctx, layer: u32, color: [u8; 3], tname: &str) -> bool {
        if ctx.depth >= MAX_DEPTH {
            self.skip(tname, "aninhamento acima de 32 níveis");
            self.stats.max_depth_reached = MAX_DEPTH;
            return false;
        }
        let Some((base, handles)) = self.block(name) else {
            *self.stats.missing_blocks.entry(name.to_string()).or_default() += 1;
            self.skip(tname, "bloco não encontrado");
            return false;
        };
        let _ = e;
        let xf = ctx.xf.then_after(&local).then_after(&Xf::translate(-base.x, -base.y));
        let hosts = self.host_set(ctx.hosts, layer);
        let child = Ctx { xf, layer0: Some(layer), byblock: color, hosts, depth: ctx.depth + 1 };
        self.stats.max_depth_reached = self.stats.max_depth_reached.max(child.depth);
        let doc = self.doc;
        for h in handles {
            if let Some(ent) = doc.get_entity(h) {
                self.entity(ent, &child);
            }
        }
        true
    }

    fn entity(&mut self, e: &EntityType, ctx: &Ctx) {
        let tname = entity_type_name(e);
        let (layer, color) = self.resolve(e, ctx);
        let hosts = ctx.hosts;
        let sc = ctx.xf.mean_scale();
        match e {
            EntityType::Line(l) => {
                self.polyline(layer, hosts, color, &[(l.start.x, l.start.y), (l.end.x, l.end.y)], &ctx.xf);
                self.drawn(tname);
            }
            EntityType::Circle(c) => {
                let xf = ctx.xf.then_after(&Xf::ocs(&c.normal));
                let pts = arc_points((c.center.x, c.center.y), c.radius, 0.0, std::f64::consts::TAU, sc);
                self.polyline(layer, hosts, color, &pts, &xf);
                self.drawn(tname);
            }
            EntityType::Arc(a) => {
                let xf = ctx.xf.then_after(&Xf::ocs(&a.normal));
                let pts = arc_points((a.center.x, a.center.y), a.radius, a.start_angle, a.end_angle, sc);
                self.polyline(layer, hosts, color, &pts, &xf);
                self.drawn(tname);
            }
            EntityType::Ellipse(el) => {
                // ELLIPSE é definida em WCS.
                let pts = ellipse_points(
                    (el.center.x, el.center.y),
                    (el.major_axis.x, el.major_axis.y),
                    el.minor_axis_ratio,
                    el.start_parameter,
                    el.end_parameter,
                    sc,
                );
                let pts = if el.normal.z < 0.0 {
                    // Normal invertida: o sentido do parâmetro se inverte no plano XY.
                    ellipse_points(
                        (el.center.x, el.center.y),
                        (el.major_axis.x, el.major_axis.y),
                        -el.minor_axis_ratio,
                        el.start_parameter,
                        el.end_parameter,
                        sc,
                    )
                } else {
                    pts
                };
                self.polyline(layer, hosts, color, &pts, &ctx.xf);
                self.drawn(tname);
            }
            EntityType::LwPolyline(p) => {
                let xf = ctx.xf.then_after(&Xf::ocs(&p.normal));
                let verts: Vec<_> = p.vertices.iter().map(|v| ((v.location.x, v.location.y), v.bulge)).collect();
                let pts = bulge_polyline(&verts, p.is_closed, sc);
                self.polyline(layer, hosts, color, &pts, &xf);
                if p.constant_width > 0.0 || p.vertices.iter().any(|v| v.start_width > 0.0 || v.end_width > 0.0) {
                    self.approx(tname, "largura de polilinha exibida como linha fina");
                } else {
                    self.drawn(tname);
                }
            }
            EntityType::Polyline2D(p) => {
                let xf = ctx.xf.then_after(&Xf::ocs(&p.normal));
                let verts: Vec<_> = p.vertices.iter().map(|v| ((v.location.x, v.location.y), v.bulge)).collect();
                let closed = p.flags.bits() & 1 != 0;
                let pts = bulge_polyline(&verts, closed, sc);
                self.polyline(layer, hosts, color, &pts, &xf);
                self.drawn(tname);
            }
            EntityType::Polyline(p) => {
                let pts: Vec<_> = p.vertices.iter().map(|v| (v.location.x, v.location.y)).collect();
                let mut pts = pts;
                if p.flags.bits() & 1 != 0 && pts.len() > 2 {
                    pts.push(pts[0]);
                }
                self.polyline(layer, hosts, color, &pts, &ctx.xf);
                self.approx(tname, "polilinha 3D projetada em planta");
            }
            EntityType::Polyline3D(p) => {
                let pts: Vec<_> = p.vertices.iter().map(|v| (v.position.x, v.position.y)).collect();
                let mut pts = pts;
                if p.flags.closed && pts.len() > 2 {
                    pts.push(pts[0]);
                }
                self.polyline(layer, hosts, color, &pts, &ctx.xf);
                self.approx(tname, "polilinha 3D projetada em planta");
            }
            EntityType::Spline(s) => {
                let ctrl: Vec<_> = s.control_points.iter().map(|p| (p.x, p.y)).collect();
                let samples = (ctrl.len() * 12).clamp(32, 4000);
                match spline_points(s.degree.max(0) as usize, &s.knots, &ctrl, &s.weights, samples) {
                    Some(pts) => {
                        self.polyline(layer, hosts, color, &pts, &ctx.xf);
                        self.drawn(tname);
                    }
                    None if s.fit_points.len() >= 2 => {
                        let pts: Vec<_> = s.fit_points.iter().map(|p| (p.x, p.y)).collect();
                        self.polyline(layer, hosts, color, &pts, &ctx.xf);
                        self.approx(tname, "spline só com pontos de ajuste: ligados por retas");
                    }
                    None => self.skip(tname, "spline sem nós/pontos de controle válidos"),
                }
            }
            EntityType::Point(p) => {
                self.point(layer, hosts, color, (p.location.x, p.location.y), &ctx.xf);
                self.drawn(tname);
            }
            EntityType::Solid(s) => {
                let xf = ctx.xf.then_after(&Xf::ocs(&s.normal));
                let c = [s.first_corner, s.second_corner, s.third_corner, s.fourth_corner].map(|v| (v.x, v.y));
                // Ordem do SOLID: 1-2-4-3.
                let tris = [c[0], c[1], c[2], c[1], c[3], c[2]];
                self.triangles(layer, hosts, color, &tris, &xf);
                self.drawn(tname);
            }
            EntityType::Hatch(h) => self.hatch(h, layer, hosts, color, ctx, tname),
            EntityType::Text(t) => {
                let xf = ctx.xf.then_after(&Xf::ocs(&t.normal));
                let hn = format!("{:?}", t.horizontal_alignment);
                let vn = format!("{:?}", t.vertical_alignment);
                let (ax, ay, use_align) = text_anchor(&hn, &vn);
                let pos = match (use_align, t.alignment_point) {
                    (true, Some(p)) => (p.x, p.y),
                    _ => (t.insertion_point.x, t.insertion_point.y),
                };
                let txt = percent_codes(&t.value);
                self.text(layer, hosts, color, pos, t.height, t.rotation, t.width_factor, ax, ay, false, txt, &xf);
                self.note_text(tname, &hn);
            }
            EntityType::AttributeEntity(a) => self.attrib(a, layer, hosts, color, ctx),
            EntityType::AttributeDefinition(a) => {
                if ctx.layer0.is_some() {
                    // Dentro de bloco: o AutoCAD não mostra ATTDEF nas inserções.
                    self.note(tname, |s| {
                        s.drawn += 1;
                        *s.notes.entry("dentro de bloco: não exibida (o ATTRIB aparece)".into()).or_default() += 1;
                    });
                } else {
                    let xf = ctx.xf.then_after(&Xf::ocs(&a.normal));
                    let (ax, ay, _) = text_anchor(&format!("{:?}", a.horizontal_alignment), &format!("{:?}", a.vertical_alignment));
                    self.text(layer, hosts, color, (a.insertion_point.x, a.insertion_point.y), a.height, a.rotation, a.width_factor, ax, ay, false, a.tag.clone(), &xf);
                    self.drawn(tname);
                }
            }
            EntityType::MText(m) => {
                let xf = ctx.xf.then_after(&Xf::ocs(&m.normal));
                let n = m.attachment_point as i32;
                let ax = match (n - 1) % 3 {
                    0 => 0.0,
                    1 => 0.5,
                    _ => 1.0,
                };
                let ay = match (n - 1) / 3 {
                    0 => 1.0,
                    1 => 0.5,
                    _ => 0.0,
                };
                let rot = match m.dwg_x_direction {
                    Some(d) if d.x != 0.0 || d.y != 0.0 => d.y.atan2(d.x),
                    _ => m.rotation,
                };
                let txt = plain_mtext(&m.value);
                self.text(layer, hosts, color, (m.insertion_point.x, m.insertion_point.y), m.height, rot, 1.0, ax, ay, true, txt, &xf);
                if m.rectangle_width > 0.0 {
                    self.approx(tname, "formatação básica; sem quebra automática de linha");
                } else {
                    self.approx(tname, "formatação básica");
                }
            }
            EntityType::Insert(ins) => {
                let base = ctx.xf.then_after(&Xf::ocs(&ins.normal));
                let cols = ins.column_count.max(1) as usize;
                let rows = ins.row_count.max(1) as usize;
                let mut ok = true;
                for r in 0..rows {
                    for c in 0..cols {
                        let off = (c as f64 * ins.column_spacing, r as f64 * ins.row_spacing);
                        let local = Xf::translate(ins.insert_point.x, ins.insert_point.y)
                            .then_after(&Xf::rotate(ins.rotation))
                            .then_after(&Xf::translate(off.0, off.1))
                            .then_after(&Xf::scale(ins.x_scale(), ins.y_scale()));
                        let c2 = Ctx { xf: base, ..ctx.clone() };
                        ok &= self.expand_block(&ins.block_name, local, e, &c2, layer, color, tname);
                        self.stats.inserts_expanded += 1;
                    }
                }
                // A camada do INSERT controla o bloco (congelar esconde tudo): conta como uso.
                self.layers[layer as usize].primitives += 1;
                if ok {
                    self.drawn(tname);
                }
                // Atributos: coordenadas já em WCS (OCS do próprio ATTRIB), fora da transformação do bloco.
                for a in &ins.attributes {
                    let wrapped = EntityType::AttributeEntity(a.clone());
                    let (al, acol) = {
                        let c3 = Ctx { layer0: Some(layer), byblock: color, ..ctx.clone() };
                        let r = self.resolve(&wrapped, &c3);
                        (r.0, r.1)
                    };
                    let hs = self.host_set(ctx.hosts, layer);
                    let c4 = Ctx { hosts: hs, ..ctx.clone() };
                    self.attrib(a, al, hs, acol, &c4);
                }
            }
            EntityType::Dimension(d) => {
                let name = d.base().block_name.clone();
                if name.is_empty() {
                    self.skip(tname, "cota sem bloco de geometria");
                } else if self.expand_block(&name, Xf::ID, e, ctx, layer, color, tname) {
                    self.layers[layer as usize].primitives += 1;
                    self.drawn(tname);
                }
            }
            EntityType::Table(t) => {
                if t.block_name.is_empty() {
                    self.skip(tname, "tabela sem bloco de geometria");
                } else {
                    let ang = t.horizontal_direction.y.atan2(t.horizontal_direction.x);
                    let local = Xf::translate(t.insertion_point.x, t.insertion_point.y).then_after(&Xf::rotate(ang));
                    if self.expand_block(&t.block_name.clone(), local, e, ctx, layer, color, tname) {
                        self.approx(tname, "desenhada pelo bloco anônimo da tabela");
                    }
                }
            }
            EntityType::Leader(l) => {
                let pts: Vec<_> = l.vertices.iter().map(|v| (v.x, v.y)).collect();
                self.polyline(layer, hosts, color, &pts, &ctx.xf);
                if l.arrow_enabled {
                    self.approx(tname, "seta não desenhada");
                } else {
                    self.drawn(tname);
                }
            }
            EntityType::MultiLeader(m) => {
                let cx = &m.context;
                for root in &cx.leader_roots {
                    for line in &root.lines {
                        let mut pts: Vec<_> = line.points.iter().map(|v| (v.x, v.y)).collect();
                        pts.push((root.connection_point.x, root.connection_point.y));
                        self.polyline(layer, hosts, color, &pts, &ctx.xf);
                    }
                }
                if cx.has_text_contents && !cx.text_string.is_empty() {
                    let rot = if cx.text_direction.x != 0.0 || cx.text_direction.y != 0.0 {
                        cx.text_direction.y.atan2(cx.text_direction.x)
                    } else {
                        cx.text_rotation
                    };
                    let tcolor = rgb_of(&cx.text_color).unwrap_or(color);
                    let txt = plain_mtext(&cx.text_string);
                    self.text(layer, hosts, tcolor, (cx.text_location.x, cx.text_location.y), cx.text_height, rot, 1.0, 0.0, 1.0, true, txt, &ctx.xf);
                }
                self.approx(tname, "linhas e texto; setas e bloco de conteúdo não desenhados");
            }
            EntityType::Viewport(_) => self.skip(tname, "viewport pertence ao espaço papel"),
            EntityType::Wipeout(_) => self.skip(tname, "máscara (wipeout) ainda não suportada"),
            EntityType::RasterImage(_) => self.skip(tname, "imagem raster ainda não suportada"),
            EntityType::XLine(_) | EntityType::Ray(_) => self.skip(tname, "linha infinita não exibida"),
            EntityType::MLine(_) => self.skip(tname, "MLINE ainda não suportada"),
            EntityType::Shape(_) => self.skip(tname, "SHAPE depende de arquivo SHX"),
            EntityType::Tolerance(_) => self.skip(tname, "tolerância geométrica ainda não suportada"),
            EntityType::Block(_) | EntityType::BlockEnd(_) | EntityType::Seqend(_) => {}
            EntityType::Unknown(_) | EntityType::Extended(_) => self.skip(tname, "tipo desconhecido/proxy"),
            _ => self.skip(tname, "fora do escopo (3D/OLE/luz)"),
        }
    }

    fn note_text(&mut self, tname: &str, h: &str) {
        if h == "Aligned" || h == "Fit" {
            self.approx(tname, "alinhamento Ajustado/Alinhado exibido como à esquerda");
        } else {
            self.approx(tname, "fonte substituta (SHX/TTF ainda não carregadas)");
        }
    }

    fn attrib(&mut self, a: &acadrust::entities::AttributeEntity, layer: u32, hosts: u32, color: [u8; 3], ctx: &Ctx) {
        if a.flags.invisible {
            self.note("ATTRIB", |s| {
                s.drawn += 1;
                *s.notes.entry("atributo invisível (não exibido, como no AutoCAD)".into()).or_default() += 1;
            });
            return;
        }
        let hn = format!("{:?}", a.horizontal_alignment);
        let vn = format!("{:?}", a.vertical_alignment);
        let (ax, ay, use_align) = text_anchor(&hn, &vn);
        let pos = if use_align { (a.alignment_point.x, a.alignment_point.y) } else { (a.insertion_point.x, a.insertion_point.y) };
        // ATTRIB de INSERT: coordenadas em WCS/OCS próprio; o contexto só fornece camada/cor.
        let xf = Xf::ocs(&a.normal);
        let _ = ctx;
        let txt = if a.is_multiline { plain_mtext(&a.value) } else { percent_codes(&a.value) };
        self.text(layer, hosts, color, pos, a.height, a.rotation, a.width_factor, ax, ay, a.is_multiline, txt, &xf);
        self.note_text("ATTRIB", &hn);
    }

    fn hatch(&mut self, h: &acadrust::entities::hatch::Hatch, layer: u32, hosts: u32, color: [u8; 3], ctx: &Ctx, tname: &str) {
        let xf = ctx.xf.then_after(&Xf::ocs(&h.normal));
        let sc = ctx.xf.mean_scale();
        let mut loops: Vec<Vec<(f64, f64)>> = Vec::new();
        for path in &h.paths {
            let mut pts: Vec<(f64, f64)> = Vec::new();
            for edge in &path.edges {
                match edge {
                    BoundaryEdge::Polyline(pe) => {
                        let verts: Vec<_> = pe.vertices.iter().map(|v| ((v.x, v.y), v.z)).collect();
                        let p = bulge_polyline(&verts, pe.is_closed, sc);
                        pts.extend(p);
                    }
                    BoundaryEdge::Line(le) => {
                        if pts.last() != Some(&(le.start.x, le.start.y)) {
                            pts.push((le.start.x, le.start.y));
                        }
                        pts.push((le.end.x, le.end.y));
                    }
                    BoundaryEdge::CircularArc(ae) => {
                        let (a0, a1) = if ae.counter_clockwise { (ae.start_angle, ae.end_angle) } else { (-ae.start_angle, -ae.end_angle) };
                        let mut p = arc_points((0.0, 0.0), ae.radius, a0, a1, sc);
                        if (a1 - a0 - std::f64::consts::TAU).abs() < 1e-9 || (a1 - a0).abs() < 1e-12 {
                            p = arc_points((0.0, 0.0), ae.radius, 0.0, std::f64::consts::TAU, sc);
                        }
                        for q in p {
                            let q = if ae.counter_clockwise { q } else { (q.0, -q.1) };
                            pts.push((ae.center.x + q.0, ae.center.y + q.1));
                        }
                    }
                    BoundaryEdge::EllipticArc(ee) => {
                        let ratio = if ee.counter_clockwise { ee.minor_axis_ratio } else { -ee.minor_axis_ratio };
                        let (p0, p1) = if ee.counter_clockwise { (ee.start_angle, ee.end_angle) } else { (-ee.start_angle, -ee.end_angle) };
                        pts.extend(ellipse_points(
                            (ee.center.x, ee.center.y),
                            (ee.major_axis_endpoint.x, ee.major_axis_endpoint.y),
                            ratio,
                            p0,
                            p1,
                            sc,
                        ));
                    }
                    BoundaryEdge::Spline(se) => {
                        let ctrl: Vec<_> = se.control_points.iter().map(|p| (p.x, p.y)).collect();
                        let w: Vec<f64> = if se.rational { se.control_points.iter().map(|p| if p.z > 0.0 { p.z } else { 1.0 }).collect() } else { vec![] };
                        match spline_points(se.degree.max(0) as usize, &se.knots, &ctrl, &w, (ctrl.len() * 12).clamp(32, 2000)) {
                            Some(p) => pts.extend(p),
                            None => pts.extend(se.fit_points.iter().map(|p| (p.x, p.y))),
                        }
                    }
                }
            }
            if pts.len() >= 3 {
                loops.push(pts);
            }
        }
        if loops.is_empty() {
            self.skip(tname, "hachura sem contorno");
            return;
        }
        if h.is_solid {
            let tris = triangulate(&loops);
            if tris.is_empty() {
                for l in &loops {
                    let mut c = l.clone();
                    c.push(l[0]);
                    self.polyline(layer, hosts, color, &c, &xf);
                }
                self.approx(tname, "preenchimento sólido não triangulado: exibido só o contorno");
            } else {
                self.triangles(layer, hosts, color, &tris, &xf);
                self.drawn(tname);
            }
        } else {
            for l in &loops {
                let mut c = l.clone();
                c.push(l[0]);
                self.polyline(layer, hosts, color, &c, &xf);
            }
            self.approx(tname, "padrão de hachura ainda não desenhado: exibido o contorno");
        }
    }
}

fn area(l: &[(f64, f64)]) -> f64 {
    let mut a = 0.0;
    for i in 0..l.len() {
        let (p, q) = (l[i], l[(i + 1) % l.len()]);
        a += p.0 * q.1 - q.0 * p.1;
    }
    a / 2.0
}

fn inside(p: (f64, f64), poly: &[(f64, f64)]) -> bool {
    let mut c = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if ((a.1 > p.1) != (b.1 > p.1)) && (p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0) {
            c = !c;
        }
        j = i;
    }
    c
}

/// Triangula os laços da hachura: laço contido num número ímpar de outros é furo.
fn triangulate(loops: &[Vec<(f64, f64)>]) -> Vec<(f64, f64)> {
    let depth: Vec<usize> = (0..loops.len())
        .map(|i| (0..loops.len()).filter(|&j| j != i && inside(loops[i][0], &loops[j])).count())
        .collect();
    let mut out = Vec::new();
    for (i, outer) in loops.iter().enumerate() {
        if depth[i] % 2 == 1 {
            continue;
        }
        // Furos diretos deste contorno.
        let holes: Vec<&Vec<(f64, f64)>> = (0..loops.len())
            .filter(|&j| depth[j] == depth[i] + 1 && inside(loops[j][0], outer))
            .map(|j| &loops[j])
            .collect();
        let mut data: Vec<f64> = Vec::new();
        let mut hole_idx: Vec<usize> = Vec::new();
        for p in outer {
            data.push(p.0);
            data.push(p.1);
        }
        for h in holes {
            hole_idx.push(data.len() / 2);
            for p in h {
                data.push(p.0);
                data.push(p.1);
            }
        }
        if area(outer).abs() < 1e-18 {
            continue;
        }
        if let Ok(idx) = earcutr::earcut(&data, &hole_idx, 2) {
            for k in idx {
                out.push((data[k * 2], data[k * 2 + 1]));
            }
        }
    }
    out
}

// ---------------------------------------------------------------- API

/// Monta a cena do espaço modelo.
pub fn build_model_scene(doc: &CadDocument) -> Scene {
    let mut b = Builder::new(doc);
    b.bg_dark = true;
    let root = Ctx { xf: Xf::ID, layer0: None, byblock: [255, 255, 255], hosts: 0, depth: 0 };
    let ents: Vec<&EntityType> = doc.model_space_entities().collect();
    for e in ents {
        b.entity(e, &root);
    }
    b.stats.global_notes = vec![
        "Tipos de linha (tracejado, eixo…) exibidos contínuos nesta versão.".into(),
        "Espessuras de linha exibidas com 1 pixel.".into(),
        "Textos com fonte substituta; SHX ainda não são carregadas.".into(),
        "Só o espaço modelo; layouts no próximo passo.".into(),
    ];

    let bounds = if b.bounds[0].is_finite() { Some(b.bounds) } else { None };
    let origin = bounds.map(|x| [(x[0] + x[2]) / 2.0, (x[1] + x[3]) / 2.0]).unwrap_or([0.0, 0.0]);

    // Ordem estável: por camada, depois cor, depois tipo (triângulos primeiro, por baixo das linhas).
    let mut keys: Vec<_> = b.batches.keys().cloned().collect();
    keys.sort_by_key(|k| (k.3 != PrimKind::Triangles, k.0, k.1, k.2, k.3 == PrimKind::Points));
    let mut vertices: Vec<f32> = Vec::new();
    let mut batches = Vec::with_capacity(keys.len());
    for k in keys {
        let pts = &b.batches[&k];
        let offset = vertices.len() as u32;
        vertices.reserve(pts.len() * 2);
        for p in pts {
            vertices.push((p.0 - origin[0]) as f32);
            vertices.push((p.1 - origin[1]) as f32);
        }
        batches.push(BatchMeta { layer: k.0, hosts: k.1, color: k.2, kind: k.3, offset, len: vertices.len() as u32 - offset });
    }
    let mut texts = std::mem::take(&mut b.texts);
    for t in &mut texts {
        t.x -= origin[0];
        t.y -= origin[1];
    }

    Scene {
        meta: SceneMeta {
            schema: 1,
            space: "model".into(),
            origin,
            bounds,
            layers: b.layers,
            host_sets: b.host_sets,
            batches,
            texts,
            stats: b.stats,
        },
        vertices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn bulge_de_90_graus() {
        // Quarto de círculo de (0,0) a (1,1) com centro (0,1)? bulge = tan(22,5°) anti-horário.
        let b = (std::f64::consts::FRAC_PI_2 / 4.0).tan();
        let mut out = vec![];
        bulge_points((1.0, 0.0), (0.0, 1.0), b, 1.0, &mut out);
        assert!(close(*out.last().unwrap(), (0.0, 1.0)));
        // Todos os pontos no círculo unitário centrado na origem.
        for p in &out {
            assert!(((p.0 * p.0 + p.1 * p.1).sqrt() - 1.0).abs() < 1e-9, "{p:?}");
        }
        // Bulge negativo: arco do outro lado, centro (1,1).
        let mut out = vec![];
        bulge_points((1.0, 0.0), (0.0, 1.0), -b, 1.0, &mut out);
        for p in &out {
            let d = ((p.0 - 1.0).powi(2) + (p.1 - 1.0).powi(2)).sqrt();
            assert!((d - 1.0).abs() < 1e-9, "{p:?}");
        }
    }

    #[test]
    fn semicirculo_bulge_1() {
        let mut out = vec![];
        bulge_points((0.0, 0.0), (2.0, 0.0), 1.0, 1.0, &mut out);
        for p in &out {
            assert!((((p.0 - 1.0).powi(2) + p.1.powi(2)).sqrt() - 1.0).abs() < 1e-9);
            assert!(p.1 <= 1e-9, "bulge positivo vai no sentido anti-horário: abaixo da corda");
        }
    }

    #[test]
    fn ocs_normal_negativa_espelha_x() {
        let xf = Xf::ocs(&Vector3::new(0.0, 0.0, -1.0));
        assert!(close(xf.apply((2.0, 3.0)), (-2.0, 3.0)));
        assert!(close(Xf::ocs(&Vector3::new(0.0, 0.0, 1.0)).apply((2.0, 3.0)), (2.0, 3.0)));
    }

    #[test]
    fn spline_bezier_quadratica() {
        // Grau 2, nós clamped: curva de Bézier quadrática.
        let pts = spline_points(2, &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0], &[(0.0, 0.0), (1.0, 2.0), (2.0, 0.0)], &[], 4).unwrap();
        assert!(close(pts[0], (0.0, 0.0)));
        assert!(close(pts[2], (1.0, 1.0)));
        assert!(close(pts[4], (2.0, 0.0)));
    }

    #[test]
    fn mtext_basico() {
        assert_eq!(plain_mtext("{\\fArial|b0;Divisão}\\Pde piso"), "Divisão\nde piso");
        assert_eq!(plain_mtext("\\H2.5;Ø%%c 10%%d"), "ØØ 10°");
        assert_eq!(plain_mtext("1\\S1^2;"), "11/2");
        assert_eq!(plain_mtext("a\\\\b"), "a\\b");
    }

    #[test]
    fn furos_na_hachura() {
        let outer = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let hole = vec![(4.0, 4.0), (6.0, 4.0), (6.0, 6.0), (4.0, 6.0)];
        let tris = triangulate(&[outer, hole]);
        let a: f64 = tris.chunks(3).map(|t| area(t).abs()).sum();
        assert!((a - 96.0).abs() < 1e-9, "área {a}");
    }

    #[test]
    fn transformacao_composta() {
        let xf = Xf::translate(10.0, 0.0).then_after(&Xf::rotate(std::f64::consts::FRAC_PI_2)).then_after(&Xf::scale(2.0, 2.0));
        assert!(close(xf.apply((1.0, 0.0)), (10.0, 2.0)));
    }
}
