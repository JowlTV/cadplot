//! Desenhos sintéticos para testes.
//!
//! Este crate é o ÚNICO lugar do repositório que grava DWG/DXF, e só existe
//! para gerar fixtures em pastas temporárias durante os testes. O app e o
//! núcleo não dependem dele (é `dev-dependency`).
//!
//! A planta sintética imita os arquivos reais de arquitetura/paisagismo:
//! camadas com prefixo e acentos, blocos aninhados (vaso dentro de canteiro),
//! cotas, hachura, textos acentuados e um layout A3 com viewport 1:50.

use std::path::{Path, PathBuf};

use acadrust::entities::dimension::DimensionLinear;
use acadrust::entities::hatch::{BoundaryEdge, BoundaryPath, Hatch, PolylineEdge};
use acadrust::entities::*;
use acadrust::objects::ObjectType;
use acadrust::tables::layer::Layer;
use acadrust::types::{Color, DxfVersion, LineWeight, Vector2, Vector3};
pub mod r12;

use acadrust::{BlockRecord, CadDocument, DwgWriter, DxfWriter, TableEntry, TextStyle};

/// Nomes de camada usados na planta (com acentos de propósito).
pub const LAYERS: &[&str] = &[
    "PA-COMENTÁRIO",
    "ARQ-PAREDE",
    "ARQ-MURETA",
    "PISO-DIVISÃO",
    "HID-ÁGUA FRIA",
    "AUX-CORTE",
    "VEG-ÁRVORES",
    "COT-COTAS",
];

pub const TEXT_ACENTUADO: &str = "Mureta de proteção — ação, coração, pé, Ø40";
pub const MTEXT_ACENTUADO: &str = "Divisão de piso\\Pcerâmica 60×60";

/// O que a planta contém, para os testes conferirem.
#[derive(Debug, Clone, Default)]
pub struct Expected {
    pub model_lines: usize,
    pub model_lwpolylines: usize,
    pub model_arcs: usize,
    pub model_circles: usize,
    pub model_ellipses: usize,
    pub model_splines: usize,
    pub model_texts: usize,
    pub model_mtexts: usize,
    pub model_hatches: usize,
    pub model_inserts: usize,
    pub model_dimensions: usize,
    pub block_names: Vec<&'static str>,
    /// Viewport de detalhe no layout "Folha A3": escala papel/modelo.
    pub viewport_scale: f64,
}

fn layer(doc: &mut CadDocument, name: &str, color: u8, lw: Option<i16>, f: impl FnOnce(&mut Layer)) {
    let mut l = Layer::new(name);
    l.color = Color::Index(color);
    if let Some(v) = lw {
        l.line_weight = LineWeight::Value(v);
    }
    f(&mut l);
    let h = doc.allocate_handle();
    l.set_handle(h);
    doc.layers.add(l).expect("camada");
}

fn on(mut e: EntityType, layer: &str) -> EntityType {
    e.common_mut().layer = layer.to_string();
    e
}

fn add(doc: &mut CadDocument, e: EntityType, layer_name: &str) {
    doc.add_entity(on(e, layer_name)).expect("entidade");
}

fn new_block(doc: &mut CadDocument, name: &str) -> acadrust::Handle {
    let mut br = BlockRecord::new(name);
    let h = doc.allocate_handle();
    br.set_handle(h);
    br.block_entity_handle = doc.allocate_handle();
    br.block_end_handle = doc.allocate_handle();
    doc.block_records.add(br).expect("bloco");
    h
}

fn add_to_block(doc: &mut CadDocument, owner: acadrust::Handle, e: EntityType, layer_name: &str) {
    let mut e = on(e, layer_name);
    e.common_mut().owner_handle = owner;
    doc.add_entity(e).expect("entidade do bloco");
}

