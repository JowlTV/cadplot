//! cadplot-inspect: relatório de importação de DWG/DXF pela linha de comando.
//!
//! Uso:
//!   cadplot-inspect <arquivo.dwg|.dxf> [mais arquivos…] [opções]
//!
//! Opções:
//!   --json <arquivo>      grava o relatório completo em JSON (UTF-8)
//!   --json-ao-lado        grava <desenho>.report.json ao lado de cada desenho
//!   --txt-ao-lado         grava <desenho>.report.txt ao lado de cada desenho
//!   --fontes <pasta>      pasta extra para procurar SHX/xrefs/imagens (pode repetir)
//!   --codepage <nome>     codepage padrão para DXF antigo sem $DWGCODEPAGE (ex.: ANSI_1252)
//!   --sem-recuperacao     não tenta o modo de recuperação se a leitura estrita falhar
//!   --linhas <n>          máximo de linhas por lista no texto (padrão 60, 0 = tudo)
//!   --cena <arquivo.cps>  grava a cena do espaço modelo (formato binário do visualizador)
//!
//! O desenho de origem é aberto somente para leitura. As saídas são arquivos
//! novos; o programa se recusa a gravar por cima do próprio desenho.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cadplot_core::{import_file, ImportOptions};

struct Args {
    inputs: Vec<PathBuf>,
    json: Option<PathBuf>,
    json_beside: bool,
    txt_beside: bool,
    opts: ImportOptions,
    max_rows: usize,
    scene: Option<PathBuf>,
}

fn usage() -> &'static str {
    "uso: cadplot-inspect <arquivo.dwg|.dxf> [mais arquivos…] [--json <saida.json>] [--json-ao-lado] [--txt-ao-lado] [--fontes <pasta>] [--codepage ANSI_1252] [--sem-recuperacao] [--linhas N]"
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        inputs: vec![],
        json: None,
        json_beside: false,
        txt_beside: false,
        opts: ImportOptions::default(),
        max_rows: 60,
        scene: None,
    };
    let mut it = std::env::args_os().skip(1);
    while let Some(arg) = it.next() {
        let s = arg.to_string_lossy().to_string();
        let mut value = |name: &str| it.next().map(PathBuf::from).ok_or(format!("{name} precisa de um valor"));
        match s.as_str() {
            "-h" | "--help" | "--ajuda" => return Err(usage().into()),
            "--json" => a.json = Some(value("--json")?),
            "--json-ao-lado" => a.json_beside = true,
            "--txt-ao-lado" => a.txt_beside = true,
            "--cena" => a.scene = Some(value("--cena")?),
            "--fontes" => a.opts.search_dirs.push(value("--fontes")?),
            "--codepage" => a.opts.default_dxf_encoding = Some(value("--codepage")?.to_string_lossy().into()),
            "--sem-recuperacao" => a.opts.allow_recovery = false,
            "--linhas" => {
                a.max_rows = value("--linhas")?
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "--linhas precisa de um número".to_string())?
            }
            _ if s.starts_with("--") => return Err(format!("opção desconhecida: {s}\n{}", usage())),
            _ => a.inputs.push(PathBuf::from(arg)),
        }
    }
    if a.inputs.is_empty() {
        return Err(usage().into());
    }
    if a.json.is_some() && a.inputs.len() > 1 {
        return Err("--json aceita um único desenho; use --json-ao-lado para vários".into());
    }
    Ok(a)
}

/// Saídas nunca podem coincidir com o arquivo de origem.
fn ensure_not_source(out: &Path, source: &Path) -> Result<(), String> {
    let same = match (out.canonicalize(), source.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => out == source,
    };
    let bad_ext = out
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|e| e == "dwg" || e == "dxf");
    if same || bad_ext {
        return Err(format!(
            "recusado: a saída {} seria o desenho de origem ou um DWG/DXF",
            out.display()
        ));
    }
    Ok(())
}

fn beside(source: &Path, suffix: &str) -> PathBuf {
    let mut name = source.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    source.with_file_name(name)
}

fn write_output(out: &Path, source: &Path, content: &str) -> Result<(), String> {
    ensure_not_source(out, source)?;
    std::fs::write(out, content).map_err(|e| format!("não foi possível gravar {}: {e}", out.display()))
}

fn main() -> ExitCode {
    // Console do Windows: força UTF-8 para os acentos aparecerem certos.
    #[cfg(windows)]
    windows_console::utf8();

    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let mut failed = false;
    for input in &args.inputs {
        match import_file(input, &args.opts) {
            Ok(imported) => {
                let r = &imported.report;
                let txt = cadplot_core::text::render(r, args.max_rows);
                println!("{txt}");
                let json = serde_json::to_string_pretty(r).expect("serializar relatório");
                let mut outs: Vec<(PathBuf, &str)> = vec![];
                if let Some(p) = &args.json {
                    outs.push((p.clone(), json.as_str()));
                }
                if args.json_beside {
                    outs.push((beside(input, ".report.json"), json.as_str()));
                }
                if args.txt_beside {
                    outs.push((beside(input, ".report.txt"), txt.as_str()));
                }
                for (p, content) in outs {
                    match write_output(&p, input, content) {
                        Ok(()) => eprintln!("relatório gravado em {}", p.display()),
                        Err(e) => {
                            eprintln!("{e}");
                            failed = true;
                        }
                    }
                }
                if let Some(p) = &args.scene {
                    let t = std::time::Instant::now();
                    let scene = cadplot_core::scene::build_model_scene(&imported.document);
                    let report = serde_json::to_value(r).ok();
                    let bytes = scene.to_bytes(report.as_ref());
                    let res = ensure_not_source(p, input)
                        .and_then(|_| std::fs::write(p, &bytes).map_err(|e| format!("não foi possível gravar {}: {e}", p.display())));
                    match res {
                        Ok(()) => eprintln!(
                            "cena gravada em {} ({:.1} MB, {} lotes, {} textos, {:.0} ms)",
                            p.display(),
                            bytes.len() as f64 / 1e6,
                            scene.meta.batches.len(),
                            scene.meta.texts.len(),
                            t.elapsed().as_secs_f64() * 1000.0
                        ),
                        Err(e) => {
                            eprintln!("{e}");
                            failed = true;
                        }
                    }
                }
                if r.read.recovery_mode {
                    failed = true;
                }
            }
            Err(e) => {
                eprintln!("ERRO ao abrir {}: {e}", input.display());
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(windows)]
mod windows_console {
    #[link(name = "kernel32")]
    extern "system" {
        fn SetConsoleOutputCP(code_page: u32) -> i32;
    }
    pub fn utf8() {
        // SAFETY: chamada simples da API do console, sem ponteiros.
        unsafe {
            SetConsoleOutputCP(65001);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recusa_gravar_na_origem() {
        let src = Path::new("planta.dwg");
        assert!(ensure_not_source(src, src).is_err());
        assert!(ensure_not_source(Path::new("outra.dxf"), src).is_err());
        assert!(ensure_not_source(Path::new("planta.dwg.report.json"), src).is_ok());
        assert!(ensure_not_source(Path::new("planta.cps"), src).is_ok());
    }

    #[test]
    fn nome_ao_lado() {
        assert_eq!(beside(Path::new("a/b/Planta Ç.dwg"), ".report.json"), PathBuf::from("a/b/Planta Ç.dwg.report.json"));
    }
}
