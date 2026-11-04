//! Nomes estáveis e legíveis para tipos, cores, unidades e versões.

use acadrust::entities::extended_entity::ExtendedEntityData;
use acadrust::objects::ObjectType;
use acadrust::{Color, DxfVersion, EntityType, LineWeight};

/// Nome normalizado do tipo de entidade (próximo do nome DXF).
pub fn entity_type_name(e: &EntityType) -> &'static str {
    match e {
        EntityType::Point(_) => "POINT",
        EntityType::Line(_) => "LINE",
        EntityType::Circle(_) => "CIRCLE",
        EntityType::Arc(_) => "ARC",
        EntityType::Ellipse(_) => "ELLIPSE",
        EntityType::Polyline(_) => "POLYLINE",
        EntityType::Polyline2D(_) => "POLYLINE_2D",
        EntityType::Polyline3D(_) => "POLYLINE_3D",
        EntityType::LwPolyline(_) => "LWPOLYLINE",
        EntityType::Text(_) => "TEXT",
        EntityType::MText(_) => "MTEXT",
        EntityType::Spline(_) => "SPLINE",
        EntityType::Helix(_) => "HELIX",
        EntityType::Dimension(_) => "DIMENSION",
        EntityType::Hatch(_) => "HATCH",
        EntityType::Solid(_) => "SOLID",
        EntityType::Face3D(_) => "3DFACE",
        EntityType::Insert(_) => "INSERT",
        EntityType::Block(_) => "BLOCK",
        EntityType::BlockEnd(_) => "ENDBLK",
        EntityType::Ray(_) => "RAY",
        EntityType::XLine(_) => "XLINE",
        EntityType::Viewport(_) => "VIEWPORT",
        EntityType::AttributeDefinition(_) => "ATTDEF",
        EntityType::AttributeEntity(_) => "ATTRIB",
        EntityType::Leader(_) => "LEADER",
        EntityType::MultiLeader(_) => "MULTILEADER",
        EntityType::MLine(_) => "MLINE",
        EntityType::Mesh(_) => "MESH",
        EntityType::RasterImage(_) => "IMAGE",
        EntityType::Solid3D(_) => "3DSOLID",
        EntityType::Region(_) => "REGION",
        EntityType::Body(_) => "BODY",
        EntityType::Surface(_) => "SURFACE",
        EntityType::Table(_) => "ACAD_TABLE",
        EntityType::Tolerance(_) => "TOLERANCE",
        EntityType::PolyfaceMesh(_) => "POLYFACE_MESH",
        EntityType::Wipeout(_) => "WIPEOUT",
        EntityType::Shape(_) => "SHAPE",
        EntityType::Underlay(_) => "UNDERLAY",
        EntityType::Seqend(_) => "SEQEND",
        EntityType::Ole2Frame(_) => "OLE2FRAME",
        EntityType::PolygonMesh(_) => "POLYGON_MESH",
        EntityType::Light(_) => "LIGHT",
        EntityType::SectionSymbol(_) => "SECTION_SYMBOL",
        EntityType::ViewBorder(_) => "VIEWBORDER",
        EntityType::Extended(_) => "EXTENDED",
        EntityType::Unknown(_) => "UNKNOWN",
    }
}

pub fn extended_kind_name(d: &ExtendedEntityData) -> &'static str {
    match d {
        ExtendedEntityData::Camera { .. } => "CAMERA",
        ExtendedEntityData::SectionObject(_) => "SECTIONOBJECT",
        ExtendedEntityData::ArcAlignedText(_) => "ARCALIGNEDTEXT",
        ExtendedEntityData::RemoteText(_) => "RTEXT",
        ExtendedEntityData::GeoPositionMarker(_) => "POSITIONMARKER",
        ExtendedEntityData::CoordinationModel(_) => "COORDINATION_MODEL",
        ExtendedEntityData::PointCloud(_) => "POINTCLOUD",
        ExtendedEntityData::PointCloudEx(_) => "POINTCLOUDEX",
        ExtendedEntityData::Proxy(_) => "ACAD_PROXY_ENTITY",
        ExtendedEntityData::OleFrame(_) => "OLEFRAME",
        ExtendedEntityData::LayoutPrintConfig(_) => "LAYOUTPRINTCONFIG",
        ExtendedEntityData::Format(_) => "FORMAT",
        ExtendedEntityData::Legacy(_) => "LEGACY",
        ExtendedEntityData::DynamicBlock(_) => "DYNAMIC_BLOCK",
        ExtendedEntityData::RegisteredClass(_) => "REGISTERED_CLASS",
        ExtendedEntityData::Arch(_) => "AEC_ARCH",
        #[allow(unreachable_patterns)]
        _ => "OTHER",
    }
}

