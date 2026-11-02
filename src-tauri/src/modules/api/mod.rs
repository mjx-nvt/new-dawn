//! The local API (D-071, 0.10.14): Twenty's REST API on this Mac — `127.0.0.1` only, off until
//! the owner turns it on, on a port picked once and kept — with API keys, webhooks to this Mac,
//! the workflows' webhook trigger and a docs page.

pub mod docs;
pub mod filter;
pub mod keys;
pub mod plane;
pub mod rest;
pub mod webhooks;

use std::sync::{Mutex, PoisonError};

use anyhow::Result;
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::kernel::hub::WorkspaceHub;

const ENABLED: &str = "api.enabled";
const PORT: &str = "api.port";

#[derive(Debug, Clone, Serialize, Type)]
pub struct ApiStatus {
    pub enabled: bool,
    pub running: bool,
    pub port: u16,
    /// `http://localhost:<port>` while it runs.
    pub address: String,
}

struct Running {
    port: u16,
    stop: tokio::sync::oneshot::Sender<()>,
    served: tauri::async_runtime::JoinHandle<()>,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);
/// Start and stop one at a time (a launch's start and the owner's toggle never cross).
static TURN: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn running_port() -> Option<u16> {
    RUNNING.lock().unwrap_or_else(PoisonError::into_inner).as_ref().map(|r| r.port)
}

/// Serves on 127.0.0.1 — the port kept from last time, or one the system picks (then kept).
pub async fn start(app: &AppHandle) -> Result<u16> {
    let _turn = TURN.lock().await;
    if let Some(port) = running_port() {
        return Ok(port);
    }
    let hub = app.state::<WorkspaceHub>();
    let saved: u16 = hub.app_setting(PORT).await?.and_then(|p| p.parse().ok()).unwrap_or(0);
    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", saved)).await {
        Ok(l) => l,
        // The kept port is taken now: another one, kept from then on.
        Err(_) if saved != 0 => tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?,
        Err(e) => return Err(e.into()),
    };
    let port = listener.local_addr()?.port();
    if port != saved {
        hub.set_app_setting(PORT, &port.to_string()).await?;
    }
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let router = rest::router(app.clone());
    let served = tauri::async_runtime::spawn(async move {
        let serve = axum::serve(listener, router).with_graceful_shutdown(async {
            let _ = stopped.await;
        });
        if let Err(e) = serve.await {
            tracing::warn!("the local API stopped: {e}");
        }
    });
    *RUNNING.lock().unwrap_or_else(PoisonError::into_inner) = Some(Running { port, stop, served });
    tracing::info!("the local API listens on 127.0.0.1:{port}");
    Ok(port)
}

/// Stops serving and waits for the port to close (so turning it on again finds it free).
pub async fn stop() {
    let _turn = TURN.lock().await;
    let running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(r) = running {
        let _ = r.stop.send(());
        let _ = r.served.await;
    }
}

pub async fn status(app: &AppHandle) -> Result<ApiStatus> {
    let hub = app.state::<WorkspaceHub>();
    let enabled = hub.app_setting(ENABLED).await?.as_deref() == Some("1");
    let running = running_port();
    let port = match running {
        Some(p) => p,
        None => hub.app_setting(PORT).await?.and_then(|p| p.parse().ok()).unwrap_or(0),
    };
    Ok(ApiStatus { enabled, running: running.is_some(), port, address: running.map(|p| format!("http://localhost:{p}")).unwrap_or_default() })
}

/// The owner turns the API on or off (kept for the next launch).
pub async fn set_enabled(app: &AppHandle, on: bool) -> Result<ApiStatus> {
    app.state::<WorkspaceHub>().set_app_setting(ENABLED, if on { "1" } else { "" }).await?;
    if on {
        start(app).await?;
    } else {
        stop().await;
    }
    status(app).await
}

/// At launch: the webhooks' listener, and the API when it was on.
pub fn spawn(app: &AppHandle) {
    webhooks::spawn(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let on = app.state::<WorkspaceHub>().app_setting(ENABLED).await.ok().flatten().as_deref() == Some("1");
        if on {
            if let Err(e) = start(&app).await {
                tracing::warn!("the local API didn't start: {e:#}");
            }
        }
    });
}