/// Planta sintética. Unidade: milímetros. Área 10 m × 8 m.
pub fn planta(version: DxfVersion) -> (CadDocument, Expected) {
    let mut doc = CadDocument::with_version(version);
    let mut exp = Expected::default();
    doc.header.insertion_units = 4; // mm
    doc.header.measurement = 1;
    doc.header.current_annotation_scale = "1:50".into();
    doc.header.annotation_scale_value = 0.02;
    // No DWG o CANNOSCALE vive no AcDbVariableDictionary, não no cabeçalho.
    doc.set_drawing_variable("CANNOSCALE", "1:50");
    if !matches!(version, DxfVersion::AC1021 | DxfVersion::AC1024 | DxfVersion::AC1027 | DxfVersion::AC1032) {
        doc.header.code_page = "ANSI_1252".into();
    }

    layer(&mut doc, "PA-COMENTÁRIO", 1, None, |_| {});
    layer(&mut doc, "ARQ-PAREDE", 7, Some(35), |_| {});
    layer(&mut doc, "ARQ-MURETA", 8, Some(25), |_| {});
    layer(&mut doc, "PISO-DIVISÃO", 253, Some(9), |l| l.flags.off = true);
    layer(&mut doc, "HID-ÁGUA FRIA", 5, None, |l| l.flags.frozen = true);
    layer(&mut doc, "AUX-CORTE", 2, None, |l| l.flags.locked = true);
    layer(&mut doc, "VEG-ÁRVORES", 3, None, |_| {});
    layer(&mut doc, "COT-COTAS", 4, None, |_| {});

    // Estilos de texto: um SHX (fonte não presente → deve aparecer como faltando)
    // e um TrueType.
    let mut romans = TextStyle::new("ROMANS");
    romans.font_file = "romans.shx".into();
    romans.set_handle(doc.allocate_handle());
    doc.text_styles.add(romans).expect("estilo");
    let mut arial = TextStyle::new("ARIAL");
    arial.font_file = "arial.ttf".into();
    arial.set_handle(doc.allocate_handle());
    doc.text_styles.add(arial).expect("estilo");

    // Paredes: contorno 10000 × 8000 com 4 LINE.
    let pts = [(0.0, 0.0), (10_000.0, 0.0), (10_000.0, 8_000.0), (0.0, 8_000.0)];
    for i in 0..4 {
        let (a, b) = (pts[i], pts[(i + 1) % 4]);
        add(&mut doc, EntityType::Line(Line::from_coords(a.0, a.1, 0.0, b.0, b.1, 0.0)), "ARQ-PAREDE");
        exp.model_lines += 1;
    }
    // Mureta com canto arredondado (bulge = tan(θ/4); 90° → 0,41421356…).
    let mut mureta = LwPolyline::new();
    mureta.add_point(Vector2::new(1_000.0, 1_000.0));
    mureta.add_point_with_bulge(Vector2::new(4_000.0, 1_000.0), (std::f64::consts::FRAC_PI_2 / 4.0).tan());
    mureta.add_point(Vector2::new(4_500.0, 1_500.0));
    mureta.add_point(Vector2::new(4_500.0, 3_000.0));
    add(&mut doc, EntityType::LwPolyline(mureta), "ARQ-MURETA");
    exp.model_lwpolylines += 1;

    // Divisão de piso: grade de 60 cm em LINE finas.
    for i in 1..5 {
        let x = 6_000.0 + 600.0 * i as f64;
        add(&mut doc, EntityType::Line(Line::from_coords(x, 500.0, 0.0, x, 3_500.0, 0.0)), "PISO-DIVISÃO");
        exp.model_lines += 1;
    }
    // Hidráulica (camada congelada), linha auxiliar de corte (camada travada).
    add(&mut doc, EntityType::Line(Line::from_coords(500.0, 7_500.0, 0.0, 9_500.0, 7_500.0, 0.0)), "HID-ÁGUA FRIA");
    add(&mut doc, EntityType::Line(Line::from_coords(5_000.0, -500.0, 0.0, 5_000.0, 8_500.0, 0.0)), "AUX-CORTE");
    exp.model_lines += 2;

    add(&mut doc, EntityType::Arc(Arc::from_coords(2_000.0, 6_000.0, 0.0, 800.0, 0.0, std::f64::consts::PI)), "VEG-ÁRVORES");
    exp.model_arcs += 1;
    add(&mut doc, EntityType::Circle(Circle::from_coords(8_000.0, 6_000.0, 0.0, 1_200.0)), "VEG-ÁRVORES");
    exp.model_circles += 1;
    add(
        &mut doc,
        EntityType::Ellipse(Ellipse::from_center_axes(
            Vector3::new(5_000.0, 5_000.0, 0.0),
            Vector3::new(1_000.0, 0.0, 0.0),
            0.5,
        )),
        "VEG-ÁRVORES",
    );
    exp.model_ellipses += 1;
    add(
        &mut doc,
        EntityType::Spline(Spline::from_fit_points(vec![
            Vector3::new(500.0, 4_000.0, 0.0),
            Vector3::new(1_500.0, 4_800.0, 0.0),
            Vector3::new(2_500.0, 4_200.0, 0.0),
            Vector3::new(3_500.0, 4_900.0, 0.0),
        ])),
        "VEG-ÁRVORES",
    );
    exp.model_splines += 1;

    let mut t = Text::with_value(TEXT_ACENTUADO, Vector3::new(1_000.0, 500.0, 0.0));
    t.height = 125.0;
    add(&mut doc, EntityType::Text(t), "PA-COMENTÁRIO");
    exp.model_texts += 1;
    add(
        &mut doc,
        EntityType::MText(Box::new(MText::with_value(MTEXT_ACENTUADO, Vector3::new(6_000.0, 4_000.0, 0.0)))),
        "PA-COMENTÁRIO",
    );
    exp.model_mtexts += 1;

    let mut hatch = Hatch::solid();
    let mut path = BoundaryPath::new();
    path.add_edge(BoundaryEdge::Polyline(PolylineEdge::new(
        vec![
            Vector2::new(6_000.0, 500.0),
            Vector2::new(8_400.0, 500.0),
            Vector2::new(8_400.0, 3_500.0),
            Vector2::new(6_000.0, 3_500.0),
        ],
        true,
    )));
    hatch.add_path(path);
    add(&mut doc, EntityType::Hatch(Box::new(hatch)), "PISO-DIVISÃO");
    exp.model_hatches += 1;

    let dim = Dimension::Linear(DimensionLinear::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(10_000.0, 0.0, 0.0),
    ));
    add(&mut doc, EntityType::Dimension(Box::new(dim)), "COT-COTAS");
    exp.model_dimensions += 1;

    // Blocos: VASO-Ø40 (círculo + cruz) aninhado em CANTEIRO.
    let vaso = new_block(&mut doc, "VASO-Ø40");
    add_to_block(&mut doc, vaso, EntityType::Circle(Circle::from_coords(0.0, 0.0, 0.0, 200.0)), "0");
    add_to_block(&mut doc, vaso, EntityType::Line(Line::from_coords(-200.0, 0.0, 0.0, 200.0, 0.0, 0.0)), "0");
    let canteiro = new_block(&mut doc, "CANTEIRO");
    add_to_block(
        &mut doc,
        canteiro,
        EntityType::LwPolyline(LwPolyline::from_points(vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(1_200.0, 0.0),
            Vector2::new(1_200.0, 600.0),
            Vector2::new(0.0, 600.0),
        ])),
        "VEG-ÁRVORES",
    );
    for x in [300.0, 900.0] {
        add_to_block(&mut doc, canteiro, EntityType::Insert(Insert::new("VASO-Ø40", Vector3::new(x, 300.0, 0.0))), "VEG-ÁRVORES");
    }
    exp.block_names = vec!["VASO-Ø40", "CANTEIRO"];
    for (x, y, rot, sc) in [(1_000.0, 2_000.0, 0.0, 1.0), (1_000.0, 3_000.0, 30.0_f64.to_radians(), 1.5)] {
        let mut ins = Insert::new("CANTEIRO", Vector3::new(x, y, 0.0)).with_uniform_scale(sc);
        ins.rotation = rot;
        add(&mut doc, EntityType::Insert(ins), "VEG-ÁRVORES");
        exp.model_inserts += 1;
    }
    add(&mut doc, EntityType::Insert(Insert::new("VASO-Ø40", Vector3::new(9_000.0, 1_000.0, 0.0))), "VEG-ÁRVORES");
    exp.model_inserts += 1;

    // Layout A3 paisagem com viewport 1:50 (papel em mm, modelo em mm).
    if version != DxfVersion::AC1009 {
        let lh = doc.add_layout("Folha A3").expect("layout");
        if let Some(ObjectType::Layout(l)) = doc.objects.get_mut(&lh) {
            l.paper_size = "ISO_full_bleed_A3_(420.00_x_297.00_MM)".into();
            l.paper_width = 420.0;
            l.paper_height = 297.0;
            l.plot_paper_units = 1;
            l.plot_rotation = 0;
            l.plot_scale_numerator = 1.0;
            l.plot_scale_denominator = 1.0;
        }
        let scale = 1.0 / 50.0;
        let mut vp = Viewport::with_size(Vector3::new(210.0, 148.5, 0.0), 400.0, 277.0)
            .with_view_target(Vector3::new(0.0, 0.0, 0.0));
        vp.view_center = Vector3::new(5_000.0, 4_000.0, 0.0);
        vp.id = 2;
        vp = vp.with_scale(scale);
        exp.viewport_scale = scale;
        doc.add_entity_to_layout(EntityType::Viewport(Box::new(vp)), "Folha A3").expect("viewport");
    }

    (doc, exp)
}

