//! Importação: abre o arquivo (somente leitura), faz o parsing com o acadrust
//! e produz o documento em memória + relatório de importação.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use acadrust::{CadDocument, DwgReadOptions, DwgReader, DxfReader, DxfReaderConfiguration, ReadOutcome};

use crate::analyze;
use crate::report::ImportReport;
use crate::resolve::SearchPaths;
use crate::source::{open_read_only, SourceError, SourceFile, SourceKind};

#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Se a leitura estrita falhar, tenta de novo em modo tolerante (failsafe)
    /// e marca o relatório como "modo de recuperação". Padrão: `true`.
    pub allow_recovery: bool,
    /// Pastas extras para procurar fontes SHX, xrefs e imagens.
    pub search_dirs: Vec<PathBuf>,
    /// Codepage a usar em DXF antigos sem $DWGCODEPAGE (ex.: "ANSI_1252").
    pub default_dxf_encoding: Option<String>,
    /// Quantas amostras de strings não-ASCII guardar no relatório.
    pub encoding_samples: usize,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            allow_recovery: true,
            search_dirs: Vec::new(),
            default_dxf_encoding: None,
            encoding_samples: 12,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error("falha ao ler o desenho: {0}")]
    Parse(String),
}

pub struct Imported {
    pub document: CadDocument,
    pub report: ImportReport,
}

pub fn import_file(path: &Path, opts: &ImportOptions) -> Result<Imported, ImportError> {
    let total = Instant::now();
    let source = open_read_only(path)?;
    let parse_started = Instant::now();
    let (outcome, strict_error) = parse(&source, opts)?;
    let parse_ms = parse_started.elapsed().as_secs_f64() * 1000.0;

    let analyze_started = Instant::now();
    let paths = SearchPaths::for_drawing(path, opts.search_dirs.clone());
    let mut report = analyze::build_report(&source, &outcome, strict_error, &paths, opts);
    report.timing.io_ms = source.io_ms;
    report.timing.parse_ms = parse_ms;
    report.timing.analyze_ms = analyze_started.elapsed().as_secs_f64() * 1000.0;
    report.timing.total_ms = total.elapsed().as_secs_f64() * 1000.0;

    Ok(Imported {
        document: outcome.document,
        report,
    })
}

fn parse(
    source: &SourceFile,
    opts: &ImportOptions,
) -> Result<(ReadOutcome, Option<String>), ImportError> {
    match parse_once(source, opts, false) {
        Ok(o) => Ok((o, None)),
        Err(strict) if opts.allow_recovery => match parse_once(source, opts, true) {
            Ok(o) => Ok((o, Some(strict))),
            Err(failsafe) => Err(ImportError::Parse(format!(
                "{strict} (modo de recuperação também falhou: {failsafe})"
            ))),
        },
        Err(strict) => Err(ImportError::Parse(strict)),
    }
}

fn parse_once(
    source: &SourceFile,
    opts: &ImportOptions,
    failsafe: bool,
) -> Result<ReadOutcome, String> {
    let cursor = Cursor::new(Arc::clone(&source.bytes));
    // O acadrust às vezes entra em pânico em arquivos corrompidos; isolamos
    // para virar erro reportável em vez de derrubar o app.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match source.kind {
        SourceKind::Dwg => {
            let mut options = DwgReadOptions::default();
            options.failsafe = failsafe;
            let mut reader = DwgReader::from_stream_with_options(cursor, options);
            reader.read_with_stats().map_err(|e| e.to_string())
        }
        SourceKind::DxfAscii | SourceKind::DxfBinary => {
            let reader = DxfReader::from_reader(cursor).map_err(|e| e.to_string())?;
            let config = DxfReaderConfiguration {
                failsafe,
                default_encoding: opts.default_dxf_encoding.clone(),
            };
            reader
                .with_configuration(config)
                .read_with_stats()
                .map_err(|e| e.to_string())
        }
    }));
    match result {
        Ok(r) => r,
        Err(panic) => {
            let msg = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "pânico sem mensagem".into());
            Err(format!("o leitor interrompeu a leitura: {msg}"))
        }
    }
}
