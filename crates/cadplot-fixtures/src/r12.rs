//! DXF R12 (AC1009) escrito à mão, em Windows-1252, para testar o caminho de
//! codepage de arquivos antigos (onde acentos costumam quebrar).

use std::path::Path;

/// Camadas do R12 (mesmos nomes acentuados da planta principal).
pub const R12_LAYERS: &[&str] = &["PA-COMENTÁRIO", "ARQ-PAREDE", "PISO-DIVISÃO", "VEG-ÁRVORES"];
pub const R12_TEXT: &str = "Canteiro de hortênsias — irrigação";

/// Codifica em Windows-1252 (só os caracteres usados aqui).
fn cp1252(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c {
            '—' => 0x97,
            c if (c as u32) < 0x100 => c as u32 as u8,
            other => panic!("caractere {other:?} fora do conjunto do fixture"),
        })
        .collect()
}

struct W(Vec<u8>);

impl W {
    fn pair(&mut self, code: i32, value: &str) {
        self.0.extend_from_slice(format!("{code:>3}\r\n").as_bytes());
        self.0.extend_from_slice(&cp1252(value));
        self.0.extend_from_slice(b"\r\n");
    }
    fn f(&mut self, code: i32, v: f64) {
        self.pair(code, &format!("{v:.6}"));
    }
}

/// Esperado no R12: 4 LINE, 1 CIRCLE, 1 ARC, 1 TEXT, 1 POLYLINE, 1 INSERT no modelo.
pub fn write(path: &Path) -> std::io::Result<()> {
    let mut w = W(Vec::new());
    w.pair(0, "SECTION");
    w.pair(2, "HEADER");
    w.pair(9, "$ACADVER");
    w.pair(1, "AC1009");
    w.pair(9, "$DWGCODEPAGE");
    w.pair(3, "ANSI_1252");
    w.pair(9, "$EXTMIN");
    w.f(10, 0.0);
    w.f(20, 0.0);
    w.pair(9, "$EXTMAX");
    w.f(10, 10_000.0);
    w.f(20, 8_000.0);
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "TABLES");
    w.pair(0, "TABLE");
    w.pair(2, "LTYPE");
    w.pair(70, "1");
    w.pair(0, "LTYPE");
    w.pair(2, "CONTINUOUS");
    w.pair(70, "0");
    w.pair(3, "Solid line");
    w.pair(72, "65");
    w.pair(73, "0");
    w.f(40, 0.0);
    w.pair(0, "ENDTAB");
    w.pair(0, "TABLE");
    w.pair(2, "LAYER");
    w.pair(70, &(R12_LAYERS.len() + 1).to_string());
    for (i, name) in std::iter::once(&"0").chain(R12_LAYERS.iter()).enumerate() {
        w.pair(0, "LAYER");
        w.pair(2, name);
        w.pair(70, "0");
        // Cor negativa = camada desligada (PISO-DIVISÃO).
        let color = if *name == "PISO-DIVISÃO" { -8 } else { (i as i32 % 7) + 1 };
        w.pair(62, &color.to_string());
        w.pair(6, "CONTINUOUS");
    }
    w.pair(0, "ENDTAB");
    w.pair(0, "TABLE");
    w.pair(2, "STYLE");
    w.pair(70, "1");
    w.pair(0, "STYLE");
    w.pair(2, "STANDARD");
    w.pair(70, "0");
    w.f(40, 0.0);
    w.f(41, 1.0);
    w.f(50, 0.0);
    w.pair(71, "0");
    w.f(42, 2.5);
    w.pair(3, "txt");
    w.pair(4, "");
    w.pair(0, "ENDTAB");
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "BLOCKS");
    w.pair(0, "BLOCK");
    w.pair(8, "0");
    w.pair(2, "VASO");
    w.pair(70, "0");
    w.f(10, 0.0);
    w.f(20, 0.0);
    w.f(30, 0.0);
    w.pair(3, "VASO");
    w.pair(0, "CIRCLE");
    w.pair(8, "0");
    w.f(10, 0.0);
    w.f(20, 0.0);
    w.f(30, 0.0);
    w.f(40, 200.0);
    w.pair(0, "ENDBLK");
    w.pair(8, "0");
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "ENTITIES");
    let pts = [(0.0, 0.0), (10_000.0, 0.0), (10_000.0, 8_000.0), (0.0, 8_000.0)];
    for i in 0..4 {
        let (a, b) = (pts[i], pts[(i + 1) % 4]);
        w.pair(0, "LINE");
        w.pair(8, "ARQ-PAREDE");
        w.f(10, a.0);
        w.f(20, a.1);
        w.f(30, 0.0);
        w.f(11, b.0);
        w.f(21, b.1);
        w.f(31, 0.0);
    }
    w.pair(0, "CIRCLE");
    w.pair(8, "VEG-ÁRVORES");
    w.f(10, 8_000.0);
    w.f(20, 6_000.0);
    w.f(30, 0.0);
    w.f(40, 1_200.0);
    w.pair(0, "ARC");
    w.pair(8, "VEG-ÁRVORES");
    w.f(10, 2_000.0);
    w.f(20, 6_000.0);
    w.f(30, 0.0);
    w.f(40, 800.0);
    w.f(50, 0.0);
    w.f(51, 180.0);
    w.pair(0, "TEXT");
    w.pair(8, "PA-COMENTÁRIO");
    w.f(10, 1_000.0);
    w.f(20, 500.0);
    w.f(30, 0.0);
    w.f(40, 125.0);
    w.pair(1, R12_TEXT);
    // POLYLINE 2D com bulge no primeiro vértice.
    w.pair(0, "POLYLINE");
    w.pair(8, "PISO-DIVISÃO");
    w.pair(66, "1");
    w.f(10, 0.0);
    w.f(20, 0.0);
    w.f(30, 0.0);
    w.pair(70, "0");
    for (x, y, bulge) in [(1_000.0, 1_000.0, 0.0), (4_000.0, 1_000.0, 0.414_213_562_373_095), (4_500.0, 1_500.0, 0.0)] {
        w.pair(0, "VERTEX");
        w.pair(8, "PISO-DIVISÃO");
        w.f(10, x);
        w.f(20, y);
        w.f(30, 0.0);
        if bulge != 0.0 {
            w.pair(42, &format!("{bulge:.15}"));
        }
    }
    w.pair(0, "SEQEND");
    w.pair(8, "PISO-DIVISÃO");
    w.pair(0, "INSERT");
    w.pair(8, "VEG-ÁRVORES");
    w.pair(2, "VASO");
    w.f(10, 9_000.0);
    w.f(20, 1_000.0);
    w.f(30, 0.0);
    w.pair(0, "ENDSEC");
    w.pair(0, "EOF");
    std::fs::write(path, w.0)
}