pub fn object_type_name(o: &ObjectType) -> String {
    let s = match o {
        ObjectType::Dictionary(_) => "DICTIONARY",
        ObjectType::Layout(_) => "LAYOUT",
        ObjectType::XRecord(_) => "XRECORD",
        ObjectType::Group(_) => "GROUP",
        ObjectType::MLineStyle(_) => "MLINESTYLE",
        ObjectType::ImageDefinition(_) => "IMAGEDEF",
        ObjectType::UnderlayDefinition(_) => "UNDERLAYDEFINITION",
        ObjectType::PlotSettings(_) => "PLOTSETTINGS",
        ObjectType::MultiLeaderStyle(_) => "MLEADERSTYLE",
        ObjectType::TableStyle(_) => "TABLESTYLE",
        ObjectType::TableContent(_) => "TABLECONTENT",
        ObjectType::Scale(_) => "SCALE",
        ObjectType::ObjectContextData(_) => "OBJECTCONTEXTDATA",
        ObjectType::SortEntitiesTable(_) => "SORTENTSTABLE",
        ObjectType::DictionaryVariable(_) => "DICTIONARYVAR",
        ObjectType::VisualStyle(_) => "VISUALSTYLE",
        ObjectType::Material(_) => "MATERIAL",
        ObjectType::ImageDefinitionReactor(_) => "IMAGEDEF_REACTOR",
        ObjectType::GeoData(_) => "GEODATA",
        ObjectType::SpatialFilter(_) => "SPATIAL_FILTER",
        ObjectType::RasterVariables(_) => "RASTERVARIABLES",
        ObjectType::BookColor(_) => "DBCOLOR",
        ObjectType::PlaceHolder(_) => "ACDBPLACEHOLDER",
        ObjectType::DictionaryWithDefault(_) => "ACDBDICTIONARYWDFLT",
        ObjectType::WipeoutVariables(_) => "WIPEOUTVARIABLES",
        ObjectType::BlockVisibilityParameter(_) => "BLOCKVISIBILITYPARAMETER",
        ObjectType::DynamicBlock(_) => "DYNAMIC_BLOCK_OBJECT",
        ObjectType::Associative(_) => "ASSOCIATIVE",
        ObjectType::ClassObject(_) => "CLASS_OBJECT",
        ObjectType::DataObject(_) => "DATA_OBJECT",
        ObjectType::Field(_) => "FIELD",
        ObjectType::FieldList(_) => "FIELDLIST",
        ObjectType::RegisteredClass(_) => "REGISTERED_CLASS",
        ObjectType::DgnLineStyle(_) => "DGN_LINESTYLE",
        ObjectType::ProxyObject(_) => "ACAD_PROXY_OBJECT",
        ObjectType::Arch(_) => "AEC_ARCH",
        ObjectType::Unknown { type_name, .. } => return format!("UNKNOWN:{type_name}"),
        #[allow(unreachable_patterns)]
        _ => "OTHER",
    };
    s.to_string()
}

pub fn color_label(c: &Color) -> String {
    match c {
        Color::ByLayer => "PorCamada".into(),
        Color::ByBlock => "PorBloco".into(),
        Color::None => "Nenhuma".into(),
        Color::Index(i) => format!("ACI {i}"),
        Color::Rgb { r, g, b } => format!("RGB {r},{g},{b}"),
    }
}

pub fn lineweight_label(w: &LineWeight) -> String {
    match w {
        LineWeight::ByLayer => "PorCamada".into(),
        LineWeight::ByBlock => "PorBloco".into(),
        LineWeight::Default => "Padrão".into(),
        LineWeight::Value(v) => format!("{:.2} mm", *v as f64 / 100.0),
        #[allow(unreachable_patterns)]
        other => format!("{other:?}"),
    }
}

