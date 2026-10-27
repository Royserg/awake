//! Native ownership and deadlines for passive startup visuals. No sleep/timer state lives here.
mod platform;
mod registry;

use platform::show_compact_window;
use registry::{Kind, Receipt, Registry, Request, Retired, Scope, Status, Tuning};
use serde::Serialize;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const EVENT: &str = "startup-presentation";
const VISIBLE: Duration = Duration::from_secs(2);

struct Service {
    inner: Mutex<Registry>,
    wake: Condvar,
}
fn service(app: &tauri::AppHandle) -> Arc<Service> {
    app.state::<Arc<Service>>().inner().clone()
}

pub fn setup(app: &tauri::AppHandle) {
    let state = Arc::new(Service {
        inner: Mutex::new(Registry {
            monitor: monitor(app).ok(),
            ..Registry::default()
        }),
        wake: Condvar::new(),
    });
    app.manage(state.clone());
    let handle = app.clone();
    std::thread::spawn(move || loop {
        let mut registry = state.inner.lock().unwrap();
        if registry.stopped {
            break;
        }
        let now = Instant::now();
        let expired: Vec<_> = [Scope::Production, Scope::Preview]
            .into_iter()
            .filter_map(|scope| {
                registry
                    .slot(scope)
                    .record
                    .as_ref()
                    .filter(|r| r.deadline.is_some_and(|d| d <= now))
                    .map(|r| (scope, r.receipt.run_id.clone()))
            })
            .collect();
        let pending: Vec<_> = [Scope::Production, Scope::Preview]
            .into_iter()
            .filter_map(|scope| {
                registry
                    .slot(scope)
                    .record
                    .as_ref()
                    .filter(|r| {
                        r.awaiting_removal
                            && r.receipt.status == Status::Preparing
                            && r.deadline.is_some_and(|d| d > now)
                    })
                    .map(|r| (scope, r.receipt.clone(), r.request.clone()))
            })
            .collect();
        if !expired.is_empty() || !registry.retired.is_empty() || !pending.is_empty() {
            // Invalidate eligibility before queuing any UI work.
            let mut ended = Vec::new();
            for (scope, token) in expired {
                if let Some(r) = registry
                    .slot_mut(scope)
                    .record
                    .as_mut()
                    .filter(|r| r.receipt.run_id == token)
                {
                    r.deadline = None;
                    r.receipt.status = Status::Ended;
                    ended.push((scope, r.receipt.clone(), r.window.take()));
                }
            }
            let retry_delay = Duration::from_millis(if pending.is_empty() { 100 } else { 10 });
            let retired = std::mem::take(&mut registry.retired);
            drop(registry);
            let app = handle.clone();
            if handle
                .run_on_main_thread(move || {
                    for retired in retired {
                        let _ = retire(&app, retired.window, &retired.token);
                    }
                    for (scope, receipt, window) in ended {
                        if let Some(window) = window {
                            let _ = retire(&app, window, &receipt.run_id);
                        }
                        notify(&app, scope, &receipt);
                    }
                    for (scope, receipt, request) in pending {
                        let state = service(&app);
                        if !state
                            .inner
                            .lock()
                            .unwrap()
                            .can_prepare_replacement(scope, &receipt.run_id)
                            || app.get_webview_window(scope.surface()).is_some()
                        {
                            continue;
                        }
                        if let Some(record) =
                            state.inner.lock().unwrap().slot_mut(scope).record.as_mut()
                        {
                            record.awaiting_removal = false;
                        }
                        if let Err(error) = prepare(&app, scope, receipt, request) {
                            eprintln!("Prepare replacement startup surface: {error}");
                        }
                    }
                })
                .is_err()
            {
                break;
            }
            let registry = state.inner.lock().unwrap();
            drop(state.wake.wait_timeout(registry, retry_delay).unwrap());
        } else {
            let delay = [Scope::Production, Scope::Preview]
                .into_iter()
                .filter_map(|scope| {
                    registry
                        .slot(scope)
                        .record
                        .as_ref()
                        .and_then(|r| r.deadline)
                })
                .min()
                .map(|deadline| deadline.saturating_duration_since(now))
                .unwrap_or(Duration::from_secs(60));
            drop(state.wake.wait_timeout(registry, delay).unwrap());
        }
    });
}