/// Variantes geradas para os testes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Dwg(DxfVersion),
    DxfAscii(DxfVersion),
    DxfBinary(DxfVersion),
}

impl Variant {
    pub fn file_name(self) -> String {
        match self {
            Variant::Dwg(v) => format!("planta_{v:?}.dwg"),
            Variant::DxfAscii(v) => format!("planta_{v:?}_ascii.dxf"),
            Variant::DxfBinary(v) => format!("planta_{v:?}_bin.dxf"),
        }
    }

    pub fn version(self) -> DxfVersion {
        match self {
            Variant::Dwg(v) | Variant::DxfAscii(v) | Variant::DxfBinary(v) => v,
        }
    }
}

pub const VARIANTS: &[Variant] = &[
    Variant::Dwg(DxfVersion::AC1032),
    Variant::Dwg(DxfVersion::AC1027),
    Variant::Dwg(DxfVersion::AC1024),
    Variant::Dwg(DxfVersion::AC1018),
    Variant::Dwg(DxfVersion::AC1015),
    Variant::DxfAscii(DxfVersion::AC1032),
    Variant::DxfAscii(DxfVersion::AC1018),
    Variant::DxfAscii(DxfVersion::AC1015),
    Variant::DxfBinary(DxfVersion::AC1032),
    // O DxfWriter binário do acadrust promove versões < 2010 para AC1024.
    Variant::DxfBinary(DxfVersion::AC1024),
];
// DXF R12 (AC1009): o DxfWriter do acadrust promove R12 para R13+, então o
// fixture R12 é escrito à mão em `r12.rs`, byte a byte, em Windows-1252.

