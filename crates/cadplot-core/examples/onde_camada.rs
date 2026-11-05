//! Diagnóstico: onde estão as entidades de uma camada (espaço/bloco) e onde
//! cada bloco é inserido. Uso: cargo run -p cadplot-core --example onde_camada -- <desenho> <camada>
use std::collections::BTreeMap;
use acadrust::EntityType;

fn main() {
    let mut a = std::env::args().skip(1);
    let (path, layer) = (a.next().expect("desenho"), a.next().expect("camada"));
    let doc = cadplot_core::import_file(path.as_ref(), &Default::default()).expect("abrir").document;
    let mut owner = std::collections::HashMap::new();
    for br in doc.block_records.iter() {
        for h in &br.entity_handles { owner.insert(*h, br.name.clone()); }
    }
    let mut by_owner: BTreeMap<String, usize> = BTreeMap::new();
    for e in doc.entities() {
        if e.common().layer.eq_ignore_ascii_case(&layer) {
            *by_owner.entry(owner.get(&e.common().handle).cloned().unwrap_or("?".into())).or_default() += 1;
        }
    }
    println!("camada {layer}:");
    for (o, n) in &by_owner {
        let mut ins: BTreeMap<String, usize> = BTreeMap::new();
        for e in doc.entities() {
            if let EntityType::Insert(i) = e {
                if &i.block_name == o { *ins.entry(owner.get(&i.common.handle).cloned().unwrap_or("?".into())).or_default() += 1; }
            }
        }
        println!("  {n:>4} em {o}  inserido em: {ins:?}");
    }
}
