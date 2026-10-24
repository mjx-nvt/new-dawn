use tauri::Manager;
use tauri_plugin_window_state::StateFlags;
use tauri_specta::{Builder, collect_commands, collect_events};
use tracing_subscriber::EnvFilter;

use error::AppError;
use kernel::hub::WorkspaceHub;

mod error;
mod events;

mod kernel {
    pub mod ai {
        pub mod log;
        pub mod models;
        pub mod ollama;
        pub mod queue;
        pub mod whisper;
    }
    pub mod export {
        pub mod docx;
        pub mod model;
        pub mod pdf;
    }
    pub mod glass;
    pub mod hub;
    pub mod menu;
    pub mod permissions;
    pub mod registry;
    pub mod tray;
    pub mod windows;
}

mod modules {
    pub mod meetings;
}

mod recorder {
    pub mod mic;
    pub mod mixer;
    pub mod saver;
    pub mod session;
    pub mod system_tap;
}

mod commands {
    pub mod glass;
    pub mod meetings;
    pub mod minutes;
    pub mod models;
    pub mod recorder;
    pub mod summary;
    pub mod window;
    pub mod workspace;
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
#[specta::specta]
fn greet(name: &str) -> Result<String, AppError> {
    Ok(format!("Hello, {}! You've been greeted from Rust!", name))
}

/// Every command the UI can call. Debug builds regenerate `src/bindings.ts` from this list.
fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            greet,
            commands::glass::glass_status,
            commands::window::open_settings,
            commands::models::list_models,
            commands::models::start_model_download,
            commands::models::cancel_model_download,
            commands::models::delete_model,
            commands::models::reveal_models_folder,
            commands::models::get_whisper_idle_unload,
            commands::models::set_whisper_idle_unload,
            commands::meetings::list_meeting_types,
            commands::meetings::search_meetings,
            commands::summary::get_summary,
            commands::summary::summarise_meeting,
            commands::summary::cancel_summary,
            commands::summary::update_summary_section,
            commands::summary::copy_summary_email,
            commands::summary::open_summary_in_mail,
            commands::summary::ollama_status,
            commands::summary::open_ollama_app,
            commands::minutes::export_minutes_docx,
            commands::minutes::export_minutes_pdf,
            commands::minutes::list_minutes_exports,
            commands::minutes::list_converters,
            commands::minutes::open_minutes_file,
            commands::recorder::recorder_status,
            commands::recorder::start_recording,
            commands::recorder::set_recording_paused,
            commands::recorder::stop_recording,
            commands::recorder::microphone_permission,
            commands::recorder::request_microphone_permission,
            commands::recorder::open_privacy_settings,
            commands::meetings::list_meetings,
            commands::meetings::get_meeting,
            commands::meetings::get_transcript,
            commands::meetings::import_audio,
            commands::meetings::transcribe_meeting,
            commands::meetings::cancel_transcription,
            commands::workspace::list_workspaces,
            commands::workspace::get_current_workspace,
            commands::workspace::set_current_workspace,
            commands::workspace::create_workspace,
            commands::workspace::update_workspace,
            commands::workspace::delete_workspace,
        ])
        .events(collect_events![
            events::WorkspacesChanged,
            events::MenuAction,
            events::OpenSettingsTab,
            events::ModelDownloadProgress,
            events::ModelDownloadFinished,
            events::TranscriptionProgress,
            events::TranscriptionFinished,
            events::RecorderStatusChanged,
            events::RecorderLevel,
            events::SummaryProgress,
            events::SummaryFinished,
        ])
}

fn export_bindings(builder: &Builder<tauri::Wry>) {
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/bindings.ts",
        )
        .expect("failed to export src/bindings.ts");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let builder = specta_builder();
    #[cfg(debug_assertions)]
    export_bindings(&builder);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        // Position and size only: restoring VISIBLE would reopen the app with a hidden main window.
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(StateFlags::POSITION | StateFlags::SIZE)
                .build(),
        )
        .menu(kernel::menu::build)
        .on_menu_event(kernel::menu::on_event)
        .invoke_handler(builder.invoke_handler())
        .on_window_event(|window, event| {
            // Closing the main window hides it; the app (and its tray item) keeps running until Quit.
            if window.label() == kernel::windows::MAIN_LABEL {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            builder.mount_events(app);
            kernel::tray::build(app.handle())?;

            if let Some(window) = app.get_webview_window("main") {
                let target = window.clone();
                window.run_on_main_thread(move || kernel::glass::apply_shell_glass(&target))?;
            }

            let root = app.path().app_data_dir()?;
            // ponytail: opens + migrates every active workspace before the window paints; fine for a
            // handful of tenants, move to a background task with a "hub ready" state if startup slows.
            let hub = tauri::async_runtime::block_on(async {
                let hub = WorkspaceHub::open(root).await?;
                hub.open_all_active().await?;
                for workspace in hub.list().await?.iter().filter(|w| !w.archived) {
                    // One damaged workspace must not stop the app from starting.
                    let recovered = async {
                        let pool = hub.pool(&workspace.slug)?;
                        recorder::session::recover(&pool).await?;
                        modules::meetings::recover_interrupted(&pool).await
                    }
                    .await;
                    if let Err(err) = recovered {
                        tracing::warn!(
                            "recovery skipped for workspace {}: {err:#}",
                            workspace.slug
                        );
                    }
                }
                let idle = hub
                    .app_setting(kernel::ai::whisper::IDLE_UNLOAD_SETTING)
                    .await?;
                anyhow::Ok((hub, idle.and_then(|v| v.parse().ok())))
            })?;
            let (hub, idle_unload_secs) = hub;
            app.manage(hub);
            app.manage(kernel::ai::models::ModelStore::new(
                app.path().app_data_dir()?.join("models"),
            )?);
            app.manage(kernel::ai::queue::AiQueue::default());
            app.manage(kernel::ai::whisper::WhisperEngine::new(
                idle_unload_secs.unwrap_or(kernel::ai::whisper::DEFAULT_IDLE_UNLOAD_SECS),
            ));
            app.manage(modules::meetings::transcribe::TranscriptionJobs::default());
            app.manage(recorder::session::Recorder::default());
            app.manage(modules::meetings::summarise::SummaryJobs::default());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    handle
                        .state::<kernel::ai::whisper::WhisperEngine>()
                        .unload_if_idle();
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            match event {
                tauri::RunEvent::Exit => {
                    if let Some(recorder) = app.try_state::<recorder::session::Recorder>() {
                        recorder.shutdown(); // finalise the WAV before the databases close
                    }
                    if let Some(hub) = app.try_state::<WorkspaceHub>() {
                        tauri::async_runtime::block_on(hub.close_all());
                    }
                }
                // Dock icon clicked while the main window is hidden.
                tauri::RunEvent::Reopen { .. } => kernel::windows::show_main(app),
                _ => {}
            }
        });
}

#[cfg(test)]
mod tests {
    /// `cargo test export_bindings` regenerates src/bindings.ts without launching the app.
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::specta_builder());
    }
}
