//! Guarda da regra "o arquivo de origem é somente leitura".
//!
//! Varre o código de produção do núcleo e da CLI e falha se aparecer qualquer
//! API de gravação de DWG/DXF ou abertura de arquivo para escrita fora dos
//! pontos permitidos. Módulos `#[cfg(test)]` são ignorados.

use std::path::Path;

const FORBIDDEN: &[&str] = &[
    "DwgWriter",
    "DxfWriter",
    "write_to_file",
    ".write(true)",
    ".append(true)",
    ".truncate(true)",
    "File::create",
    "fs::write",
    "fs::copy",
    "fs::rename",
    "fs::remove_file",
    "set_permissions",
];

/// Arquivos de produção que podem gravar saídas NOVAS (nunca a origem).
/// Cada um tem o seu próprio teste garantindo que recusa o caminho de origem.
const ALLOWED_WRITERS: &[&str] = &["crates/cadplot-inspect/src/main.rs"];

fn production_part(src: &str) -> &str {
    match src.find("#[cfg(test)]") {
        Some(i) => &src[..i],
        None => src,
    }
}

fn scan(dir: &Path, root: &Path, hits: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let p = entry.path();
        if p.is_dir() {
            scan(&p, root, hits);
        } else if p.extension().is_some_and(|e| e == "rs") {
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            let allowed = ALLOWED_WRITERS.contains(&rel.as_str());
            let src = std::fs::read_to_string(&p).unwrap();
            for (n, line) in production_part(&src).lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                for f in FORBIDDEN {
                    let is_output_write = matches!(*f, "File::create" | "fs::write");
                    if code.contains(f) && !(allowed && is_output_write) {
                        hits.push(format!("{rel}:{}: {f}", n + 1));
                    }
                }
            }
        }
    }
}

#[test]
fn nucleo_e_cli_nao_gravam_dwg_dxf() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let mut hits = Vec::new();
    for d in ["crates/cadplot-core/src", "crates/cadplot-inspect/src"] {
        let p = root.join(d);
        if p.exists() {
            scan(&p, &root, &mut hits);
        }
    }
    assert!(hits.is_empty(), "APIs de escrita encontradas em código de produção:\n{}", hits.join("\n"));
}

#[test]
fn nucleo_nao_depende_do_gerador_de_fixtures() {
    let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let deps = manifest.split("[dev-dependencies]").next().unwrap();
    assert!(!deps.contains("cadplot-fixtures"), "cadplot-fixtures só pode ser dev-dependency");
}
