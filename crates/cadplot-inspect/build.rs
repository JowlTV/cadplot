//! Embute metadados de versão no executável do Windows (sem ícone por enquanto).
fn main() {
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let mut res = winresource::WindowsResource::new();
            res.set_language(0x0416); // pt-BR
            if let Err(e) = res.compile() {
                println!("cargo:warning=metadados de versão não embutidos: {e}");
            }
        }
    }
}