pub fn shutdown(app: &tauri::AppHandle) {
    let state = service(app);
    state.inner.lock().unwrap().stopped = true;
    state.wake.notify_all();
}

fn notify(app: &tauri::AppHandle, scope: Scope, receipt: &Receipt) {
    if let Err(error) = app.emit_to(scope.owner(), EVENT, receipt) {
        eprintln!("Startup event: {error}");
    }
}

fn retire(app: &tauri::AppHandle, window: WebviewWindow, token: &str) -> Result<(), String> {
    if window.label() == "main" {
        let state = service(app);
        let superseded = state
            .inner
            .lock()
            .unwrap()
            .production
            .record
            .as_ref()
            .is_some_and(|r| {
                r.receipt.run_id != token && r.window.is_some() && r.request.kind != Kind::Cinematic
            });
        if superseded {
            return Ok(());
        }
        if let Err(error) = window.hide() {
            state.inner.lock().unwrap().retired.push(Retired {
                window,
                token: token.into(),
            });
            state.wake.notify_all();
            return Err(format!("Compact hide: {error}"));
        }
        return Ok(());
    }
    // A delayed cleanup must never address a replacement by label.
    if let Some(current) = app.get_webview_window(window.label()) {
        let owned = current.url().ok().is_some_and(|url| {
            url.query_pairs()
                .any(|(key, value)| key == "runId" && value == token)
        });
        if !owned {
            return Ok(());
        }
    } else {
        return Ok(());
    }
    let hidden = window.hide();
    if let Err(error) = window.destroy() {
        eprintln!("Startup surface cleanup: {error}; hide result: {hidden:?}");
        let state = service(app);
        state.inner.lock().unwrap().retired.push(Retired {
            window,
            token: token.into(),
        });
        state.wake.notify_all();
        return Err(format!("Startup surface cleanup: {error}"));
    }
    Ok(())
}

/// Main-thread internal operation; IPC wrappers validate the actual caller separately.
fn end(
    app: &tauri::AppHandle,
    scope: Scope,
    token: Option<&str>,
    barrier: Option<u64>,
    cinematic_only: Option<bool>,
) -> Result<(), String> {
    let state = service(app);
    let Some(result) = state
        .inner
        .lock()
        .unwrap()
        .end(scope, token, barrier, cinematic_only)
    else {
        return Ok(());
    };
    let cleanup = if let Some(window) = result.1 {
        retire(app, window, &result.0.run_id)
    } else {
        Ok(())
    };
    notify(app, scope, &result.0);
    state.wake.notify_all();
    cleanup
}

pub fn dismiss_intro(app: &tauri::AppHandle) {
    let _ = end(app, Scope::Production, None, None, Some(true));
}
pub fn hide_main(app: &tauri::AppHandle) {
    let _ = end(app, Scope::Production, None, None, Some(false));
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

pub fn allowed_app_caller(window: &WebviewWindow) -> Result<(), String> {
    if matches!(window.label(), "main" | "timer") {
        Ok(())
    } else {
        Err("Caller cannot access Awake state".into())
    }
}
fn require(window: &WebviewWindow, label: &str) -> Result<(), String> {
    if window.label() == label {
        Ok(())
    } else {
        Err("Invalid presentation caller".into())
    }
}

async fn on_main<T: Send + 'static>(
    app: tauri::AppHandle,
    operation: impl FnOnce(tauri::AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(operation(handle));
    })
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rx.recv().map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())?
}

fn monitor(app: &tauri::AppHandle) -> Result<tauri::Monitor, String> {
    let selected = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten());
    selected
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or_else(|| "No monitor available".into())
}