pub fn version_code(v: DxfVersion) -> &'static str {
    match v {
        DxfVersion::AC1009 => "AC1009",
        DxfVersion::AC1012 => "AC1012",
        DxfVersion::AC1014 => "AC1014",
        DxfVersion::AC1015 => "AC1015",
        DxfVersion::AC1018 => "AC1018",
        DxfVersion::AC1021 => "AC1021",
        DxfVersion::AC1024 => "AC1024",
        DxfVersion::AC1027 => "AC1027",
        DxfVersion::AC1032 => "AC1032",
        _ => "desconhecida",
    }
}

pub fn version_name(code: &str) -> &'static str {
    match code {
        "AC1006" => "AutoCAD R10",
        "AC1009" => "AutoCAD R11/R12",
        "AC1012" => "AutoCAD R13",
        "AC1014" => "AutoCAD R14",
        "AC1015" => "AutoCAD 2000–2002",
        "AC1018" => "AutoCAD 2004–2006",
        "AC1021" => "AutoCAD 2007–2009",
        "AC1024" => "AutoCAD 2010–2012",
        "AC1027" => "AutoCAD 2013–2017",
        "AC1032" => "AutoCAD 2018–2027 (formato DWG 2018)",
        _ => "versão não reconhecida",
    }
}

/// Nome de $INSUNITS.
pub fn insunits_name(code: i16) -> &'static str {
    match code {
        0 => "sem unidade",
        1 => "polegadas",
        2 => "pés",
        3 => "milhas",
        4 => "milímetros",
        5 => "centímetros",
        6 => "metros",
        7 => "quilômetros",
        8 => "micropolegadas",
        9 => "mils",
        10 => "jardas",
        11 => "angstroms",
        12 => "nanômetros",
        13 => "mícrons",
        14 => "decímetros",
        15 => "decâmetros",
        16 => "hectômetros",
        17 => "gigâmetros",
        18 => "unidades astronômicas",
        19 => "anos-luz",
        20 => "parsecs",
        21 => "pés topográficos (EUA)",
        22 => "polegadas topográficas (EUA)",
        23 => "jardas topográficas (EUA)",
        24 => "milhas topográficas (EUA)",
        _ => "código desconhecido",
    }
}

/// Fator de conversão de uma unidade $INSUNITS para milímetros, quando conhecido.
pub fn insunits_to_mm(code: i16) -> Option<f64> {
    Some(match code {
        1 => 25.4,
        2 => 304.8,
        4 => 1.0,
        5 => 10.0,
        6 => 1000.0,
        7 => 1_000_000.0,
        14 => 100.0,
        _ => return None,
    })
}

/// "1:50" a partir de um fator papel/desenho (0,02 → "1:50").
pub fn scale_label(factor: f64) -> Option<String> {
    if !(factor.is_finite() && factor > 0.0) {
        return None;
    }
    // Tolerância relativa: viewports guardam altura/altura da vista em double e
    // 1:50 costuma vir como 1:50,0000004.
    let close = |v: f64| (v - v.round()).abs() <= 1e-6 * v.abs().max(1.0) * 100.0;
    if factor >= 1.0 {
        let n = factor;
        Some(if close(n) {
            format!("{}:1", n.round() as i64)
        } else {
            format!("{}:1", trim_dec(n))
        })
    } else {
        let d = 1.0 / factor;
        Some(if close(d) {
            format!("1:{}", d.round() as i64)
        } else {
            format!("1:{}", trim_dec(d))
        })
    }
}

fn trim_dec(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escalas_com_ruido() {
        assert_eq!(scale_label(1.0 / 50.000_000_4).as_deref(), Some("1:50"));
        assert_eq!(scale_label(20.000_000_3).as_deref(), Some("20:1"));
        assert_eq!(scale_label(1.0 / 1.5).as_deref(), Some("1:1.5"));
        assert_eq!(scale_label(1.0 / 12.5).as_deref(), Some("1:12.5"));
    }

    #[test]
    fn escalas() {
        assert_eq!(scale_label(0.02).as_deref(), Some("1:50"));
        assert_eq!(scale_label(0.01).as_deref(), Some("1:100"));
        assert_eq!(scale_label(1.0).as_deref(), Some("1:1"));
        assert_eq!(scale_label(2.0).as_deref(), Some("2:1"));
        assert_eq!(scale_label(0.0), None);
        assert_eq!(scale_label(f64::NAN), None);
    }

    #[test]
    fn versoes() {
        assert!(version_name("AC1032").contains("2027"));
        assert_eq!(version_code(DxfVersion::AC1032), "AC1032");
    }
}
