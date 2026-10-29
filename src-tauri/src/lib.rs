use tauri::Manager;
use tauri_specta::{Builder, collect_commands, collect_events};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use error::AppError;
use kernel::hub::WorkspaceHub;

mod error;
mod events;

mod kernel {
    pub mod ai {
        pub mod days;
        pub mod live;
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
        pub mod rebrand;
    }
    pub mod backup;
    pub mod brand {
        pub mod logo;
        pub mod tokens;
    }
    pub mod glass;
    pub mod hub;
    pub mod menu;
    pub mod notify;
    pub mod permissions;
    pub mod profile;
    pub mod registry;
    pub mod storage;
    pub mod tray;
    pub mod windows;
}

mod modules {
    pub mod agents;
    pub mod brand;
    pub mod meetings;
    pub mod people;
    pub mod tasks;
    pub mod vocabulary;
}

mod recorder {
    pub mod mic;
    pub mod mixer;
    pub mod saver;
    pub mod session;
    pub mod system_tap;
}

mod commands {
    pub mod agents;
    pub mod backup;
    pub mod brand;
    pub mod glass;
    pub mod meetings;
    pub mod minutes;
    pub mod models;
    pub mod onboarding;
    pub mod people;
    pub mod profile;
    pub mod recorder;
    pub mod storage;
    pub mod summary;
    pub mod tasks;
    pub mod templates;
    pub mod vocabulary;
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
            commands::summary::confirm_names,
            commands::summary::rewrite_email,
            commands::summary::apply_corrections,
            commands::summary::dismiss_names,
            commands::summary::update_summary_section,
            commands::summary::copy_summary_email,
            commands::summary::open_summary_in_mail,
            commands::summary::ollama_status,
            commands::summary::open_ollama_app,
            commands::summary::pull_ollama_model,
            commands::summary::remove_ollama_model,
            commands::summary::select_ollama_model,
            commands::summary::unload_ollama_model,
            commands::templates::list_meeting_type_templates,
            commands::templates::create_meeting_type,
            commands::templates::update_meeting_type,
            commands::templates::set_default_meeting_type,
            commands::templates::delete_meeting_type,
            commands::templates::check_minutes_template,
            commands::templates::replace_type_template,
            commands::templates::reset_type_template,
            commands::templates::open_type_template,
            commands::templates::minutes_template_fields,
            commands::templates::generate_minutes_template,
            commands::templates::preview_minutes_template,
            commands::meetings::set_meeting_type,
            commands::brand::get_brand,
            commands::brand::update_brand,
            commands::brand::set_brand_logo,
            commands::brand::reset_brand,
            commands::brand::translate_slogan,
            commands::storage::get_storage_report,
            commands::storage::set_retention_days,
            commands::storage::compress_recordings,
            commands::storage::trash_old_recordings,
            commands::summary::update_attendees,
            commands::tasks::list_tasks,
            commands::tasks::add_task,
            commands::tasks::update_task,
            commands::tasks::delete_task,
            commands::tasks::add_action_item_to_tasks,
            commands::tasks::open_task_count,
            commands::agents::list_agents,
            commands::agents::set_agent_guide,
            commands::agents::reset_agent_guide,
            commands::agents::set_agent_model,
            commands::people::list_people,
            commands::vocabulary::list_vocabulary,
            commands::vocabulary::save_vocabulary_term,
            commands::vocabulary::delete_vocabulary_term,
            commands::vocabulary::whisper_prompt_preview,
            commands::vocabulary::restore_transcript_segment,
            commands::people::save_person,
            commands::people::delete_person,
            commands::people::merge_people,
            commands::backup::get_backup_status,
            commands::backup::set_backup_folder,
            commands::backup::backup_now,
            commands::onboarding::get_onboarding_done,
            commands::onboarding::set_onboarding_done,
            commands::profile::get_profile,
            commands::profile::set_profile,
            commands::onboarding::test_notification,
            commands::minutes::export_minutes_docx,
            commands::minutes::export_minutes_pdf,
            commands::minutes::list_minutes_exports,
            commands::minutes::list_converters,
            commands::minutes::open_minutes_file,
            commands::minutes::delete_minutes_export,
            commands::meetings::update_transcript_segment,
            commands::meetings::set_meeting_notes,
            commands::meetings::rename_meeting,
            commands::meetings::delete_meeting,
            commands::recorder::recorder_status,
            commands::recorder::start_recording,
            commands::recorder::set_recording_paused,
            commands::recorder::stop_recording,
            commands::recorder::live_lines,
            commands::recorder::copy_live_text,
            commands::recorder::last_quick,
            commands::recorder::microphone_permission,
            commands::recorder::request_microphone_permission,
            commands::recorder::open_privacy_settings,
            commands::meetings::list_meetings,
            commands::meetings::get_meeting,
            commands::meetings::get_transcript,
            commands::meetings::get_waveform,
            commands::meetings::set_meeting_format,
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
            events::OllamaPullProgress,
            events::OllamaPullFinished,
            events::BrandChanged,
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

/// Logs to the terminal and to daily files in `~/Library/Logs/co.majorisdigital.newdawn/` (`.dev` in a debug build)
/// (7 kept) — a released app has no terminal, so the file is how problems get diagnosed.
/// The guard flushes the file writer; keep it alive for the whole run.
fn init_logging() -> Option<WorkerGuard> {
    let filter = || EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let file = std::env::var_os("HOME").and_then(|home| {
        RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("new-dawn")
            .filename_suffix("log")
            .max_log_files(7)
            .build(kernel::storage::log_dir(std::path::Path::new(&home)))
            .ok()
    });
    let Some(file) = file else {
        tracing_subscriber::fmt().with_env_filter(filter()).init();
        return None;
    };
    let (writer, guard) = tracing_appender::non_blocking(file);
    tracing_subscriber::registry()
        .with(filter())
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(writer),
        )
        .init();
    Some(guard)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_guard = init_logging();