fn begin(
    app: &tauri::AppHandle,
    scope: Scope,
    revision: u64,
    request: Request,
) -> Result<Receipt, String> {
    let state = service(app);
    let (receipt, previous, fresh) =
        state
            .inner
            .lock()
            .unwrap()
            .begin(scope, revision, request.clone())?;
    if !fresh {
        return Ok(receipt);
    }
    if let Some(old) = previous {
        if let Some(window) = old.window {
            // A compact state change reuses main without a hide/show flash.
            if !(window.label() == "main" && request.kind != Kind::Cinematic) {
                retire(app, window, &old.receipt.run_id)?;
            }
        }
    }
    // destroy() is queued even on the UI thread. A fixed label cannot be reused
    // until Tauri has removed the old webview from its manager.
    if !(scope == Scope::Production && request.kind != Kind::Cinematic)
        && app.get_webview_window(scope.surface()).is_some()
    {
        if let Some(record) = state.inner.lock().unwrap().slot_mut(scope).record.as_mut() {
            record.awaiting_removal = true;
        }
        state.wake.notify_all();
        return Ok(receipt);
    }
    prepare(app, scope, receipt, request)
}

fn prepare(
    app: &tauri::AppHandle,
    scope: Scope,
    receipt: Receipt,
    request: Request,
) -> Result<Receipt, String> {
    let state = service(app);
    let created: Result<WebviewWindow, String> = (|| {
        if scope == Scope::Production && request.kind != Kind::Cinematic {
            let window = app
                .get_webview_window("main")
                .ok_or("Missing update owner")?;
            if !state.inner.lock().unwrap().compact_positioned {
                let launch_monitor = state.inner.lock().unwrap().monitor.clone();
                if let Some(monitor) = launch_monitor {
                    let scale = monitor.scale_factor();
                    let size = (300. * scale).round() as i32;
                    window
                        .set_position(PhysicalPosition::new(
                            monitor.position().x + (monitor.size().width as i32 - size) / 2,
                            monitor.position().y + (monitor.size().height as i32 - size) / 2,
                        ))
                        .map_err(|e| e.to_string())?;
                }
            }
            show_compact_window(&window)?;
            state.inner.lock().unwrap().compact_positioned = true;
            return Ok(window);
        }
        let cinematic = request.kind == Kind::Cinematic;
        let selected = state
            .inner
            .lock()
            .unwrap()
            .monitor
            .clone()
            .ok_or("No launch monitor available")?;
        #[allow(unused_mut)]
        let mut builder = WebviewWindowBuilder::new(
            app,
            scope.surface(),
            WebviewUrl::App(format!("/intro?runId={}", receipt.run_id).into()),
        )
        .title("Awake")
        .decorations(false)
        .resizable(false)
        .visible(false)
        .focused(false)
        .focusable(!cinematic)
        .skip_taskbar(true)
        .shadow(false)
        .always_on_top(cinematic)
        .transparent(cinematic)
        .background_color(tauri::webview::Color(
            0,
            0,
            0,
            if cinematic { 0 } else { 255 },
        ))
        .inner_size(300., 300.);
        #[cfg(windows)]
        {
            if cinematic {
                builder = builder.no_redirection_bitmap(true);
            }
        }
        let window = builder.build().map_err(|e| e.to_string())?;
        // Retain the actual handle before subsequent fallible geometry operations.
        if let Some(record) = state.inner.lock().unwrap().slot_mut(scope).record.as_mut() {
            record.window = Some(window.clone());
        }
        if cinematic {
            window
                .set_ignore_cursor_events(true)
                .map_err(|e| e.to_string())?;
            window
                .set_position(*selected.position())
                .map_err(|e| e.to_string())?;
            window
                .set_size(*selected.size())
                .map_err(|e| e.to_string())?;
        } else {
            let size = (300. * selected.scale_factor()).round() as i32;
            window
                .set_position(PhysicalPosition::new(
                    selected.position().x + (selected.size().width as i32 - size) / 2,
                    selected.position().y + (selected.size().height as i32 - size) / 2,
                ))
                .map_err(|e| e.to_string())?;
        }
        let handle = app.clone();
        let token = receipt.run_id.clone();
        let expected = *selected.size();
        window.on_window_event(move |event| {
            let changed = matches!(
                event,
                tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
            ) || (cinematic
                && matches!(event, tauri::WindowEvent::ScaleFactorChanged { .. }))
                || (cinematic
                    && matches!(event, tauri::WindowEvent::Resized(size) if *size != expected));
            if changed {
                let _ = end(&handle, scope, Some(&token), None, None);
            }
        });
        Ok(window)
    })();
    match created {
        Ok(window) => {
            let mut registry = state.inner.lock().unwrap();
            let record = registry.slot_mut(scope).record.as_mut().unwrap();
            if record.receipt.run_id != receipt.run_id || record.receipt.status != Status::Preparing
            {
                drop(registry);
                let _ = retire(app, window, &receipt.run_id);
                return Err("Presentation cancelled during creation".into());
            }
            record.window = Some(window);
            if scope == Scope::Production && request.kind != Kind::Cinematic {
                record.receipt.status = Status::Active;
                record.shown_at = Some(Instant::now());
                record.deadline = None;
            }
            let result = record.receipt.clone();
            drop(registry);
            state.wake.notify_all();
            notify(app, scope, &result);
            Ok(result)
        }
        Err(error) => {
            let window = {
                let mut registry = state.inner.lock().unwrap();
                let record = registry.slot_mut(scope).record.as_mut().unwrap();
                record.receipt.status = Status::Failed;
                record.receipt.error = Some(error.clone());
                record.deadline = None;
                record.window.take()
            };
            if let Some(window) = window {
                let _ = retire(app, window, &receipt.run_id);
            }
            let receipt = state
                .inner
                .lock()
                .unwrap()
                .slot(scope)
                .record
                .as_ref()
                .unwrap()
                .receipt
                .clone();
            notify(app, scope, &receipt);
            state.wake.notify_all();
            Err(error)
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceConfig {
    run_id: String,
    mode: Kind,
    tuning: Tuning,
    reduced_motion: bool,
    preview: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ready {
    shown: bool,
    elapsed_ms: f64,
    remaining_ms: f64,
}

fn surface_scope(window: &WebviewWindow) -> Result<Scope, String> {
    match window.label() {
        "startup-intro" => Ok(Scope::Production),
        "startup-preview-surface" if cfg!(debug_assertions) => Ok(Scope::Preview),
        _ => Err("Not a passive surface".into()),
    }
}

fn ready(app: &tauri::AppHandle, scope: Scope, token: &str) -> Result<Ready, String> {
    let state = service(app);
    let (window, kind, shown) = {
        let registry = state.inner.lock().unwrap();
        if !registry.matches(scope, token) {
            return Ok(Ready {
                shown: false,
                elapsed_ms: 0.,
                remaining_ms: 0.,
            });
        }
        let r = registry.slot(scope).record.as_ref().unwrap();
        (
            r.window.clone().ok_or("Surface not prepared")?,
            r.request.kind,
            r.shown_at,
        )
    };
    if shown.is_none() {
        if kind == Kind::Cinematic {
            window.show().map_err(|e| e.to_string())?;
        } else {
            show_compact_window(&window)?;
        }
    }
    let result = {
        let mut registry = state.inner.lock().unwrap();
        if !registry.matches(scope, token) {
            drop(registry);
            let _ = window.hide();
            return Ok(Ready {
                shown: false,
                elapsed_ms: 0.,
                remaining_ms: 0.,
            });
        }
        let record = registry.slot_mut(scope).record.as_mut().unwrap();
        let started = *record.shown_at.get_or_insert_with(Instant::now);
        record.receipt.status = Status::Active;
        if shown.is_none() {
            record.deadline = Some(if scope == Scope::Preview {
                record.lease
            } else {
                started + VISIBLE
            });
        }
        let elapsed = started.elapsed().as_secs_f64() * 1000.;
        (
            Ready {
                shown: true,
                elapsed_ms: elapsed,
                remaining_ms: record
                    .deadline
                    .unwrap()
                    .saturating_duration_since(Instant::now())
                    .as_secs_f64()
                    * 1000.,
            },
            record.receipt.clone(),
        )
    };
    state.wake.notify_all();
    notify(app, scope, &result.1);
    Ok(result.0)
}

#[tauri::command]
pub fn startup_context(app: tauri::AppHandle, window: WebviewWindow) -> Result<u64, String> {
    require(&window, "main")?;
    Ok(service(&app).inner.lock().unwrap().production.revision)
}
#[tauri::command]
pub async fn start_intro(
    app: tauri::AppHandle,
    window: WebviewWindow,
    revision: u64,
    tuning: Tuning,
) -> Result<Receipt, String> {
    require(&window, "main")?;
    on_main(app, move |app| {
        begin(
            &app,
            Scope::Production,
            revision,
            Request {
                command: "start_intro",
                kind: Kind::Cinematic,
                tuning,
                reduced_motion: false,
            },
        )
    })
    .await
}
#[tauri::command]
pub fn intro_config(
    app: tauri::AppHandle,
    window: WebviewWindow,
    run_id: String,
) -> Result<SurfaceConfig, String> {
    let scope = surface_scope(&window)?;
    let state = service(&app);
    let registry = state.inner.lock().unwrap();
    if !registry.matches(scope, &run_id) {
        return Err("Obsolete surface configuration".into());
    }
    let r = registry.slot(scope).record.as_ref().unwrap();
    Ok(SurfaceConfig {
        run_id,
        mode: r.request.kind,
        tuning: r.request.tuning.clone(),
        reduced_motion: r.request.reduced_motion,
        preview: scope == Scope::Preview,
    })
}
#[tauri::command]
pub async fn intro_ready(
    app: tauri::AppHandle,
    window: WebviewWindow,
    run_id: String,
) -> Result<Ready, String> {
    require(&window, "startup-intro")?;
    on_main(app, move |app| ready(&app, Scope::Production, &run_id)).await
}
#[tauri::command]
pub async fn finish_intro(
    app: tauri::AppHandle,
    window: WebviewWindow,
    run_id: Option<String>,
    revision: Option<u64>,
) -> Result<(), String> {
    if window.label() == "main" {
        if revision.is_none() && run_id.is_none() {
            return Err("Missing cancellation identity".into());
        }
    } else {
        require(&window, "startup-intro")?;
        if run_id.is_none() || revision.is_some() {
            return Err("Surface must finish its own token".into());
        }
    }
    on_main(app, move |app| {
        end(
            &app,
            Scope::Production,
            run_id.as_deref(),
            revision,
            Some(true),
        )
    })
    .await
}
#[tauri::command]
pub async fn show_compact(
    app: tauri::AppHandle,
    window: WebviewWindow,
    revision: u64,
    kind: String,
) -> Result<Receipt, String> {
    require(&window, "main")?;
    let kind = match kind.as_str() {
        "intro" => Kind::CompactIntro,
        "wait" => Kind::CompactWait,
        "error" => Kind::CompactError,
        _ => return Err("Invalid compact kind".into()),
    };
    on_main(app, move |app| {
        begin(
            &app,
            Scope::Production,
            revision,
            Request {
                command: "show_compact",
                kind,
                tuning: Tuning::default(),
                reduced_motion: false,
            },
        )
    })
    .await
}
#[tauri::command]
pub async fn hide_compact(
    app: tauri::AppHandle,
    window: WebviewWindow,
    compact_token: Option<String>,
    revision: Option<u64>,
) -> Result<(), String> {
    require(&window, "main")?;
    if compact_token.is_none() && revision.is_none() {
        return Err("Missing cancellation identity".into());
    }
    on_main(app, move |app| {
        end(
            &app,
            Scope::Production,
            compact_token.as_deref(),
            revision,
            Some(false),
        )
    })
    .await
}

#[cfg(debug_assertions)]
pub fn open_preview(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("startup-preview") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = on_main(handle, |app| {
            WebviewWindowBuilder::new(&app, "startup-preview", WebviewUrl::App("/preview".into()))
                .title("Awake · Intro Preview")
                .inner_size(1220., 850.)
                .build()
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        {
            eprintln!("Open intro preview: {error}");
        }
    });
}
pub fn preview_closed(app: &tauri::AppHandle) {
    let _ = end(app, Scope::Preview, None, None, None);
}

#[cfg(debug_assertions)]
#[tauri::command]
pub fn preview_context(app: tauri::AppHandle, window: WebviewWindow) -> Result<u64, String> {
    require(&window, "startup-preview")?;
    Ok(service(&app).inner.lock().unwrap().preview.revision)
}
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn preview_present(
    app: tauri::AppHandle,
    window: WebviewWindow,
    revision: u64,
    kind: Kind,
    tuning: Tuning,
    reduced_motion: bool,
) -> Result<Receipt, String> {
    require(&window, "startup-preview")?;
    on_main(app, move |app| {
        begin(
            &app,
            Scope::Preview,
            revision,
            Request {
                command: "preview_present",
                kind,
                tuning,
                reduced_motion,
            },
        )
    })
    .await
}
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn preview_ready(
    app: tauri::AppHandle,
    window: WebviewWindow,
    run_id: String,
) -> Result<Ready, String> {
    require(&window, "startup-preview-surface")?;
    on_main(app, move |app| ready(&app, Scope::Preview, &run_id)).await
}
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn preview_finish(
    app: tauri::AppHandle,
    window: WebviewWindow,
    run_id: Option<String>,
    revision: Option<u64>,
) -> Result<(), String> {
    if window.label() == "startup-preview" {
        if run_id.is_none() && revision.is_none() {
            return Err("Missing cancellation identity".into());
        }
    } else {
        require(&window, "startup-preview-surface")?;
        if run_id.is_none() || revision.is_some() {
            return Err("Surface must finish its own token".into());
        }
    }
    on_main(app, move |app| {
        end(&app, Scope::Preview, run_id.as_deref(), revision, None)
    })
    .await
}
#[cfg(debug_assertions)]
#[tauri::command]
pub fn preview_control(
    app: tauri::AppHandle,
    window: WebviewWindow,
    run_id: String,
    control: serde_json::Value,
) -> Result<(), String> {
    require(&window, "startup-preview")?;
    let state = service(&app);
    if !state.inner.lock().unwrap().matches(Scope::Preview, &run_id) {
        return Err("Obsolete preview run".into());
    }
    let action = control
        .get("action")
        .and_then(|a| a.as_str())
        .ok_or("Missing preview action")?;
    if !matches!(action, "pause" | "resume" | "seek" | "tune" | "status") {
        return Err("Invalid preview action".into());
    }
    if let Some(tuning) = control.get("tuning") {
        serde_json::from_value::<Tuning>(tuning.clone())
            .map_err(|e| e.to_string())?
            .validate()?;
    }
    if action == "seek"
        && !control
            .get("elapsedMs")
            .and_then(|v| v.as_f64())
            .is_some_and(|v| v.is_finite() && (0. ..=600_000.).contains(&v))
    {
        return Err("Invalid preview time".into());
    }
    app.emit_to(
        "startup-preview-surface",
        "startup-preview-control",
        serde_json::json!({ "runId": run_id, "control": control }),
    )
    .map_err(|e| e.to_string())
}
#[cfg(debug_assertions)]
#[tauri::command]
pub fn preview_sample(
    app: tauri::AppHandle,
    window: WebviewWindow,
    run_id: String,
    elapsed_ms: f64,
    paused: bool,
) -> Result<(), String> {
    require(&window, "startup-preview-surface")?;
    if !elapsed_ms.is_finite() || elapsed_ms < 0. {
        return Err("Invalid preview sample".into());
    }
    let state = service(&app);
    {
        let mut registry = state.inner.lock().unwrap();
        if !registry.matches(Scope::Preview, &run_id) {
            return Ok(());
        }
        let record = registry.preview.record.as_mut().unwrap();
        if record
            .last_sample
            .is_some_and(|last| last.elapsed() < Duration::from_millis(100))
        {
            return Ok(());
        }
        record.last_sample = Some(Instant::now());
    }
    app.emit_to(
        "startup-preview",
        "startup-preview-sample",
        serde_json::json!({ "runId": run_id, "elapsedMs": elapsed_ms, "paused": paused }),
    )
    .map_err(|e| e.to_string())
}
