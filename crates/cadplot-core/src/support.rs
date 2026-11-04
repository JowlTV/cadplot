//! Matriz de suporte de renderização (Fase 1).
//!
//! Diz, para cada tipo de entidade, o que o CadPlot pretende fazer com ele.
//! Nada é descartado em silêncio: tudo que não for `Render` vira aviso no
//! relatório e na interface.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportLevel {
    /// Renderização completa prevista na Fase 1.
    Render,
    /// Renderizado com limitações conhecidas (ex.: MTEXT só formatação básica).
    Partial,
    /// Estrutural: não desenha nada por si (SEQEND, ATTDEF dentro de bloco…).
    Structural,
    /// Fora do escopo da Fase 1 (3D, OLE…): listado como não renderizado.
    OutOfScope,
    /// Tipo que o leitor não reconheceu.
    Unknown,
}

impl SupportLevel {
    pub fn label_pt(self) -> &'static str {
        match self {
            SupportLevel::Render => "renderiza",
            SupportLevel::Partial => "parcial",
            SupportLevel::Structural => "estrutural",
            SupportLevel::OutOfScope => "não renderiza (fora do escopo)",
            SupportLevel::Unknown => "desconhecido (não renderiza)",
        }
    }

    pub fn is_rendered(self) -> bool {
        matches!(self, SupportLevel::Render | SupportLevel::Partial)
    }
}

/// Classificação pelo nome de tipo normalizado (ver `naming::entity_type_name`).
pub fn support_for(type_name: &str) -> SupportLevel {
    use SupportLevel::*;
    match type_name {
        "LINE" | "LWPOLYLINE" | "POLYLINE_2D" | "ARC" | "CIRCLE" | "ELLIPSE" | "SPLINE"
        | "TEXT" | "INSERT" | "ATTRIB" | "POINT" | "SOLID" | "VIEWPORT" | "LEADER"
        | "XLINE" | "RAY" => Render,
        // Formatação básica, padrões de hachura, bloco anônimo da cota etc.
        "MTEXT" | "HATCH" | "DIMENSION" | "MULTILEADER" | "ACAD_TABLE" | "WIPEOUT"
        | "MLINE" | "TOLERANCE" | "SHAPE" | "POLYLINE" | "POLYLINE_3D" => Partial,
        "ATTDEF" | "SEQEND" | "BLOCK" | "ENDBLK" => Structural,
        "UNKNOWN" => Unknown,
        _ => OutOfScope,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipos_da_fase_1_sao_renderizados() {
        for t in [
            "LINE", "LWPOLYLINE", "POLYLINE_2D", "ARC", "CIRCLE", "ELLIPSE", "SPLINE", "TEXT",
            "MTEXT", "HATCH", "INSERT", "DIMENSION", "LEADER", "MULTILEADER", "ACAD_TABLE",
            "VIEWPORT",
        ] {
            assert!(support_for(t).is_rendered(), "{t} deveria ser renderizado");
        }
    }

    #[test]
    fn tres_d_e_desconhecidos_nao() {
        for t in ["3DSOLID", "REGION", "BODY", "MESH", "SURFACE", "OLE2FRAME", "LIGHT"] {
            assert_eq!(support_for(t), SupportLevel::OutOfScope);
        }
        assert_eq!(support_for("UNKNOWN"), SupportLevel::Unknown);
    }
}
