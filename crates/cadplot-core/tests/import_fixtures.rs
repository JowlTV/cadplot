//! Testes de importação sobre fixtures sintéticos gerados por código.

use std::path::PathBuf;

use cadplot_core::report::{FontKind, ImportReport, ResolveStatus};
use cadplot_core::support::SupportLevel;
use cadplot_core::{import_file, ImportOptions};
use cadplot_fixtures::{write_variant, Expected, Variant, LAYERS, MTEXT_ACENTUADO, TEXT_ACENTUADO, VARIANTS};

fn tmp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("cadplot-test-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn count(r: &ImportReport, t: &str) -> (usize, usize) {
    r.entities
        .by_type
        .iter()
        .find(|c| c.entity_type == t)
        .map(|c| (c.count, c.model_space))
        .unwrap_or((0, 0))
}

fn check_variant(v: Variant) -> Result<(), String> {
    let dir = tmp_dir(&v.file_name());
    let (path, exp) = write_variant(&dir, v)?;
    let before = std::fs::read(&path).unwrap();
    let mtime_before = std::fs::metadata(&path).unwrap().modified().unwrap();

    let imported = import_file(&path, &ImportOptions::default()).map_err(|e| format!("{v:?}: {e}"))?;
    let r = imported.report;

    // Regra de fidelidade: o arquivo de origem não muda.
    assert_eq!(std::fs::read(&path).unwrap(), before, "{v:?}: origem alterada");
    assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), mtime_before);

    check_report(v, &r, &exp);
    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

fn check_report(v: Variant, r: &ImportReport, exp: &Expected) {
    let ctx = format!("{v:?}");
    assert!(!r.read.recovery_mode, "{ctx}: não deveria precisar de recuperação: {:?}", r.read.strict_error);
    let expected_code = cadplot_core::naming::version_code(v.version());
    assert_eq!(r.file.version_code, expected_code, "{ctx}: versão");

    // Acentos: nomes exatos, sem caractere de substituição.
    for name in LAYERS {
        assert!(r.layers.iter().any(|l| l.name == *name), "{ctx}: camada {name} ausente; tem {:?}",
            r.layers.iter().map(|l| &l.name).collect::<Vec<_>>());
    }
    assert_eq!(r.encoding.strings_with_replacement_char, 0, "{ctx}: encoding");
    assert!(r.encoding.non_ascii_samples.iter().any(|s| s == TEXT_ACENTUADO), "{ctx}: texto acentuado: {:?}", r.encoding.non_ascii_samples);
    assert!(r.encoding.non_ascii_samples.iter().any(|s| s == MTEXT_ACENTUADO), "{ctx}: mtext acentuado");

    // Estado das camadas.
    let l = |n: &str| r.layers.iter().find(|l| l.name == n).unwrap();
    assert!(!l("PISO-DIVISÃO").on, "{ctx}: PISO-DIVISÃO deveria estar desligada");
    assert!(l("HID-ÁGUA FRIA").frozen, "{ctx}: HID congelada");
    assert!(l("AUX-CORTE").locked, "{ctx}: AUX travada");
    assert_eq!(l("ARQ-PAREDE").lineweight, "0.35 mm", "{ctx}: espessura");
    assert_eq!(l("ARQ-PAREDE").entity_count, 4, "{ctx}: entidades em ARQ-PAREDE");
    assert!(r.layer_prefix_groups.iter().any(|g| g.prefix == "PA" && g.layers == 1));

    // Contagens no espaço modelo.
    for (t, n) in [
        ("LINE", exp.model_lines),
        ("LWPOLYLINE", exp.model_lwpolylines),
        ("ARC", exp.model_arcs),
        ("CIRCLE", exp.model_circles),
        ("ELLIPSE", exp.model_ellipses),
        ("SPLINE", exp.model_splines),
        ("TEXT", exp.model_texts),
        ("MTEXT", exp.model_mtexts),
        ("HATCH", exp.model_hatches),
        ("INSERT", exp.model_inserts),
        ("DIMENSION", exp.model_dimensions),
    ] {
        assert_eq!(count(r, t).1, n, "{ctx}: {t} no modelo; por tipo = {:#?}", r.entities.by_type);
    }
    assert_eq!(r.entities.unowned, 0, "{ctx}: entidades sem dono");
    assert!(r.entities.by_type.iter().all(|t| t.support != SupportLevel::Unknown), "{ctx}: tipos desconhecidos");

    // Blocos e aninhamento.
    for b in &exp.block_names {
        assert!(r.blocks.blocks.iter().any(|x| x.name == *b), "{ctx}: bloco {b}");
    }
    let canteiro = r.blocks.blocks.iter().find(|b| b.name == "CANTEIRO").unwrap();
    assert_eq!(canteiro.nesting_depth, 2, "{ctx}: CANTEIRO contém VASO");
    assert_eq!(canteiro.insert_count, 2, "{ctx}: inserções de CANTEIRO");
    let vaso = r.blocks.blocks.iter().find(|b| b.name == "VASO-Ø40").unwrap();
    assert_eq!(vaso.insert_count, 3, "{ctx}: 2 no canteiro + 1 no modelo");
    assert!(r.blocks.cycles_detected.is_empty());

    // Fontes: SHX ausente é aviso; TTF não verificada fora do Windows.
    let romans = r.fonts.iter().find(|f| f.name.eq_ignore_ascii_case("romans.shx")).expect("romans");
    assert_eq!(romans.kind, FontKind::Shx);
    assert_eq!(romans.status, ResolveStatus::Missing);
    assert!(r.warnings.iter().any(|w| w.key == "font.missing"), "{ctx}: aviso de fonte");

    // Unidades e escala anotativa.
    assert_eq!(r.drawing.insunits_code, 4, "{ctx}: mm");
    if v.version() >= acadrust::DxfVersion::AC1021 {
        assert_eq!(r.drawing.cannoscale, "1:50", "{ctx}: CANNOSCALE");
    }

    // Layout e viewport 1:50.
    if v.version() != acadrust::DxfVersion::AC1009 {
        let lay = r.layouts.iter().find(|l| l.name == "Folha A3").unwrap_or_else(|| panic!("{ctx}: layout; tem {:?}", r.layouts.iter().map(|l| &l.name).collect::<Vec<_>>()));
        assert!((lay.paper_width_mm - 420.0).abs() < 1e-9 && (lay.paper_height_mm - 297.0).abs() < 1e-9, "{ctx}: papel A3 {}x{}", lay.paper_width_mm, lay.paper_height_mm);
        let vp = lay.viewports.iter().find(|v| !v.is_paper_viewport).unwrap_or_else(|| panic!("{ctx}: viewport de detalhe; {:#?}", lay.viewports));
        let f = vp.scale_factor.unwrap();
        assert!((f - exp.viewport_scale).abs() < 1e-12, "{ctx}: escala {f}");
        assert_eq!(vp.scale_label.as_deref(), Some("1:50"));
    }
}

