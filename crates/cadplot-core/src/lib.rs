//! CadPlot: núcleo de leitura.
//!
//! Princípios (inegociáveis):
//! - o arquivo de origem é somente leitura; o núcleo não contém nenhum
//!   caminho de código que grave DWG/DXF ou escreva no arquivo de origem;
//! - coordenadas em `f64`, sem arredondamento;
//! - nenhuma entidade é descartada em silêncio: tudo que não for renderizado
//!   aparece no relatório de importação.

pub mod analyze;
pub mod import;
pub mod naming;
pub mod report;
pub mod resolve;
pub mod scene;
pub mod source;
pub mod support;
pub mod text;

pub use import::{import_file, ImportError, ImportOptions, Imported};
pub use report::ImportReport;