/// Grava uma variante em `dir`. Retorna o caminho e o esperado.
pub fn write_variant(dir: &Path, variant: Variant) -> Result<(PathBuf, Expected), String> {
    let (doc, exp) = planta(variant.version());
    let path = dir.join(variant.file_name());
    match variant {
        Variant::Dwg(_) => DwgWriter::write_to_file(&path, &doc).map_err(|e| e.to_string())?,
        Variant::DxfAscii(_) => DxfWriter::new(&doc).write_to_file(&path).map_err(|e| e.to_string())?,
        Variant::DxfBinary(_) => DxfWriter::new_binary(&doc).write_to_file(&path).map_err(|e| e.to_string())?,
    }
    Ok((path, exp))
}

/// Planta grande para teste de desempenho: grade de `n` LINE + blocos.
pub fn planta_grande(n_entities: usize) -> CadDocument {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.header.insertion_units = 4;
    layer(&mut doc, "PISO-DIVISÃO", 8, None, |_| {});
    layer(&mut doc, "VEG-ÁRVORES", 3, None, |_| {});
    let vaso = new_block(&mut doc, "VASO");
    add_to_block(&mut doc, vaso, EntityType::Circle(Circle::from_coords(0.0, 0.0, 0.0, 200.0)), "0");
    let side = (n_entities as f64).sqrt().ceil() as usize;
    let mut made = 0;
    'outer: for i in 0..side {
        for j in 0..side {
            if made >= n_entities {
                break 'outer;
            }
            let (x, y) = (i as f64 * 100.0, j as f64 * 100.0);
            let e = match made % 4 {
                0 | 1 => on(EntityType::Line(Line::from_coords(x, y, 0.0, x + 90.0, y, 0.0)), "PISO-DIVISÃO"),
                2 => on(
                    EntityType::LwPolyline(LwPolyline::from_points(vec![
                        Vector2::new(x, y),
                        Vector2::new(x + 50.0, y + 50.0),
                        Vector2::new(x + 90.0, y),
                    ])),
                    "PISO-DIVISÃO",
                ),
                _ => on(EntityType::Insert(Insert::new("VASO", Vector3::new(x, y, 0.0))), "VEG-ÁRVORES"),
            };
            doc.add_entity(e).expect("entidade");
            made += 1;
        }
    }
    doc
}

