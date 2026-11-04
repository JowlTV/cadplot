//! Monta o relatório de importação a partir do documento lido.

use std::collections::{BTreeMap, HashMap, HashSet};

use acadrust::objects::ObjectType;
use acadrust::{CadDocument, EntityType, Handle, ReadOutcome};

use crate::import::ImportOptions;
use crate::naming::{
    color_label, entity_type_name, extended_kind_name, insunits_name, lineweight_label,
    object_type_name, scale_label, version_code, version_name,
};
use crate::report::*;
use crate::resolve::{font_kind, resolve_external, resolve_font, SearchPaths};
use crate::source::{SourceFile, SourceKind};
use crate::support::{support_for, SupportLevel};

/// Onde uma entidade mora.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Space {
    Model,
    Paper,
    Block,
    Unowned,
}

pub fn build_report(
    source: &SourceFile,
    outcome: &ReadOutcome,
    strict_error: Option<String>,
    paths: &SearchPaths,
    opts: &ImportOptions,
) -> ImportReport {
    let doc = &outcome.document;
    let stats = &outcome.stats;
    let mut warnings: Vec<Warning> = Vec::new();

    // ---------------------------------------------------------------- arquivo
    let version_code_str = source
        .dwg_version_code
        .clone()
        .unwrap_or_else(|| version_code(doc.version).to_string());
    let file = FileInfo {
        path: source.path.display().to_string(),
        file_name: source
            .path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        size_bytes: source.bytes.len() as u64,
        sha256: source.sha256.clone(),
        kind: source.kind,
        version_name: version_name(&version_code_str).to_string(),
        version_code: version_code_str.clone(),
        maintenance_version: doc.maintenance_version,
    };
    if version_name(&version_code_str) == "versão não reconhecida" {
        warn(
            &mut warnings,
            WarningLevel::Warning,
            "file.version_unknown",
            format!("Versão de arquivo não reconhecida: {version_code_str}"),
        );
    }

    let read = ReadInfo {
        recovery_mode: strict_error.is_some() || stats.recovery_mode,
        strict_error: strict_error.clone(),
        stream_completed: stats.stream_completed,
        source_records: stats.source_records,
        decoded_source_records: stats.decoded_source_records,
        skipped_source_records: stats.skipped_source_records,
        recovered_errors: stats.recovered_errors,
        output_entities: stats.output_entities,
        output_objects: stats.output_objects,
        output_table_records: stats.output_table_records,
    };
    if let Some(e) = &strict_error {
        warn(
            &mut warnings,
            WarningLevel::Error,
            "read.recovery_mode",
            format!("A leitura estrita falhou e o arquivo foi aberto em modo de recuperação. Partes do desenho podem estar faltando. Erro original: {e}"),
        );
    }
    if stats.skipped_source_records > 0 {
        warn(
            &mut warnings,
            WarningLevel::Warning,
            "read.skipped_records",
            format!("{} registros do arquivo foram pulados pelo leitor.", stats.skipped_source_records),
        );
    }

    // ------------------------------------------------------- donos/espaços
    let mut owner_of: HashMap<Handle, (Space, String)> = HashMap::new();
    let mut block_record_by_handle: HashMap<Handle, String> = HashMap::new();
    for br in doc.block_records.iter() {
        block_record_by_handle.insert(br.handle, br.name.clone());
        let space = classify_block(&br.name);
        for h in &br.entity_handles {
            owner_of.insert(*h, (space, br.name.clone()));
        }
    }
    let space_of = |e: &EntityType| -> (Space, Option<String>) {
        let c = e.common();
        if let Some((s, n)) = owner_of.get(&c.handle) {
            return (*s, Some(n.clone()));
        }
        if let Some(n) = block_record_by_handle.get(&c.owner_handle) {
            return (classify_block(n), Some(n.clone()));
        }
        (Space::Unowned, None)
    };

    // ----------------------------------------------------------- entidades
    let mut by_type: BTreeMap<&'static str, [usize; 4]> = BTreeMap::new(); // total, model, paper, blocks
    let mut ents = EntitySummary::default();
    let mut per_layer: HashMap<String, usize> = HashMap::new();
    let mut per_block_count: HashMap<String, usize> = HashMap::new();
    let mut insert_counts: HashMap<String, usize> = HashMap::new();
    let mut block_children: HashMap<String, HashSet<String>> = HashMap::new();
    let mut enc = EncodingScan::new(opts.encoding_samples);

    for e in doc.entities() {
        let name = entity_type_name(e);
        let (space, owner) = space_of(e);
        let slot = by_type.entry(name).or_insert([0; 4]);
        bump(slot, &mut ents, space);
        *per_layer.entry(e.common().layer.clone()).or_default() += 1;
        if let Some(o) = &owner {
            *per_block_count.entry(o.clone()).or_default() += 1;
        }
        if e.common().invisible {
            ents.invisible_flagged += 1;
        }
        match e {
            EntityType::Unknown(u) => {
                *ents.unknown_by_dxf_name.entry(u.dxf_name.clone()).or_default() += 1;
            }
            EntityType::Extended(x) => {
                *ents
                    .extended_by_kind
                    .entry(extended_kind_name(&x.data).to_string())
                    .or_default() += 1;
            }
            EntityType::Insert(ins) => {
                *insert_counts.entry(ins.block_name.clone()).or_default() += 1;
                if let Some(o) = &owner {
                    block_children
                        .entry(o.clone())
                        .or_default()
                        .insert(ins.block_name.clone());
                }
                for a in &ins.attributes {
                    let slot = by_type.entry("ATTRIB").or_insert([0; 4]);
                    bump(slot, &mut ents, space);
                    *per_layer.entry(a.common.layer.clone()).or_default() += 1;
                    enc.check(&a.value);
                }
            }
            EntityType::Dimension(d) => {
                let b = &d.base().block_name;
                if !b.is_empty() {
                    *insert_counts.entry(b.clone()).or_default() += 1;
                }
                enc.check(&d.base().text);
            }
            EntityType::Text(t) => enc.check(&t.value),
            EntityType::MText(t) => enc.check(&t.value),
            EntityType::AttributeEntity(a) => enc.check(&a.value),
            EntityType::AttributeDefinition(a) => {
                enc.check(&a.default_value);
                enc.check(&a.prompt);
            }
            _ => {}
        }
    }
    ents.by_type = by_type
        .into_iter()
        .map(|(t, [n, m, p, b])| EntityTypeCount {
            entity_type: t.to_string(),
            count: n,
            model_space: m,
            paper_space: p,
            in_blocks: b,
            support: support_for(t),
        })
        .collect();
    ents.by_type.sort_by(|a, b| b.count.cmp(&a.count).then(a.entity_type.cmp(&b.entity_type)));

    for t in &ents.by_type {
        match t.support {
            SupportLevel::OutOfScope | SupportLevel::Unknown => warn(
                &mut warnings,
                WarningLevel::Warning,
                "entity.not_rendered",
                format!(
                    "{} entidade(s) {} não serão renderizadas ({}).",
                    t.count,
                    t.entity_type,
                    t.support.label_pt()
                ),
            ),
            SupportLevel::Partial => warn(
                &mut warnings,
                WarningLevel::Info,
                "entity.partial",
                format!(
                    "{} entidade(s) {} terão renderização parcial (limitações em docs/reports).",
                    t.count, t.entity_type
                ),
            ),
            _ => {}
        }
    }
    for (n, c) in &ents.unknown_by_dxf_name {
        warn(
            &mut warnings,
            WarningLevel::Warning,
            "entity.unknown_type",
            format!("{c} entidade(s) de tipo desconhecido \"{n}\" (preservadas, não renderizadas)."),
        );
    }
    for (n, c) in &ents.extended_by_kind {
        warn(
            &mut warnings,
            WarningLevel::Warning,
            "entity.extended",
            format!("{c} entidade(s) especiais/proxy \"{n}\" não serão renderizadas."),
        );
    }

    // -------------------------------------------------------------- objetos
    let mut object_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut unknown_objects: BTreeMap<String, usize> = BTreeMap::new();
    let mut annotation_scales = Vec::new();
    let mut images = Vec::new();
    for o in doc.objects.values() {
        let n = object_type_name(o);
        if let Some(rest) = n.strip_prefix("UNKNOWN:") {
            *unknown_objects.entry(rest.to_string()).or_default() += 1;
        } else if n == "ACAD_PROXY_OBJECT" {
            *unknown_objects.entry(n.clone()).or_default() += 1;
        }
        *object_counts.entry(n).or_default() += 1;
        match o {
            ObjectType::Scale(s) => annotation_scales.push(ScaleInfo {
                name: s.name.clone(),
                paper_units: s.paper_units,
                drawing_units: s.drawing_units,
            }),
            ObjectType::ImageDefinition(img) => {
                let (status, resolved) = resolve_external(&img.file_name, paths);
                images.push(ExternalRef {
                    name: file_tail(&img.file_name),
                    stored_path: img.file_name.clone(),
                    status,
                    resolved_path: resolved.map(|p| p.display().to_string()),
                });
            }
            _ => {}
        }
    }
    annotation_scales.sort_by(|a, b| a.name.cmp(&b.name));
    images.sort_by(|a, b| a.stored_path.cmp(&b.stored_path));
    if !unknown_objects.is_empty() {
        let total: usize = unknown_objects.values().sum();
        warn(
            &mut warnings,
            WarningLevel::Info,
            "object.unknown",
            format!("{total} objeto(s) não-gráfico(s) desconhecido(s) foram preservados sem interpretação (não afetam o desenho visível, em geral)."),
        );
    }
    for img in images.iter().filter(|i| i.status == ResolveStatus::Missing) {
        warn(
            &mut warnings,
            WarningLevel::Warning,
            "image.missing",
            format!("Imagem referenciada não encontrada: {}", img.stored_path),
        );
    }

    // -------------------------------------------------------------- camadas
    let mut layers: Vec<LayerInfo> = doc
        .layers
        .iter()
        .map(|l| {
            enc.check(&l.name);
            LayerInfo {
                name: l.name.clone(),
                color: color_label(&l.color),
                linetype: l.line_type.clone(),
                lineweight: lineweight_label(&l.line_weight),
                on: !l.flags.off,
                frozen: l.flags.frozen,
                locked: l.flags.locked,
                plottable: l.is_plottable,
                xref_dependent: l.flags.xref_dependent,
                entity_count: per_layer.get(&l.name).copied().unwrap_or(0),
                description: l.description.clone(),
            }
        })
        .collect();
    layers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let known: HashSet<String> = layers.iter().map(|l| l.name.to_lowercase()).collect();
    let mut orphan_layers: Vec<_> = per_layer
        .keys()
        .filter(|n| !known.contains(&n.to_lowercase()))
        .cloned()
        .collect();
    orphan_layers.sort();
    for n in orphan_layers {
        warn(
            &mut warnings,
            WarningLevel::Warning,
            "layer.missing_record",
            format!("Entidades usam a camada \"{n}\", que não existe na tabela de camadas."),
        );
    }
    let layer_prefix_groups = prefix_groups(&layers);

    // --------------------------------------------------------------- blocos
    let blocks = block_summary(doc, &per_block_count, &insert_counts, &block_children, &mut enc);
    for c in &blocks.cycles_detected {
        warn(
            &mut warnings,
            WarningLevel::Error,
            "block.cycle",
            format!("Referência circular de blocos detectada envolvendo \"{c}\"."),
        );
    }

    // ----------------------------------------------------------------- xrefs
    let mut xrefs = Vec::new();
    for br in doc.block_records.iter() {
        if br.flags.is_xref || br.flags.is_xref_overlay {
            let (status, resolved) = resolve_external(&br.xref_path, paths);
            xrefs.push(ExternalRef {
                name: br.name.clone(),
                stored_path: br.xref_path.clone(),
                status,
                resolved_path: resolved.map(|p| p.display().to_string()),
            });
        }
    }
    for x in xrefs.iter().filter(|x| x.status == ResolveStatus::Missing) {
        warn(
            &mut warnings,
            WarningLevel::Warning,
            "xref.missing",
            format!("Xref \"{}\" não encontrada em: {}", x.name, x.stored_path),
        );
    }

    // -------------------------------------------------------- estilos/fontes
    let mut text_styles = Vec::new();
    let mut fonts: BTreeMap<String, FontRef> = BTreeMap::new();
    for s in doc.text_styles.iter() {
        enc.check(&s.name);
        text_styles.push(TextStyleInfo {
            name: s.name.clone(),
            font_file: s.font_file.clone(),
            big_font_file: s.big_font_file.clone(),
            true_type_family: s.true_type_font.clone(),
            height: s.height,
            width_factor: s.width_factor,
            oblique_deg: s.oblique_angle.to_degrees(),
            annotative: s.annotative,
            is_shape_file: s.is_shape_file,
        });
        for f in [&s.font_file, &s.big_font_file] {
            let f = f.trim();
            if f.is_empty() {
                continue;
            }
            let key = f.to_lowercase();
            let entry = fonts.entry(key).or_insert_with(|| {
                let (status, resolved) = resolve_font(f, paths);
                FontRef {
                    name: f.to_string(),
                    kind: font_kind(f),
                    used_by_styles: vec![],
                    status,
                    resolved_path: resolved.map(|p| p.display().to_string()),
                }
            });
            entry.used_by_styles.push(s.name.clone());
        }
    }
    text_styles.sort_by(|a, b| a.name.cmp(&b.name));
    let fonts: Vec<FontRef> = fonts.into_values().collect();
    for f in fonts.iter().filter(|f| f.status == ResolveStatus::Missing) {
        warn(
            &mut warnings,
            WarningLevel::Warning,
            "font.missing",
            format!(
                "Fonte {} \"{}\" não encontrada (estilos: {}). Textos com ela serão desenhados com fonte substituta.",
                match f.kind {
                    FontKind::Shx => "SHX",
                    FontKind::TrueType => "TrueType",
                    FontKind::Other => "",
                },
                f.name,
                f.used_by_styles.join(", ")
            ),
        );
    }

    // -------------------------------------------------------------- layouts
    let layouts = layouts(doc, &per_block_count, &block_record_by_handle);

    // ------------------------------------------------------------ desenho
    let h = &doc.header;
    let cannoscale = resolve_cannoscale(doc);
    let drawing = DrawingInfo {
        insunits_code: h.insertion_units,
        insunits_name: insunits_name(h.insertion_units).to_string(),
        measurement: h.measurement,
        linear_unit_format: h.linear_unit_format,
        cannoscale: cannoscale.0,
        cannoscale_value: cannoscale.1,
        cannoscale_source: cannoscale.2.into(),
        ltscale: h.linetype_scale,
        psltscale: h.paper_space_linetype_scaling,
        model_extents: extents(h.model_space_extents_min, h.model_space_extents_max),
        paper_extents: extents(h.paper_space_extents_min, h.paper_space_extents_max),
        annotation_scales,
    };
    if h.insertion_units == 0 {
        warn(
            &mut warnings,
            WarningLevel::Info,
            "units.unitless",
            "O desenho não declara unidade ($INSUNITS = 0). Defina mm ou m antes de plotar.".into(),
        );
    }

    // ----------------------------------------------------------- encoding
    let encoding = EncodingInfo {
        declared_code_page: h.code_page.clone(),
        storage: storage_label(source.kind, doc).into(),
        strings_with_replacement_char: enc.bad,
        non_ascii_samples: enc.samples,
        strings_checked: enc.checked,
    };
    if encoding.strings_with_replacement_char > 0 {
        warn(
            &mut warnings,
            WarningLevel::Error,
            "encoding.replacement_chars",
            format!(
                "{} texto(s) contêm caracteres inválidos (�): provável codepage incorreto (declarado: {}).",
                encoding.strings_with_replacement_char, encoding.declared_code_page
            ),
        );
    }

    // ------------------------------------------------------- diagnósticos
    let mut diagnostics: Vec<Diagnostic> = stats
        .diagnostics
        .iter()
        .map(|d| Diagnostic {
            source: "reader".into(),
            severity: "warning".into(),
            code: d.code.clone(),
            message: d.message.clone(),
            handle: d.record_handle.map(|h| format!("{h:X}")),
            record_type: d.record_type.clone(),
        })
        .collect();
    for n in doc.notifications.iter() {
        diagnostics.push(Diagnostic {
            source: "notification".into(),
            severity: format!("{:?}", n.notification_type).to_lowercase(),
            code: format!("{:?}", n.notification_type),
            message: n.message.clone(),
            handle: None,
            record_type: None,
        });
    }
    if !diagnostics.is_empty() {
        warn(
            &mut warnings,
            WarningLevel::Info,
            "read.diagnostics",
            format!("O leitor registrou {} diagnóstico(s); veja a seção de diagnósticos.", diagnostics.len()),
        );
    }

    warnings.sort_by(|a, b| b.level.cmp(&a.level));

    ImportReport {
        schema_version: REPORT_SCHEMA_VERSION,
        cadplot_core_version: env!("CARGO_PKG_VERSION").into(),
        acadrust_version: acadrust::VERSION.into(),
        file,
        timing: Timing::default(),
        read,
        encoding,
        drawing,
        entities: ents,
        unknown_objects,
        object_counts,
        layers,
        layer_prefix_groups,
        blocks,
        layouts,
        text_styles,
        fonts,
        xrefs,
        images,
        diagnostics,
        warnings,
    }
}

