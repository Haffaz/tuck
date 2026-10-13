mod cards;
mod cluster;
mod collection;
mod config;
mod engine;
#[cfg(test)]
mod evaluation;
mod files;
mod filing;
mod helper;
mod instructions;
mod model;
mod model_lock;
mod native;
mod parallel;
mod provider;
mod reader;
mod suggest;
mod watching;
use engine::Engine;
use model::{Approval, Preferences, Snapshot};
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
async fn choose_folder(root: String, engine: tauri::State<'_, Arc<Engine>>) -> Result<(), String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.choose_folder(&root))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn map_folder(engine: tauri::State<'_, Arc<Engine>>) -> Result<(), String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.map_folder())
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn start_tuck(watch: bool, engine: tauri::State<'_, Arc<Engine>>) -> Result<usize, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.start_tuck(watch))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn stop_tuck(engine: tauri::State<Arc<Engine>>) {
    engine.stop_tuck();
}

#[tauri::command]
async fn save_preferences(
    preferences: Preferences,
    api_key: Option<String>,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<(), String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || match api_key {
        Some(key) if !key.is_empty() => engine.save_preferences_with_key(preferences, Some(key)),
        _ => engine.save_preferences(preferences),
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn suggest_categories(
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<suggest::Proposal, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.suggest_categories())
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn import_documents(
    paths: Vec<String>,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<usize, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        engine.import(paths.into_iter().map(PathBuf::from).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn update_review(
    id: String,
    folder: String,
    title: String,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<(), String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.update_review(&id, &folder, &title))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn organize_documents(
    approvals: Vec<Approval>,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<(), String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.organize(approvals))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn organize_proposal(
    files: Vec<suggest::Placement>,
    watch: bool,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<usize, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.organize_proposal(files, watch))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn clear_batch(engine: tauri::State<Arc<Engine>>) {
    engine.clear();
}
#[tauri::command]
fn skip_document(id: String, engine: tauri::State<Arc<Engine>>) -> Result<(), String> {
    engine.skip(&id)
}
#[tauri::command]
async fn preview_document(
    id: String,
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<(), String> {
    let path = engine.preview_path(&id)?;
    let (sender, receiver) = tauri::async_runtime::channel(1);
    app.run_on_main_thread(move || {
        let _ = sender.try_send(native::preview(&path));
    })
    .map_err(|e| e.to_string())?;
    let mut receiver = receiver;
    receiver.recv().await.ok_or("Preview unavailable")?
}
#[tauri::command]
async fn check_model(app: tauri::AppHandle, model: model::LocalModel) -> Result<String, String> {
    let helper = bundled(&app, "tuck-apple")?;
    tauri::async_runtime::spawn_blocking(move || {
        use crate::provider::ModelProvider;
        let _serial = model_lock::hold();
        provider::selected(&helper, model).status()
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn check_connection(
    app: tauri::AppHandle,
    connection: model::ModelConnection,
    api_key: Option<String>,
    list_models: bool,
) -> Result<String, String> {
    let helper = bundled(&app, "tuck-apple")?;
    tauri::async_runtime::spawn_blocking(move || {
        files::validate_connection(&connection)?;
        let _serial = model_lock::hold();
        let response = helper::invoke(&helper, Some(serde_json::json!({"model":"custom", "operation":if list_models { "models" } else { "status" }, "connection":connection, "apiKey":api_key})), std::time::Duration::from_secs(90))?;
        if list_models { serde_json::to_string(&response["models"]).map_err(|e| e.to_string()) }
        else { response["status"].as_str().map(str::to_owned).ok_or("Invalid connection response".into()) }
    })
    .await
    .map_err(|e| e.to_string())?
}
/// A Swift binary shipped in `resources/`: from the source tree in debug builds, from the
/// app bundle otherwise.
fn bundled(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    let resources = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    } else {
        app.path().resource_dir().map_err(|e| e.to_string())?
    };
    Ok(resources.join("resources").join(name))
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
            let storage = app.path().app_data_dir()?;
            let reader = bundled(app.handle(), "tuck-reader").map_err(std::io::Error::other)?;
            let engine = Arc::new(Engine::new(storage, reader).map_err(std::io::Error::other)?);
            app.manage(engine.clone());
            Engine::start(engine);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            map_folder,
            start_tuck,
            stop_tuck,
            choose_folder,
            save_preferences,
            import_documents,
            suggest_categories,
            update_review,
            organize_documents,
            organize_proposal,
            clear_batch,
            skip_document,
            check_model,
            check_connection,
            preview_document
        ])
        .run(tauri::generate_context!())
        .expect("Tuck could not start");
}
