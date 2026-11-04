//! Relatório de importação em texto legível (pt-BR).

use std::fmt::Write;

use crate::report::*;

fn yes(b: bool) -> &'static str {
    if b {
        "sim"
    } else {
        "não"
    }
}

fn level(l: WarningLevel) -> &'static str {
    match l {
        WarningLevel::Error => "ERRO ",
        WarningLevel::Warning => "AVISO",
        WarningLevel::Info => "info ",
    }
}

fn status(s: ResolveStatus) -> &'static str {
    match s {
        ResolveStatus::Found => "encontrada",
        ResolveStatus::Missing => "FALTANDO",
        ResolveStatus::NotChecked => "não verificada (fonte do sistema)",
    }
}

/// Formata o relatório. `max_rows` limita listas longas (camadas, blocos); 0 = sem limite.
pub fn render(r: &ImportReport, max_rows: usize) -> String {
    let lim = |n: usize| if max_rows == 0 { n } else { n.min(max_rows) };
    let mut o = String::new();
    let f = &r.file;
    let _ = writeln!(o, "RELATÓRIO DE IMPORTAÇÃO — CadPlot (núcleo {}, acadrust {})", r.cadplot_core_version, r.acadrust_version);
    let _ = writeln!(o, "{}", "=".repeat(78));
    let _ = writeln!(o, "Arquivo:      {}", f.path);
    let _ = writeln!(o, "Tamanho:      {:.2} MB ({} bytes)", f.size_bytes as f64 / 1_048_576.0, f.size_bytes);
    let _ = writeln!(o, "SHA-256:      {}", f.sha256);
    let _ = writeln!(o, "Formato:      {}", f.kind.label());
    let _ = writeln!(o, "Versão:       {} — {} (manutenção {})", f.version_code, f.version_name, f.maintenance_version);
    let t = &r.timing;
    let _ = writeln!(
        o,
        "Tempo:        {:.0} ms total (E/S {:.0} ms, leitura {:.0} ms, análise {:.0} ms)",
        t.total_ms, t.io_ms, t.parse_ms, t.analyze_ms
    );
    let rd = &r.read;
    let _ = writeln!(
        o,
        "Leitura:      {}; registros {} lidos / {} decodificados / {} pulados; erros recuperados {}",
        if rd.recovery_mode { "MODO DE RECUPERAÇÃO" } else { "estrita, ok" },
        rd.source_records,
        rd.decoded_source_records,
        rd.skipped_source_records,
        rd.recovered_errors
    );

    let _ = writeln!(o, "\nAVISOS ({})", r.warnings.len());
    if r.warnings.is_empty() {
        let _ = writeln!(o, "  nenhum");
    }
    for w in &r.warnings {
        let _ = writeln!(o, "  [{}] {}", level(w.level), w.message);
    }

    let e = &r.encoding;
    let _ = writeln!(o, "\nCODIFICAÇÃO");
    let _ = writeln!(o, "  $DWGCODEPAGE: {}   armazenamento: {}", e.declared_code_page, e.storage);
    let _ = writeln!(o, "  textos verificados: {}   com caractere inválido (�): {}", e.strings_checked, e.strings_with_replacement_char);
    if !e.non_ascii_samples.is_empty() {
        let _ = writeln!(o, "  amostras com acento: {}", e.non_ascii_samples.join(" | "));
    }

    let d = &r.drawing;
    let _ = writeln!(o, "\nDESENHO");
    let _ = writeln!(o, "  $INSUNITS: {} ({})   $MEASUREMENT: {}", d.insunits_code, d.insunits_name, if d.measurement == 1 { "métrico" } else { "imperial" });
    let _ = writeln!(o, "  $CANNOSCALE: {} (valor {})   $LTSCALE: {}   $PSLTSCALE: {}", d.cannoscale, d.cannoscale_value, d.ltscale, yes(d.psltscale));
    if let Some(x) = d.model_extents {
        let _ = writeln!(o, "  extensão (modelo): ({:.3}, {:.3}) – ({:.3}, {:.3})", x[0], x[1], x[2], x[3]);
    }
    if !d.annotation_scales.is_empty() {
        let names: Vec<_> = d.annotation_scales.iter().map(|s| s.name.as_str()).collect();
        let _ = writeln!(o, "  escalas de anotação: {}", names.join(", "));
    }

    let en = &r.entities;
    let _ = writeln!(o, "\nENTIDADES: {} (modelo {}, papel {}, em definições de bloco {}, sem dono {})", en.total, en.model_space, en.paper_space, en.in_block_definitions, en.unowned);
    let _ = writeln!(o, "  {:<16} {:>9} {:>9} {:>9} {:>9}  suporte", "tipo", "total", "modelo", "papel", "blocos");
    for t in &en.by_type {
        let _ = writeln!(o, "  {:<16} {:>9} {:>9} {:>9} {:>9}  {}", t.entity_type, t.count, t.model_space, t.paper_space, t.in_blocks, t.support.label_pt());
    }
    for (k, v) in &en.unknown_by_dxf_name {
        let _ = writeln!(o, "  desconhecida: {k} × {v}");
    }
    for (k, v) in &en.extended_by_kind {
        let _ = writeln!(o, "  especial/proxy: {k} × {v}");
    }

    let _ = writeln!(o, "\nOBJETOS DESCONHECIDOS: {}", if r.unknown_objects.is_empty() { "nenhum".to_string() } else { String::new() });
    for (k, v) in &r.unknown_objects {
        let _ = writeln!(o, "  {k} × {v}");
    }

    let _ = writeln!(o, "\nCAMADAS: {}", r.layers.len());
    let _ = writeln!(o, "  {:<34} {:<12} {:<12} {:<10} lig cong bloq  entid.", "nome", "cor", "linetype", "espessura");
    for l in r.layers.iter().take(lim(r.layers.len())) {
        let _ = writeln!(
            o,
            "  {:<34} {:<12} {:<12} {:<10} {:<3} {:<4} {:<4} {:>7}",
            l.name, l.color, l.linetype, l.lineweight, yes(l.on), yes(l.frozen), yes(l.locked), l.entity_count
        );
    }
    if r.layers.len() > lim(r.layers.len()) {
        let _ = writeln!(o, "  … mais {} (veja o JSON)", r.layers.len() - lim(r.layers.len()));
    }
    let _ = writeln!(o, "  grupos por prefixo:");
    for g in &r.layer_prefix_groups {
        let _ = writeln!(o, "    {:<16} {:>4} camadas {:>8} entidades", g.prefix, g.layers, g.entities);
    }

    let b = &r.blocks;
    let _ = writeln!(o, "\nBLOCOS: {} definições ({} anônimos, {} xref), aninhamento máximo {}", b.definitions, b.anonymous, b.xref_blocks, b.max_nesting_depth);
    let named: Vec<_> = b.blocks.iter().filter(|x| !x.anonymous).collect();
    for x in named.iter().take(lim(named.len())) {
        let _ = writeln!(o, "  {:<40} {:>6} entid. {:>6} inserções  nível {}", x.name, x.entity_count, x.insert_count, x.nesting_depth);
    }
    if named.len() > lim(named.len()) {
        let _ = writeln!(o, "  … mais {} (veja o JSON)", named.len() - lim(named.len()));
    }

    let _ = writeln!(o, "\nLAYOUTS: {}", r.layouts.len());
    for l in &r.layouts {
        if l.is_model {
            let _ = writeln!(o, "  [Modelo] {} entidades", l.entity_count);
            continue;
        }
        let _ = writeln!(
            o,
            "  [{}] papel \"{}\" {:.1} × {:.1} mm, rotação {}, escala de plot {}, {} entidades",
            l.name, l.paper_size, l.paper_width_mm, l.paper_height_mm, l.plot_rotation * 90, l.plot_scale, l.entity_count
        );
        for v in &l.viewports {
            if v.is_paper_viewport {
                continue;
            }
            let _ = writeln!(
                o,
                "      viewport {} (id {}): {:.1} × {:.1} no papel, escala {} , anotativa {}, camadas congeladas {}{}{}",
                v.handle,
                v.id,
                v.width,
                v.height,
                v.scale_label.as_deref().unwrap_or("?"),
                v.annotation_scale.as_deref().unwrap_or("—"),
                v.frozen_layers,
                if v.locked { ", travada" } else { "" },
                if v.non_rectangular_clip { ", recorte não-retangular" } else { "" },
            );
        }
    }

    let _ = writeln!(o, "\nESTILOS DE TEXTO: {}", r.text_styles.len());
    for s in &r.text_styles {
        let _ = writeln!(
            o,
            "  {:<24} fonte \"{}\"{}{}{}",
            s.name,
            s.font_file,
            if s.big_font_file.is_empty() { String::new() } else { format!(" bigfont \"{}\"", s.big_font_file) },
            if s.true_type_family.is_empty() { String::new() } else { format!(" TTF \"{}\"", s.true_type_family) },
            if s.annotative { " (anotativo)" } else { "" }
        );
    }
    let _ = writeln!(o, "\nFONTES: {}", r.fonts.len());
    for f in &r.fonts {
        let _ = writeln!(o, "  {:<24} {:?}  {}", f.name, f.kind, status(f.status));
    }

    let _ = writeln!(o, "\nXREFS: {}", if r.xrefs.is_empty() { "nenhuma".into() } else { r.xrefs.len().to_string() });
    for x in &r.xrefs {
        let _ = writeln!(o, "  {} → {}  [{}]", x.name, x.stored_path, status(x.status));
    }
    if !r.images.is_empty() {
        let _ = writeln!(o, "\nIMAGENS: {}", r.images.len());
        for x in &r.images {
            let _ = writeln!(o, "  {}  [{}]", x.stored_path, status(x.status));
        }
    }

    let _ = writeln!(o, "\nDIAGNÓSTICOS DO LEITOR: {}", r.diagnostics.len());
    for dg in r.diagnostics.iter().take(lim(r.diagnostics.len())) {
        let _ = writeln!(
            o,
            "  [{}] {}: {}{}",
            dg.severity,
            dg.code,
            dg.message,
            dg.handle.as_ref().map(|h| format!(" (handle {h})")).unwrap_or_default()
        );
    }
    if r.diagnostics.len() > lim(r.diagnostics.len()) {
        let _ = writeln!(o, "  … mais {} (veja o JSON)", r.diagnostics.len() - lim(r.diagnostics.len()));
    }
    o
}
