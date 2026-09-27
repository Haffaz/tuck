mod classifier;
mod engine;
mod files;
mod model;
use engine::Engine;
use model::{Preferences, Snapshot};
use std::{path::PathBuf, sync::Arc};
use tauri::Manager;

#[tauri::command]
async fn get_state(engine: tauri::State<'_, Arc<Engine>>) -> Result<Snapshot, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.snapshot())
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn save_preferences(
    preferences: Preferences,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<(), String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.save_preferences(preferences))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn set_watching(enabled: bool, engine: tauri::State<Arc<Engine>>) -> Result<(), String> {
    engine.watch(enabled)
}
#[tauri::command]
async fn import_documents(
    paths: Vec<String>,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<usize, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        engine.enqueue(paths.into_iter().map(PathBuf::from).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn organize_existing(engine: tauri::State<'_, Arc<Engine>>) -> Result<usize, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.organize_existing())
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn undo_action(id: String, engine: tauri::State<'_, Arc<Engine>>) -> Result<(), String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.undo(&id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn check_model() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(classifier::models)
        .await
        .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let storage = app.path().app_data_dir()?.join("state.json");
            let reader = if cfg!(debug_assertions) {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader")
            } else {
                app.path().resource_dir()?.join("resources/tuck-reader")
            };
            let engine = Arc::new(Engine::new(storage, reader).map_err(std::io::Error::other)?);
            app.manage(engine.clone());
            Engine::start(engine);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_preferences,
            set_watching,
            import_documents,
            organize_existing,
            undo_action,
            check_model
        ])
        .run(tauri::generate_context!())
        .expect("Tuck could not start");
}