/// CANNOSCALE: no DWG e no DXF o AutoCAD guarda o valor no
/// AcDbVariableDictionary. O leitor DWG do acadrust já reflete isso no
/// cabeçalho; o leitor DXF não (0.6.8), então resolvemos aqui.
fn resolve_cannoscale(doc: &CadDocument) -> (String, f64, &'static str) {
    let h = &doc.header;
    let (name, source) = match doc.drawing_variable("CANNOSCALE") {
        Some(v) if !v.trim().is_empty() => (v.to_string(), "AcDbVariableDictionary"),
        _ => (h.current_annotation_scale.clone(), "cabeçalho"),
    };
    let value = doc
        .objects
        .values()
        .find_map(|o| match o {
            ObjectType::Scale(s) if s.name.eq_ignore_ascii_case(&name) && s.drawing_units != 0.0 => {
                Some(s.paper_units / s.drawing_units)
            }
            _ => None,
        })
        .unwrap_or(if name == h.current_annotation_scale { h.annotation_scale_value } else { f64::NAN });
    (name, value, source)
}

fn warn(list: &mut Vec<Warning>, level: WarningLevel, key: &str, message: String) {
    list.push(Warning {
        level,
        key: key.into(),
        message,
    });
}

fn bump(slot: &mut [usize; 4], ents: &mut EntitySummary, space: Space) {
    slot[0] += 1;
    ents.total += 1;
    match space {
        Space::Model => {
            slot[1] += 1;
            ents.model_space += 1;
        }
        Space::Paper => {
            slot[2] += 1;
            ents.paper_space += 1;
        }
        Space::Block => {
            slot[3] += 1;
            ents.in_block_definitions += 1;
        }
        Space::Unowned => ents.unowned += 1,
    }
}