/// Grava um documento qualquer em DWG (só para fixtures).
pub fn write_doc_dwg(path: &Path, doc: &CadDocument) -> Result<(), String> {
    DwgWriter::write_to_file(path, doc).map_err(|e| e.to_string())
}

/// Padrão visto em arquivo real de detalhamento: desenho em METROS sem
/// $INSUNITS, layout montado em unidades de desenho e plotado em 1000:50
/// (1 unidade do papel = 20 mm). Viewports 1:1 e 1:1,5 no papel/modelo
/// viram 1:50 e 1:75 reais.
pub fn planta_metros_layout_escalado() -> CadDocument {
    let mut doc = CadDocument::with_version(DxfVersion::AC1027);
    doc.header.insertion_units = 0;
    doc.header.measurement = 1;
    layer(&mut doc, "PA-PAGINAÇÃO DE PISO", 1, None, |_| {});
    add(&mut doc, EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 5.0, 0.0, 0.0)), "PA-PAGINAÇÃO DE PISO");
    let lh = doc.add_layout("0002").expect("layout");
    if let Some(ObjectType::Layout(l)) = doc.objects.get_mut(&lh) {
        l.paper_size = "UserDefinedMetric (420.00 x 800.00MM)".into();
        l.paper_width = 420.0;
        l.paper_height = 800.0;
        l.plot_rotation = 1;
        l.plot_paper_units = 1;
        l.plot_scale_numerator = 1000.0;
        l.plot_scale_denominator = 50.0;
    }
    for (id, x, scale) in [(2_i16, 5.0, 1.0), (3, 15.0, 1.0 / 1.5)] {
        let mut vp = Viewport::with_size(Vector3::new(x, 10.0, 0.0), 8.0, 6.0);
        vp.view_center = Vector3::new(2.5, 0.0, 0.0);
        vp.id = id;
        vp = vp.with_scale(scale);
        doc.add_entity_to_layout(EntityType::Viewport(Box::new(vp)), "0002").expect("viewport");
    }
    doc
}
