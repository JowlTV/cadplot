//! Estruturas do relatório de importação.
//!
//! Chaves JSON estáveis em inglês (contrato com a interface); os textos para o
//! usuário saem traduzidos em `text.rs` / camada de i18n.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::source::SourceKind;
use crate::support::SupportLevel;

pub const REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub schema_version: u32,
    pub cadplot_core_version: String,
    pub acadrust_version: String,
    pub file: FileInfo,
    pub timing: Timing,
    pub read: ReadInfo,
    pub encoding: EncodingInfo,
    pub drawing: DrawingInfo,
    pub entities: EntitySummary,
    pub unknown_objects: BTreeMap<String, usize>,
    pub object_counts: BTreeMap<String, usize>,
    pub layers: Vec<LayerInfo>,
    pub layer_prefix_groups: Vec<PrefixGroup>,
    pub blocks: BlockSummary,
    pub layouts: Vec<LayoutInfo>,
    pub text_styles: Vec<TextStyleInfo>,
    pub fonts: Vec<FontRef>,
    pub xrefs: Vec<ExternalRef>,
    pub images: Vec<ExternalRef>,
    pub diagnostics: Vec<Diagnostic>,
    /// Avisos consolidados para o usuário (o que não será renderizado, o que falta etc.).
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub kind: SourceKind,
    /// Código de versão: AC1032, AC1027…
    pub version_code: String,
    /// Nome humano da família de formato: "AutoCAD 2018–2027 (DWG 2018)".
    pub version_name: String,
    pub maintenance_version: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Timing {
    pub io_ms: f64,
    pub parse_ms: f64,
    pub analyze_ms: f64,
    pub total_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadInfo {
    /// `true` quando a leitura estrita falhou e foi refeita em modo tolerante.
    pub recovery_mode: bool,
    pub strict_error: Option<String>,
    pub stream_completed: bool,
    pub source_records: usize,
    pub decoded_source_records: usize,
    pub skipped_source_records: usize,
    pub recovered_errors: usize,
    pub output_entities: usize,
    pub output_objects: usize,
    pub output_table_records: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodingInfo {
    /// $DWGCODEPAGE declarado (ex.: ANSI_1252).
    pub declared_code_page: String,
    /// Como as strings são de fato armazenadas.
    pub storage: String,
    /// Strings que contêm U+FFFD (caractere de substituição) após a decodificação:
    /// sinal forte de codepage errado.
    pub strings_with_replacement_char: usize,
    /// Amostras de strings não-ASCII (para conferir acentos visualmente).
    pub non_ascii_samples: Vec<String>,
    pub strings_checked: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawingInfo {
    /// $INSUNITS (código) e nome.
    pub insunits_code: i16,
    pub insunits_name: String,
    /// $MEASUREMENT: 0 = imperial, 1 = métrico.
    pub measurement: i16,
    /// $LUNITS (formato linear).
    pub linear_unit_format: i16,
    /// $CANNOSCALE
    pub cannoscale: String,
    /// $CANNOSCALEVALUE (papel/desenho; 1:50 → 0,02).
    pub cannoscale_value: f64,
    pub ltscale: f64,
    pub psltscale: bool,
    pub model_extents: Option<[f64; 4]>,
    pub paper_extents: Option<[f64; 4]>,
    /// Escalas de anotação definidas no desenho (ACAD_SCALELIST).
    pub annotation_scales: Vec<ScaleInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleInfo {
    pub name: String,
    pub paper_units: f64,
    pub drawing_units: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntitySummary {
    pub total: usize,
    pub model_space: usize,
    pub paper_space: usize,
    pub in_block_definitions: usize,
    /// Sem dono conhecido (não pertence a nenhum block record listado).
    pub unowned: usize,
    pub by_type: Vec<EntityTypeCount>,
    /// Entidades UNKNOWN agrupadas pelo nome DXF original.
    pub unknown_by_dxf_name: BTreeMap<String, usize>,
    /// Entidades "estendidas"/proxy agrupadas pela variante.
    pub extended_by_kind: BTreeMap<String, usize>,
    pub invisible_flagged: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeCount {
    pub entity_type: String,
    pub count: usize,
    pub model_space: usize,
    pub paper_space: usize,
    pub in_blocks: usize,
    pub support: SupportLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerInfo {
    pub name: String,
    pub color: String,
    pub linetype: String,
    pub lineweight: String,
    pub on: bool,
    pub frozen: bool,
    pub locked: bool,
    pub plottable: bool,
    pub xref_dependent: bool,
    pub entity_count: usize,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixGroup {
    pub prefix: String,
    pub layers: usize,
    pub entities: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockSummary {
    pub definitions: usize,
    pub anonymous: usize,
    pub xref_blocks: usize,
    pub max_nesting_depth: usize,
    pub cycles_detected: Vec<String>,
    pub blocks: Vec<BlockInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub name: String,
    pub entity_count: usize,
    pub insert_count: usize,
    pub anonymous: bool,
    pub is_xref: bool,
    pub has_attributes: bool,
    pub nesting_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutInfo {
    pub name: String,
    pub tab_order: i16,
    pub is_model: bool,
    pub block_record: String,
    pub entity_count: usize,
    pub paper_size: String,
    /// Largura × altura do papel em mm, já rotacionado conforme `plot_rotation`.
    pub paper_width_mm: f64,
    pub paper_height_mm: f64,
    pub plot_rotation: i16,
    pub plot_paper_units: i16,
    pub plot_scale: String,
    pub plot_style_sheet: String,
    pub viewports: Vec<ViewportInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportInfo {
    pub handle: String,
    pub id: i16,
    /// Viewport id 1: a "janela" do próprio papel, não é uma vista do modelo.
    pub is_paper_viewport: bool,
    pub center: [f64; 2],
    pub width: f64,
    pub height: f64,
    pub view_center: [f64; 2],
    pub view_height: f64,
    pub twist_deg: f64,
    /// Unidades de papel por unidade de desenho (altura / view_height).
    pub scale_factor: Option<f64>,
    /// Escala legível ("1:50"), assumindo papel e desenho na mesma unidade.
    pub scale_label: Option<String>,
    pub annotation_scale: Option<String>,
    pub frozen_layers: usize,
    pub on: bool,
    pub locked: bool,
    pub non_rectangular_clip: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyleInfo {
    pub name: String,
    pub font_file: String,
    pub big_font_file: String,
    pub true_type_family: String,
    pub height: f64,
    pub width_factor: f64,
    pub oblique_deg: f64,
    pub annotative: bool,
    pub is_shape_file: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontKind {
    Shx,
    TrueType,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolveStatus {
    Found,
    Missing,
    /// Fonte de sistema (TrueType) não verificada nesta plataforma.
    NotChecked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontRef {
    pub name: String,
    pub kind: FontKind,
    pub used_by_styles: Vec<String>,
    pub status: ResolveStatus,
    pub resolved_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalRef {
    pub name: String,
    pub stored_path: String,
    pub status: ResolveStatus,
    pub resolved_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub source: String,
    pub severity: String,
    pub code: String,
    pub message: String,
    pub handle: Option<String>,
    pub record_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    pub level: WarningLevel,
    /// Chave estável para i18n (ex.: "entity.not_rendered").
    pub key: String,
    /// Texto em pt-BR já formatado.
    pub message: String,
}
