//! CadPlot — app desktop (Tauri 2).
//!
//! O processo nativo só LÊ desenhos: abre o arquivo pelo núcleo
//! (`cadplot-core`, somente leitura), monta a cena e devolve os bytes para a
//! interface. Não há rede, telemetria nem gravação de arquivos.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use cadplot_core::{import_file, ImportOptions};
use tauri::ipc::Response;
use tauri::State;

/// Arquivo passado na linha de comando ("Abrir com…" no Windows).
struct InitialFile(Mutex<Option<String>>);

#[tauri::command]
fn initial_file(state: State<'_, InitialFile>) -> Option<String> {
    state.0.lock().ok().and_then(|mut g| g.take())
}

#[tauri::command]
async fn open_drawing(path: String) -> Result<Response, String> {
    let path = PathBuf::from(path);
    let bytes = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let imported = import_file(&path, &ImportOptions::default()).map_err(|e| e.to_string())?;
        let scene = cadplot_core::scene::build_model_scene(&imported.document);
        let report = serde_json::to_value(&imported.report).map_err(|e| e.to_string())?;
        Ok(scene.to_bytes(Some(&report)))
    })
    .await
    .map_err(|e| format!("falha interna: {e}"))??;
    Ok(Response::new(bytes))
}

fn main() {
    let initial = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .find(|a| {
            let l = a.to_lowercase();
            l.ends_with(".dwg") || l.ends_with(".dxf")
        });

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(InitialFile(Mutex::new(initial)))
        .invoke_handler(tauri::generate_handler![open_drawing, initial_file])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o CadPlot");
}