#[test]
fn todas_as_variantes() {
    let mut failures = Vec::new();
    for v in VARIANTS {
        let res = std::panic::catch_unwind(|| check_variant(*v));
        match res {
            Ok(Ok(())) => eprintln!("ok   {v:?}"),
            Ok(Err(e)) => failures.push(format!("{v:?}: {e}")),
            Err(p) => failures.push(format!(
                "{v:?}: {}",
                p.downcast_ref::<String>().cloned().unwrap_or_else(|| "pânico".into())
            )),
        }
    }
    assert!(failures.is_empty(), "falhas:\n{}", failures.join("\n\n"));
}

#[test]
fn relatorio_texto_e_json() {
    let dir = tmp_dir("texto");
    let (path, _) = write_variant(&dir, Variant::Dwg(acadrust::DxfVersion::AC1032)).unwrap();
    let r = import_file(&path, &ImportOptions::default()).unwrap().report;
    let txt = cadplot_core::text::render(&r, 50);
    assert!(txt.contains("PA-COMENTÁRIO"));
    assert!(txt.contains("AC1032"));
    let json = serde_json::to_string_pretty(&r).unwrap();
    let back: ImportReport = serde_json::from_str(&json).unwrap();
    assert_eq!(back.layers.len(), r.layers.len());
    // JSON em UTF-8 com o acento literal (não escapado como Á por padrão do serde).
    assert!(json.contains("PA-COMENTÁRIO"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn arquivo_invalido_vira_erro_e_nao_panico() {
    let dir = tmp_dir("lixo");
    let p = dir.join("lixo.dwg");
    std::fs::write(&p, b"AC1032\0\0\0\0\0\0isto nao e um dwg").unwrap();
    let res = import_file(&p, &ImportOptions::default());
    assert!(res.is_err() || res.as_ref().unwrap().report.read.recovery_mode);
    let p2 = dir.join("texto.dxf");
    std::fs::write(&p2, b"ola mundo").unwrap();
    assert!(import_file(&p2, &ImportOptions::default()).is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn dxf_r12_windows_1252() {
    use cadplot_fixtures::r12::{R12_LAYERS, R12_TEXT};
    let dir = tmp_dir("r12");
    let path = dir.join("planta_r12.dxf");
    cadplot_fixtures::r12::write(&path).unwrap();
    let raw = std::fs::read(&path).unwrap();
    assert!(raw.windows(2).any(|w| w == b"\xC1R"), "fixture deve estar em cp1252 (Á = 0xC1)");

    let r = import_file(&path, &ImportOptions::default()).unwrap().report;
    eprintln!("{}", cadplot_core::text::render(&r, 30));
    assert_eq!(r.file.version_code, "AC1009");
    assert_eq!(r.encoding.declared_code_page, "ANSI_1252");
    assert_eq!(r.encoding.strings_with_replacement_char, 0);
    for n in R12_LAYERS {
        assert!(r.layers.iter().any(|l| l.name == *n), "camada {n}: {:?}", r.layers.iter().map(|l| &l.name).collect::<Vec<_>>());
    }
    assert!(r.encoding.non_ascii_samples.iter().any(|s| s == R12_TEXT), "{:?}", r.encoding.non_ascii_samples);
    assert!(!r.layers.iter().find(|l| l.name == "PISO-DIVISÃO").unwrap().on, "cor negativa = desligada");
    assert_eq!(count(&r, "LINE").1, 4);
    assert_eq!(count(&r, "CIRCLE").1, 1);
    assert_eq!(count(&r, "ARC").1, 1);
    assert_eq!(count(&r, "TEXT").1, 1);
    assert_eq!(count(&r, "INSERT").1, 1);
    let poly = count(&r, "POLYLINE").1 + count(&r, "POLYLINE_2D").1;
    assert_eq!(poly, 1, "{:#?}", r.entities.by_type);
    std::fs::remove_dir_all(&dir).ok();
}
