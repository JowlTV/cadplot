//! Abertura do arquivo de origem em modo estritamente somente-leitura.
//!
//! Regra de fidelidade: não existe caminho de código no núcleo que escreva no
//! arquivo de origem. O arquivo é aberto com `OpenOptions::read(true)` e nada
//! mais, lido inteiro para a memória e fechado imediatamente. Todo o parsing
//! acontece sobre o buffer em memória.

use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Formato físico detectado pelo conteúdo (não pela extensão).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Dwg,
    DxfAscii,
    DxfBinary,
}

impl SourceKind {
    pub fn label(self) -> &'static str {
        match self {
            SourceKind::Dwg => "DWG",
            SourceKind::DxfAscii => "DXF (ASCII)",
            SourceKind::DxfBinary => "DXF (binário)",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("não foi possível abrir {path}: {source}")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("não foi possível ler {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("formato não reconhecido em {path} (não parece DWG nem DXF)")]
    UnknownFormat { path: PathBuf },
}

/// Conteúdo do arquivo de origem já em memória.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub bytes: Arc<[u8]>,
    pub kind: SourceKind,
    /// Código de versão lido do cabeçalho DWG (ex.: "AC1032"). Para DXF fica
    /// `None` aqui e é obtido de `$ACADVER` depois do parsing.
    pub dwg_version_code: Option<String>,
    pub sha256: String,
    pub io_ms: f64,
}

/// Abre o arquivo apenas para leitura, lê tudo e fecha.
pub fn open_read_only(path: &Path) -> Result<SourceFile, SourceError> {
    let started = Instant::now();
    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|source| SourceError::Open {
            path: path.to_path_buf(),
            source,
        })?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|source| SourceError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    drop(file);
    let io_ms = started.elapsed().as_secs_f64() * 1000.0;

    let kind = sniff(&buf).ok_or_else(|| SourceError::UnknownFormat {
        path: path.to_path_buf(),
    })?;
    let dwg_version_code = if kind == SourceKind::Dwg {
        Some(String::from_utf8_lossy(&buf[..6]).into_owned())
    } else {
        None
    };
    let sha256 = hex(&Sha256::digest(&buf));

    Ok(SourceFile {
        path: path.to_path_buf(),
        bytes: Arc::from(buf.into_boxed_slice()),
        kind,
        dwg_version_code,
        sha256,
        io_ms,
    })
}

/// Detecta o formato pelos primeiros bytes.
pub fn sniff(bytes: &[u8]) -> Option<SourceKind> {
    if bytes.len() >= 6 && &bytes[..4] == b"AC10" && bytes[4..6].iter().all(u8::is_ascii_digit) {
        return Some(SourceKind::Dwg);
    }
    // R11/R12 DWG antigo usa "AC1009"/"AC1006"; coberto acima. Versões pré-R10 não.
    if bytes.starts_with(b"AutoCAD Binary DXF") {
        return Some(SourceKind::DxfBinary);
    }
    // DXF ASCII: primeiro par de grupo deve ser "0" seguido de SECTION (ou "999" comentário).
    let head = &bytes[..bytes.len().min(512)];
    let head = head.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(head);
    let text = String::from_utf8_lossy(head);
    let mut lines = text.lines().map(str::trim);
    match (lines.next(), lines.next()) {
        (Some("0"), Some("SECTION")) => Some(SourceKind::DxfAscii),
        (Some("999"), _) => Some(SourceKind::DxfAscii),
        _ => None,
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_dwg() {
        assert_eq!(sniff(b"AC1032\0\0\0"), Some(SourceKind::Dwg));
        assert_eq!(sniff(b"AC1009xxxx"), Some(SourceKind::Dwg));
    }

    #[test]
    fn sniff_dxf() {
        assert_eq!(sniff(b"  0\r\nSECTION\r\n  2\r\nHEADER"), Some(SourceKind::DxfAscii));
        assert_eq!(sniff(b"AutoCAD Binary DXF\r\n\x1a\0"), Some(SourceKind::DxfBinary));
        assert_eq!(sniff(b"999\nfeito por algo\n0\nSECTION"), Some(SourceKind::DxfAscii));
    }

    #[test]
    fn sniff_lixo() {
        assert_eq!(sniff(b"%PDF-1.7"), None);
        assert_eq!(sniff(b""), None);
    }
}
