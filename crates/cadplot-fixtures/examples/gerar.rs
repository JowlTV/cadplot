//! Gera todas as variantes da planta sintética numa pasta.
//! Uso: cargo run -p cadplot-fixtures --example gerar -- <pasta>
fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "out/fixtures".into()));
    std::fs::create_dir_all(&dir).expect("pasta");
    for v in cadplot_fixtures::VARIANTS {
        match cadplot_fixtures::write_variant(&dir, *v) {
            Ok((p, _)) => println!("ok    {}", p.display()),
            Err(e) => println!("ERRO  {v:?}: {e}"),
        }
    }
}