fn classify_block(name: &str) -> Space {
    let lower = name.to_ascii_lowercase();
    if lower == "*model_space" {
        Space::Model
    } else if lower.starts_with("*paper_space") {
        Space::Paper
    } else {
        Space::Block
    }
}

fn extents(min: acadrust::Vector3, max: acadrust::Vector3) -> Option<[f64; 4]> {
    let ok = [min.x, min.y, max.x, max.y].iter().all(|v| v.is_finite() && v.abs() < 1e19);
    if ok && max.x >= min.x && max.y >= min.y {
        Some([min.x, min.y, max.x, max.y])
    } else {
        None
    }
}

fn file_tail(p: &str) -> String {
    p.rsplit(['\\', '/']).next().unwrap_or(p).to_string()
}

fn storage_label(kind: SourceKind, doc: &CadDocument) -> &'static str {
    use acadrust::DxfVersion::*;
    let unicode = matches!(doc.version, AC1021 | AC1024 | AC1027 | AC1032);
    match (kind, unicode) {
        (SourceKind::Dwg, true) => "Unicode (UTF-16, nativo do DWG 2007+)",
        (SourceKind::Dwg, false) => "codepage do arquivo ($DWGCODEPAGE)",
        (_, true) => "UTF-8 (DXF 2007+)",
        (_, false) => "codepage do arquivo ($DWGCODEPAGE)",
    }
}

