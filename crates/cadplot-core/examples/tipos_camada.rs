//! Diagnóstico: tipos e resumo das entidades de uma camada no espaço modelo.
use acadrust::EntityType;
fn main() {
    let mut a = std::env::args().skip(1);
    let (path, layer) = (a.next().expect("desenho"), a.next().expect("camada"));
    let doc = cadplot_core::import_file(path.as_ref(), &Default::default()).expect("abrir").document;
    for e in doc.model_space_entities() {
        if !e.common().layer.eq_ignore_ascii_case(&layer) { continue; }
        let t = cadplot_core::naming::entity_type_name(e);
        let extra = match e {
            EntityType::Hatch(h) => format!("solid={} paths={} edges={:?} pattern={:?}", h.is_solid, h.paths.len(), h.paths.iter().map(|p| p.edges.len()).collect::<Vec<_>>(), h.pattern.name),
            EntityType::Insert(i) => format!("bloco={}", i.block_name),
            _ => String::new(),
        };
        println!("{t} {:X} {extra}", e.common().handle.value());
    }
}
