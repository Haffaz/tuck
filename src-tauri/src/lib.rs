mod apple;
mod card_filing;
mod card_suggest;
mod cards;
mod categories;
mod classifier;
mod cluster;
mod collection;
mod config;
mod coordinator;
mod engine;
#[cfg(test)]
mod evaluation;
mod files;
mod instructions;
mod model;
mod native;
mod parallel;
mod provider;
mod rule_check;
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
async fn suggest_folder_rule(
    folder: String,
    preferences: Preferences,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<String, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.suggest_rule(&folder, &preferences))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn check_folder_rules(
    preferences: Preferences,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<Vec<rule_check::RuleIssue>, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.check_rules(&preferences))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn suggest_categories(
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<categories::Proposal, String> {
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
async fn review_existing_documents(
    root: String,
    recursive: bool,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<usize, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.review_existing(&root, recursive))
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
fn clear_batch(engine: tauri::State<Arc<Engine>>) {
    engine.clear();
}
#[tauri::command]
fn skip_document(id: String, engine: tauri::State<Arc<Engine>>) -> Result<(), String> {
    engine.skip(&id)
}
#[tauri::command]
async fn retry_document(id: String, engine: tauri::State<'_, Arc<Engine>>) -> Result<(), String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || engine.retry(&id))
        .await
        .map_err(|e| e.to_string())?
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
    let helper = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-apple")
    } else {
        app.path()
            .resource_dir()
            .map_err(|e| e.to_string())?
            .join("resources/tuck-apple")
    };
    tauri::async_runtime::spawn_blocking(move || {
        use crate::provider::ModelProvider;
        let _serial = coordinator::serial();
        provider::selected(&helper.with_file_name("tuck-reader"), model).status()
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
    let helper = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-apple")
    } else {
        app.path()
            .resource_dir()
            .map_err(|e| e.to_string())?
            .join("resources/tuck-apple")
    };
    tauri::async_runtime::spawn_blocking(move || {
        files::validate_connection(&connection)?;
        let _serial = coordinator::serial();
        let response = apple::invoke(&helper, Some(serde_json::json!({"model":"custom", "operation":if list_models { "models" } else { "status" }, "connection":connection, "apiKey":api_key})), std::time::Duration::from_secs(90))?;
        if list_models { serde_json::to_string(&response["models"]).map_err(|e| e.to_string()) }
        else { response["status"].as_str().map(str::to_owned).ok_or("Invalid connection response".into()) }
    })
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
            let storage = app.path().app_data_dir()?;
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
            map_folder,
            start_tuck,
            stop_tuck,
            choose_folder,
            save_preferences,
            import_documents,
            review_existing_documents,
            suggest_categories,
            suggest_folder_rule,
            check_folder_rules,
            update_review,
            organize_documents,
            clear_batch,
            skip_document,
            retry_document,
            check_model,
            check_connection,
            preview_document
        ])
        .run(tauri::generate_context!())
        .expect("Tuck could not start");
}
