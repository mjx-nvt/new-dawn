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
        pub mod resources;
        pub mod translate;
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
    pub mod csv;
    pub mod records;
    pub mod text;
    pub mod registry;
    pub mod storage;
    pub mod tray;
    pub mod windows;
}

mod modules {
    pub mod agents;
    pub mod brand;
    pub mod cockpit;
    pub mod crm;
    pub mod eml;
    pub mod feedback;
    pub mod inbox;
    pub mod mailbox;
    pub mod api;
    pub mod assistant;
    pub mod workflows;
    pub mod lessons;
    pub mod meetings;
    pub mod memory;
    pub mod model_check;
    pub mod people;
    pub mod projects;
    pub mod switches;
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
    pub mod cockpit;
    pub mod crm;
    pub mod datamodel;
    pub mod files;
    pub mod glass;
    pub mod inbox;
    pub mod mailbox;
    pub mod api;
    pub mod assistant;
    pub mod workflows;
    pub mod layouts;
    pub mod meetings;
    pub mod minutes;
    pub mod models;
    pub mod navigation;
    pub mod dashboards;
    pub mod transfer;
    pub mod onboarding;
    pub mod people;
    pub mod profile;
    pub mod projects;
    pub mod switches;
    pub mod views;
    pub mod recorder;
    pub mod records;
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
            commands::models::get_speech_choices,
            commands::models::set_speech_model,
            commands::models::ai_activity,
            commands::models::get_ai_policy,
            commands::models::set_ai_policy,
            commands::models::run_ai_job_now,
            commands::models::list_model_checks,
            commands::models::start_model_check,
            commands::models::cancel_model_check,
            commands::models::pick_model_check,
            commands::models::list_references,
            commands::models::set_meeting_reference,
            commands::meetings::list_meeting_types,
            commands::meetings::search_meetings,
            commands::meetings::search_by_meaning,
            commands::summary::get_summary,
            commands::summary::summarise_meeting,
            commands::summary::cancel_summary,
            commands::summary::confirm_names,
            commands::summary::rewrite_email,
            commands::summary::get_summary_translation,
            commands::summary::translate_summary,
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
            commands::agents::learn_now,
            commands::agents::approve_lesson,
            commands::agents::dismiss_lesson,
            commands::agents::forget_learned,
            commands::projects::list_projects,
            commands::projects::get_project,
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::delete_project,
            commands::projects::apply_project_update,
            commands::projects::dismiss_project_update,
            commands::projects::find_project_updates,
            commands::crm::list_people_cards,
            commands::crm::list_waiting_commitments,
            commands::crm::get_person,
            commands::crm::set_commitment_status,
            commands::crm::set_person_notes,
            commands::crm::collect_commitments,
            commands::crm::list_unlinked_commitments,
            commands::crm::list_dismissed_commitments,
            commands::inbox::list_emails,
            commands::inbox::read_email_file,
            commands::inbox::read_mail_selection,
            commands::inbox::forget_email,
            commands::inbox::add_email_task,
            commands::inbox::dismiss_email_task,
            commands::mailbox::list_email_threads,
            commands::mailbox::list_email_messages,
            commands::mailbox::mail_settings,
            commands::mailbox::set_mail_settings,
            commands::mailbox::list_blocklist,
            commands::mailbox::block_handle,
            commands::mailbox::unblock_handle,
            commands::mailbox::add_person_from_email,
            commands::mailbox::calendar_events,
            commands::mailbox::mail_running,
            commands::cockpit::cockpit_today,
            commands::switches::get_modules,
            commands::switches::set_modules,
            commands::records::list_objects,
            commands::records::get_object,
            commands::records::list_records,
            commands::records::get_record,
            commands::records::create_record,
            commands::records::update_record,
            commands::records::delete_record,
            commands::records::restore_record,
            commands::records::destroy_record,
            commands::records::search_records,
            commands::views::list_views,
            commands::views::get_view,
            commands::views::view_query,
            commands::views::create_view,
            commands::views::update_view,
            commands::views::delete_view,
            commands::views::duplicate_view,
            commands::views::reorder_views,
            commands::views::save_view,
            commands::views::set_view_fields,
            commands::views::set_view_filters,
            commands::views::set_view_sorts,
            commands::views::set_view_groups,
            commands::views::aggregate_records,
            commands::views::group_counts,
            commands::layouts::get_record_layout,
            commands::layouts::save_record_layout,
            commands::layouts::reset_record_layout,
            commands::layouts::list_timeline,
            commands::layouts::record_neighbours,
            commands::layouts::list_linked,
            commands::layouts::add_linked,
            commands::layouts::list_attachments,
            commands::datamodel::create_object,
            commands::datamodel::update_object,
            commands::datamodel::set_object_active,
            commands::datamodel::delete_object,
            commands::datamodel::create_field,
            commands::datamodel::update_field,
            commands::datamodel::set_field_active,
            commands::datamodel::delete_field,
            commands::files::add_attachment,
            commands::files::add_attachment_bytes,
            commands::files::save_attachment,
            commands::files::attachment_path,
            commands::files::delete_attachment,
            commands::navigation::list_favorites,
            commands::navigation::add_favorite,
            commands::navigation::remove_favorite,
            commands::navigation::move_favorite,
            commands::navigation::create_favorite_folder,
            commands::navigation::rename_favorite_folder,
            commands::navigation::delete_favorite_folder,
            commands::navigation::list_navigation,
            commands::navigation::set_navigation,
            commands::navigation::list_all_views,
            commands::navigation::note_visit,
            commands::navigation::list_recents,
            commands::transfer::read_import_file,
            commands::transfer::validate_import,
            commands::transfer::run_import,
            commands::transfer::export_view,
            commands::transfer::list_transfer_jobs,
            commands::dashboards::dashboard_layout,
            commands::dashboards::save_dashboard_layout,
            commands::dashboards::duplicate_dashboard,
            commands::dashboards::chart_data,
            commands::workflows::list_workflow_versions,
            commands::workflows::workflow_draft,
            commands::workflows::save_workflow_draft,
            commands::workflows::activate_workflow_version,
            commands::workflows::deactivate_workflow_version,
            commands::workflows::discard_workflow_draft,
            commands::workflows::use_workflow_version_as_draft,
            commands::workflows::run_workflow,
            commands::workflows::workflow_run_detail,
            commands::workflows::answer_workflow_form,
            commands::workflows::retry_workflow_run,
            commands::workflows::cancel_workflow_run,
            commands::workflows::list_manual_workflows,
            commands::workflows::pending_workflow_code,
            commands::workflows::answer_workflow_code,
            commands::assistant::list_assistant_agents,
            commands::assistant::save_assistant_agent,
            commands::assistant::delete_assistant_agent,
            commands::assistant::list_chat_threads,
            commands::assistant::new_chat_thread,
            commands::assistant::delete_chat_thread,
            commands::assistant::chat_messages,
            commands::assistant::ask_assistant,
            commands::assistant::apply_chat_proposal,
            commands::assistant::dismiss_chat_proposal,
            commands::api::api_status,
            commands::api::set_api_enabled,
            commands::api::list_api_keys,
            commands::api::create_api_key,
            commands::api::revoke_api_key,
            commands::api::list_webhooks,
            commands::api::save_webhook,
            commands::api::delete_webhook,
            commands::api::webhook_deliveries,
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
            // tauri.conf.json's `$APPDATA` scope is the installed app's folder; a debug build
            // keeps its own (D-044), so its recordings and logos are allowed here.
            #[cfg(debug_assertions)]
            app.asset_protocol_scope()
                .allow_directory(root.join("workspaces"), true)?;
            // ponytail: opens + migrates every active workspace before the window paints; fine for a
            // handful of tenants, move to a background task with a "hub ready" state if startup slows.
            let hub = tauri::async_runtime::block_on(async {
                let hub = WorkspaceHub::open(root).await?;
                hub.open_all_active().await?;
                // Jobs a quit interrupted, resumed once the app's state is in place (D-048).
                let mut interrupted = Vec::new();
                for workspace in hub.list().await?.iter().filter(|w| !w.archived) {
                    // One damaged workspace must not stop the app from starting.
                    let recovered = async {
                        let pool = hub.pool(&workspace.slug)?;
                        recorder::session::recover(&pool).await?;
                        modules::model_check::recover(&pool).await?;
                        modules::meetings::recover_interrupted(&pool).await
                    }
                    .await;
                    match recovered {
                        Ok(jobs) => interrupted.push((workspace.slug.clone(), jobs)),
                        Err(err) => tracing::warn!(
                            "recovery skipped for workspace {}: {err:#}",
                            workspace.slug
                        ),
                    }
                }
                if let Some(model) = hub.app_setting(kernel::ai::ollama::MODEL_SETTING).await? {
                    kernel::ai::ollama::use_model(&model);
                }
                let idle = hub
                    .app_setting(kernel::ai::whisper::IDLE_UNLOAD_SETTING)
                    .await?;
                anyhow::Ok((hub, idle.and_then(|v| v.parse().ok()), interrupted))
            })?;
            let (hub, idle_unload_secs, interrupted) = hub;
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
            app.manage(modules::model_check::ModelChecks::default());
            // Catch-up work waits for a recording to end (D-050).
            let watched = app.handle().clone();
            app.state::<kernel::ai::queue::AiQueue>()
                .set_recording_check(move || {
                    watched
                        .state::<recorder::session::Recorder>()
                        .status()
                        .phase
                        != recorder::session::RecorderPhase::Idle
                });
            kernel::backup::spawn_scheduler(app.handle());
            modules::workflows::triggers::spawn(app.handle());
            modules::api::spawn(app.handle());
            kernel::storage::spawn_scheduler(app.handle());
            modules::lessons::spawn(app.handle());
            modules::memory::spawn(app.handle());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                for (slug, jobs) in interrupted {
                    modules::meetings::resume_interrupted(&handle, &slug, jobs).await;
                }
            });
            #[cfg(debug_assertions)]
            if let Err(err) = kernel::windows::dev_open(app.handle()) {
                tracing::warn!("NEWDAWN_DEV_OPEN ignored: {err:#}");
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    // Not while an AI job holds the queue: a live session keeps Whisper loaded
                    // through its silences (D-041).
                    let queue = handle.state::<kernel::ai::queue::AiQueue>();
                    if let Some(_turn) = queue.inner().try_acquire(
                        kernel::ai::queue::Lane::Background,
                        kernel::ai::queue::Task::new("Freeing memory"),
                    ) {
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