    let builder = specta_builder();
    #[cfg(debug_assertions)]
    export_bindings(&builder);

    let app = tauri::Builder::default()
        // macOS kills a WKWebView content process under memory pressure (Whisper, Qwen and the
        // webview share the RAM); WebKit respawns an empty one and the window stays grey (D-034).
        .on_web_content_process_terminate(|webview| {
            tracing::warn!(
                "web content process of window '{}' was terminated; reloading",
                webview.label()
            );
            if let Err(err) = webview.reload() {
                tracing::error!("could not reload '{}': {err}", webview.label());
            }
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        // OTA updates (D-031): the only non-localhost traffic besides model downloads, and only
        // when the user (or the daily check they can switch off) asks for it.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());
    // Position and size only: restoring VISIBLE would reopen the app with a hidden main window.
    // Not in a debug build: the plugin's file sits in the installed app's data folder (D-044).
    #[cfg(not(debug_assertions))]
    let app = app.plugin(
        tauri_plugin_window_state::Builder::new()
            .with_state_flags(
                tauri_plugin_window_state::StateFlags::POSITION
                    | tauri_plugin_window_state::StateFlags::SIZE,
            )
            .build(),
    );
    app.menu(kernel::menu::build)
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
            // Dropped at Exit (the run loop ends the process without unwinding): flushes the log.
            app.manage(std::sync::Mutex::new(log_guard));
            builder.mount_events(app);
            kernel::tray::build(app.handle())?;

            if let Some(window) = app.get_webview_window("main") {
                let target = window.clone();
                window.run_on_main_thread(move || kernel::glass::apply_shell_glass(&target))?;
            }

            let root = kernel::storage::data_dir(app.handle())?;
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
                if let Some(model) = hub.app_setting(kernel::ai::ollama::MODEL_SETTING).await? {
                    kernel::ai::ollama::use_model(&model);
                }
                let idle = hub
                    .app_setting(kernel::ai::whisper::IDLE_UNLOAD_SETTING)
                    .await?;
                anyhow::Ok((hub, idle.and_then(|v| v.parse().ok())))
            })?;
            let (hub, idle_unload_secs) = hub;
            app.manage(hub);
            app.manage(kernel::ai::models::ModelStore::new(
                kernel::storage::data_dir(app.handle())?.join("models"),
            )?);
            app.manage(kernel::ai::queue::AiQueue::default());
            app.manage(kernel::ai::whisper::WhisperEngine::new(
                idle_unload_secs.unwrap_or(kernel::ai::whisper::DEFAULT_IDLE_UNLOAD_SECS),
            ));
            app.manage(modules::meetings::transcribe::TranscriptionJobs::default());
            app.manage(recorder::session::Recorder::default());
            app.manage(kernel::ai::live::LiveLines::default());
            app.manage(modules::meetings::summarise::SummaryJobs::default());
            kernel::backup::spawn_scheduler(app.handle());
            kernel::storage::spawn_scheduler(app.handle());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    // Not while an AI job holds the queue: a live session keeps Whisper loaded
                    // through its silences (D-041).
                    let queue = handle.state::<kernel::ai::queue::AiQueue>();
                    if let Some(_turn) = queue.inner().try_acquire() {
                        handle
                            .state::<kernel::ai::whisper::WhisperEngine>()
                            .unload_if_idle();
                    }
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
                    if let Some(guard) = app.try_state::<std::sync::Mutex<Option<WorkerGuard>>>() {
                        guard
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .take();
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