/// Agrupamento por prefixo: tudo antes do primeiro '-' ou '_' (ex.: "PA-COMENTÁRIO" → "PA").
pub fn layer_prefix(name: &str) -> Option<&str> {
    let idx = name.find(['-', '_'])?;
    if idx == 0 {
        return None;
    }
    Some(&name[..idx])
}

fn prefix_groups(layers: &[LayerInfo]) -> Vec<PrefixGroup> {
    let mut map: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for l in layers {
        let key = layer_prefix(&l.name).map_or_else(|| "(sem prefixo)".to_string(), str::to_uppercase);
        let e = map.entry(key).or_default();
        e.0 += 1;
        e.1 += l.entity_count;
    }
    map.into_iter()
        .map(|(prefix, (layers, entities))| PrefixGroup {
            prefix,
            layers,
            entities,
        })
        .collect()
}

fn block_summary(
    doc: &CadDocument,
    per_block_count: &HashMap<String, usize>,
    insert_counts: &HashMap<String, usize>,
    children: &HashMap<String, HashSet<String>>,
    enc: &mut EncodingScan,
) -> BlockSummary {
    let mut s = BlockSummary::default();
    let mut depth_cache: HashMap<String, usize> = HashMap::new();
    let mut cycles: HashSet<String> = HashSet::new();
    for br in doc.block_records.iter() {
        if classify_block(&br.name) != Space::Block {
            continue;
        }
        enc.check(&br.name);
        let depth = nesting_depth(&br.name, children, &mut depth_cache, &mut Vec::new(), &mut cycles);
        s.definitions += 1;
        if br.flags.anonymous || br.name.starts_with('*') {
            s.anonymous += 1;
        }
        if br.flags.is_xref || br.flags.is_xref_overlay {
            s.xref_blocks += 1;
        }
        s.max_nesting_depth = s.max_nesting_depth.max(depth);
        s.blocks.push(BlockInfo {
            name: br.name.clone(),
            entity_count: per_block_count.get(&br.name).copied().unwrap_or(br.entity_handles.len()),
            insert_count: insert_counts.get(&br.name).copied().unwrap_or(0),
            anonymous: br.flags.anonymous || br.name.starts_with('*'),
            is_xref: br.flags.is_xref || br.flags.is_xref_overlay,
            has_attributes: br.flags.has_attributes,
            nesting_depth: depth,
        });
    }
    s.blocks.sort_by(|a, b| a.anonymous.cmp(&b.anonymous).then(a.name.cmp(&b.name)));
    let mut c: Vec<_> = cycles.into_iter().collect();
    c.sort();
    s.cycles_detected = c;
    s
}

