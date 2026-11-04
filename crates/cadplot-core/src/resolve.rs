//! Resolução de recursos externos: fontes SHX/TTF, xrefs e imagens.
//!
//! Só consulta a existência de arquivos (metadados). Nunca abre para escrita.

use std::path::{Path, PathBuf};

use crate::report::{FontKind, ResolveStatus};

#[derive(Debug, Clone, Default)]
pub struct SearchPaths {
    /// Pasta do desenho (sempre consultada primeiro).
    pub drawing_dir: Option<PathBuf>,
    /// Pastas extras configuradas pelo usuário (fontes SHX da empresa etc.).
    pub extra_dirs: Vec<PathBuf>,
    /// Pasta de fontes do sistema (Windows: %WINDIR%\Fonts).
    pub system_font_dir: Option<PathBuf>,
}

impl SearchPaths {
    pub fn for_drawing(drawing: &Path, extra_dirs: Vec<PathBuf>) -> Self {
        Self {
            drawing_dir: drawing.parent().map(Path::to_path_buf),
            extra_dirs,
            system_font_dir: system_font_dir(),
        }
    }
}

fn system_font_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        let windir = std::env::var_os("WINDIR").map(PathBuf::from)?;
        Some(windir.join("Fonts"))
    } else {
        None
    }
}

pub fn font_kind(name: &str) -> FontKind {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".shx") || !lower.contains('.') {
        // Sem extensão o AutoCAD assume SHX.
        FontKind::Shx
    } else if lower.ends_with(".ttf") || lower.ends_with(".ttc") || lower.ends_with(".otf") {
        FontKind::TrueType
    } else {
        FontKind::Other
    }
}

/// Procura uma fonte pelo nome do arquivo (case-insensitive no nome).
pub fn resolve_font(name: &str, paths: &SearchPaths) -> (ResolveStatus, Option<PathBuf>) {
    let kind = font_kind(name);
    let file_name = file_name_of(name);
    let file_name = if kind == FontKind::Shx && !file_name.to_ascii_lowercase().ends_with(".shx") {
        format!("{file_name}.shx")
    } else {
        file_name
    };

    let mut dirs: Vec<&Path> = Vec::new();
    if let Some(d) = &paths.drawing_dir {
        dirs.push(d);
    }
    dirs.extend(paths.extra_dirs.iter().map(PathBuf::as_path));
    if kind == FontKind::TrueType {
        if let Some(d) = &paths.system_font_dir {
            dirs.push(d);
        }
    }
    // Caminho absoluto gravado no estilo.
    let stored = Path::new(name);
    if stored.is_absolute() && stored.is_file() {
        return (ResolveStatus::Found, Some(stored.to_path_buf()));
    }
    for d in dirs {
        if let Some(p) = find_case_insensitive(d, &file_name) {
            return (ResolveStatus::Found, Some(p));
        }
    }
    if kind == FontKind::TrueType && paths.system_font_dir.is_none() {
        return (ResolveStatus::NotChecked, None);
    }
    (ResolveStatus::Missing, None)
}

/// Resolve um caminho de xref/imagem como o AutoCAD faz: caminho gravado
/// (absoluto ou relativo à pasta do desenho) e, se não achar, só o nome do
/// arquivo na pasta do desenho e nas pastas extras.
pub fn resolve_external(stored: &str, paths: &SearchPaths) -> (ResolveStatus, Option<PathBuf>) {
    if stored.trim().is_empty() {
        return (ResolveStatus::Missing, None);
    }
    let normalized = stored.replace('\\', std::path::MAIN_SEPARATOR_STR);
    let p = Path::new(&normalized);
    if p.is_absolute() && p.is_file() {
        return (ResolveStatus::Found, Some(p.to_path_buf()));
    }
    if let Some(dir) = &paths.drawing_dir {
        if !p.is_absolute() {
            let cand = dir.join(p);
            if cand.is_file() {
                return (ResolveStatus::Found, Some(cand));
            }
        }
    }
    let file_name = file_name_of(stored);
    let mut dirs: Vec<&Path> = Vec::new();
    if let Some(d) = &paths.drawing_dir {
        dirs.push(d);
    }
    dirs.extend(paths.extra_dirs.iter().map(PathBuf::as_path));
    for d in dirs {
        if let Some(found) = find_case_insensitive(d, &file_name) {
            return (ResolveStatus::Found, Some(found));
        }
    }
    (ResolveStatus::Missing, None)
}

/// Nome de arquivo aceitando separadores Windows mesmo fora do Windows.
fn file_name_of(stored: &str) -> String {
    stored
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(stored)
        .to_string()
}

fn find_case_insensitive(dir: &Path, file_name: &str) -> Option<PathBuf> {
    let direct = dir.join(file_name);
    if direct.is_file() {
        return Some(direct);
    }
    let wanted = file_name.to_lowercase();
    let rd = std::fs::read_dir(dir).ok()?;
    for entry in rd.flatten() {
        if entry.file_name().to_string_lossy().to_lowercase() == wanted {
            let p = entry.path();
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipos_de_fonte() {
        assert_eq!(font_kind("romans.shx"), FontKind::Shx);
        assert_eq!(font_kind("ROMANS"), FontKind::Shx);
        assert_eq!(font_kind("arial.ttf"), FontKind::TrueType);
    }

    #[test]
    fn nome_com_barra_invertida() {
        assert_eq!(file_name_of(r"C:\Projetos\XREF\base.dwg"), "base.dwg");
        assert_eq!(file_name_of("..\\x\\y.dwg"), "y.dwg");
        assert_eq!(file_name_of("y.dwg"), "y.dwg");
    }

    #[test]
    fn resolve_em_pasta_temporaria() {
        let dir = std::env::temp_dir().join(format!("cadplot-resolve-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Romans.SHX"), b"x").unwrap();
        std::fs::write(dir.join("Planta Base.dwg"), b"x").unwrap();
        let paths = SearchPaths {
            drawing_dir: Some(dir.clone()),
            extra_dirs: vec![],
            system_font_dir: None,
        };
        assert_eq!(resolve_font("romans.shx", &paths).0, ResolveStatus::Found);
        assert_eq!(resolve_font("romans", &paths).0, ResolveStatus::Found);
        assert_eq!(resolve_font("txt.shx", &paths).0, ResolveStatus::Missing);
        assert_eq!(resolve_font("arial.ttf", &paths).0, ResolveStatus::NotChecked);
        assert_eq!(
            resolve_external(r"C:\Outro\Lugar\Planta Base.dwg", &paths).0,
            ResolveStatus::Found
        );
        assert_eq!(resolve_external(r".\xref\falta.dwg", &paths).0, ResolveStatus::Missing);
        std::fs::remove_dir_all(&dir).ok();
    }
}