/// Profundidade de aninhamento: 1 = bloco sem INSERT dentro.
fn nesting_depth(
    name: &str,
    children: &HashMap<String, HashSet<String>>,
    cache: &mut HashMap<String, usize>,
    stack: &mut Vec<String>,
    cycles: &mut HashSet<String>,
) -> usize {
    if let Some(d) = cache.get(name) {
        return *d;
    }
    if stack.iter().any(|s| s.eq_ignore_ascii_case(name)) {
        cycles.insert(name.to_string());
        return 0;
    }
    stack.push(name.to_string());
    let mut best = 0;
    if let Some(kids) = children.get(name) {
        for k in kids {
            best = best.max(nesting_depth(k, children, cache, stack, cycles));
        }
    }
    stack.pop();
    let d = best + 1;
    cache.insert(name.to_string(), d);
    d
}

fn layouts(
    doc: &CadDocument,
    per_block_count: &HashMap<String, usize>,
    block_record_by_handle: &HashMap<Handle, String>,
) -> Vec<LayoutInfo> {
    // Viewports por block record de layout.
    let mut vps_by_block: HashMap<String, Vec<ViewportInfo>> = HashMap::new();
    for e in doc.entities() {
        let EntityType::Viewport(vp) = e else { continue };
        let owner = block_record_by_handle
            .get(&vp.common.owner_handle)
            .cloned()
            .or_else(|| {
                doc.block_records
                    .iter()
                    .find(|b| b.entity_handles.contains(&vp.common.handle))
                    .map(|b| b.name.clone())
            })
            .unwrap_or_default();
        let factor = if vp.view_height > 0.0 && vp.height > 0.0 {
            Some(vp.height / vp.view_height)
        } else {
            None
        };
        let annotation_scale = doc
            .viewport_annotation_scale(vp.common.handle)
            .and_then(|h| match doc.objects.get(&h) {
                Some(ObjectType::Scale(s)) => Some(s.name.clone()),
                _ => None,
            });
        vps_by_block.entry(owner.to_lowercase()).or_default().push(ViewportInfo {
            handle: format!("{:X}", vp.common.handle.value()),
            id: vp.id,
            is_paper_viewport: vp.id == 1,
            center: [vp.center.x, vp.center.y],
            width: vp.width,
            height: vp.height,
            view_center: [vp.view_center.x, vp.view_center.y],
            view_height: vp.view_height,
            twist_deg: vp.twist_angle.to_degrees(),
            scale_factor: factor,
            scale_label: factor.and_then(scale_label),
            annotation_scale,
            frozen_layers: vp.frozen_layers.len(),
            on: !vp.status.is_off,
            locked: vp.status.locked,
            non_rectangular_clip: vp.status.non_rectangular_clipping,
        });
    }

    let mut out = Vec::new();
    for o in doc.objects.values() {
        let ObjectType::Layout(l) = o else { continue };
        let block = block_record_by_handle.get(&l.block_record).cloned().unwrap_or_default();
        let is_model = l.name.eq_ignore_ascii_case("Model") || block.eq_ignore_ascii_case("*Model_Space");
        let mut viewports = if is_model {
            vec![]
        } else {
            vps_by_block.remove(&block.to_lowercase()).unwrap_or_default()
        };
        viewports.sort_by_key(|v| v.id);
        // plot_paper_units: 0 = polegadas, 1 = mm, 2 = pixels. O papel em LAYOUT vem em mm.
        let (mut w, mut hgt) = (l.paper_width, l.paper_height);
        if l.plot_rotation == 1 || l.plot_rotation == 3 {
            std::mem::swap(&mut w, &mut hgt);
        }
        let plot_scale = if l.plot_scale_denominator > 0.0 && l.plot_scale_numerator > 0.0 {
            format!("{}:{}", trim_num(l.plot_scale_numerator), trim_num(l.plot_scale_denominator))
        } else {
            "—".into()
        };
        out.push(LayoutInfo {
            name: l.name.clone(),
            tab_order: l.tab_order,
            is_model,
            entity_count: per_block_count.get(&block).copied().unwrap_or(0),
            block_record: block,
            paper_size: l.paper_size.clone(),
            paper_width_mm: w,
            paper_height_mm: hgt,
            plot_rotation: l.plot_rotation,
            plot_paper_units: l.plot_paper_units,
            plot_scale,
            plot_style_sheet: l.plot_style_sheet.clone(),
            viewports,
        });
    }
    out.sort_by(|a, b| b.is_model.cmp(&a.is_model).then(a.tab_order.cmp(&b.tab_order)));
    out
}

fn trim_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v}")
    }
}

struct EncodingScan {
    checked: usize,
    bad: usize,
    samples: Vec<String>,
    max_samples: usize,
}

impl EncodingScan {
    fn new(max_samples: usize) -> Self {
        Self {
            checked: 0,
            bad: 0,
            samples: Vec::new(),
            max_samples,
        }
    }

    fn check(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.checked += 1;
        if s.contains('\u{FFFD}') {
            self.bad += 1;
        }
        if !s.is_ascii() && self.samples.len() < self.max_samples {
            let sample: String = s.chars().take(80).collect();
            if !self.samples.contains(&sample) {
                self.samples.push(sample);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixos() {
        assert_eq!(layer_prefix("PA-COMENTÁRIO"), Some("PA"));
        assert_eq!(layer_prefix("ARQ_PAREDE"), Some("ARQ"));
        assert_eq!(layer_prefix("0"), None);
        assert_eq!(layer_prefix("-X"), None);
    }
}
