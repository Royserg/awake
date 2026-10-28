mod patrol;
mod startup_presentation;

use nosleep::{NoSleep, NoSleepType};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

#[tauri::command]
fn hide_window(app: tauri::AppHandle, window: WebviewWindow) -> Result<(), String> {
    startup_presentation::allowed_app_caller(&window)?;
    startup_presentation::hide_main(&app);
    app.emit_to("main", "startup-main-hidden", ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn show_splash(app: tauri::AppHandle, window: WebviewWindow) -> Result<(), String> {
    startup_presentation::allowed_app_caller(&window)?;
    app.emit_to("main", "startup-compat-show", ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn activate(app: tauri::AppHandle, window: WebviewWindow) -> Result<String, String> {
    startup_presentation::allowed_app_caller(&window)?;
    patrol::awake(app.clone(), patrol::AwakeOp::Activate).await?;
    queue_tray_refresh(&app);
    Ok("activated".to_string())
}

#[tauri::command]
async fn deactivate(app: tauri::AppHandle, window: WebviewWindow) -> Result<String, String> {
    startup_presentation::allowed_app_caller(&window)?;
    patrol::awake(app.clone(), patrol::AwakeOp::Deactivate).await?;
    queue_tray_refresh(&app);
    Ok("deactivated".to_string())
}

#[tauri::command]
async fn set_timer_duration(
    app: tauri::AppHandle,
    duration_ms: u64,
    window: WebviewWindow,
) -> Result<(), String> {
    startup_presentation::allowed_app_caller(&window)?;
    patrol::awake(app.clone(), patrol::AwakeOp::SetTimer(duration_ms)).await?;
    queue_tray_refresh(&app);

    // Close the timer window
    if let Some(window) = app.get_webview_window("timer") {
        let _ = window.destroy();
    }
    Ok(())
}

#[tauri::command]
async fn cancel_timer(app: tauri::AppHandle, window: WebviewWindow) -> Result<(), String> {
    startup_presentation::allowed_app_caller(&window)?;
    patrol::awake(app.clone(), patrol::AwakeOp::CancelTimer).await?;
    queue_tray_refresh(&app);
    Ok(())
}

#[tauri::command]
fn get_timer_remaining_ms(
    window: WebviewWindow,
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<u64>, String> {
    startup_presentation::allowed_app_caller(&window)?;
    let state = state.lock().unwrap();
    Ok(
        if let (Some(activated_at), Some(timer_duration)) =
            (state.activated_at, state.timer_duration)
        {
            let elapsed = activated_at.elapsed();
            if elapsed < timer_duration {
                Some((timer_duration - elapsed).as_millis() as u64)
            } else {
                None
            }
        } else {
            None
        },
    )
}

#[tauri::command]
fn open_timer_window(app: tauri::AppHandle, window: WebviewWindow) -> Result<(), String> {
    startup_presentation::allowed_app_caller(&window)?;
    open_timer_window_internal(app);
    Ok(())
}

fn open_timer_window_internal(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("timer") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        // Create window if it doesn't exist
        let _ = WebviewWindowBuilder::new(&app, "timer", WebviewUrl::App("/timer".into()))
            .title("Set Timer")
            .inner_size(280.0, 200.0)
            .resizable(false)
            .always_on_top(true)
            .focused(true)
            .build();
    }
}

struct AppState {
    no_sleep_active: bool,
    no_sleep: NoSleep,
    activated_at: Option<Instant>,
    timer_duration: Option<Duration>,
}

impl AppState {
    fn new() -> Self {
        let mut no_sleep = NoSleep::new().unwrap();
        no_sleep
            .start(NoSleepType::PreventUserIdleDisplaySleep)
            .expect("Failed to start NoSleep");
        Self {
            no_sleep,
            no_sleep_active: true,
            activated_at: Some(Instant::now()),
            timer_duration: None,
        }
    }

    fn activate(&mut self) -> Result<(), String> {
        if !self.no_sleep_active {
            self.no_sleep
                .start(NoSleepType::PreventUserIdleDisplaySleep)
                .map_err(|error| error.to_string())?;
            self.no_sleep_active = true;
            self.activated_at = Some(Instant::now());
            self.timer_duration = None;
        }
        Ok(())
    }

    fn deactivate(&mut self) -> Result<(), String> {
        if self.no_sleep_active {
            self.no_sleep.stop().map_err(|error| error.to_string())?;
        }
        self.no_sleep_active = false;
        self.activated_at = None;
        self.timer_duration = None;
        Ok(())
    }

    fn set_timer(&mut self, duration: Duration) -> Result<(), String> {
        self.activate()?;
        self.activated_at = Some(Instant::now());
        self.timer_duration = Some(duration);
        Ok(())
    }

    fn expire_timer(&mut self, now: Instant) -> Result<(), String> {
        if self.no_sleep_active {
            if let (Some(started_at), Some(duration)) = (self.activated_at, self.timer_duration) {
                if now.saturating_duration_since(started_at) >= duration {
                    self.deactivate()?;
                }
            }
        }
        Ok(())
    }
}

fn format_duration(elapsed: Duration) -> String {
    let total_secs = elapsed.as_secs();
    let mins = total_secs / 60;
    let hours = mins / 60;
    let mins = mins % 60;

    if hours > 0 {
        format!("● Awake ({}:{:02})", hours, mins)
    } else {
        format!("● Awake ({})", mins)
    }
}

fn format_remaining(elapsed: Duration, timer_duration: Duration) -> String {
    let remaining = timer_duration.saturating_sub(elapsed);
    let total_secs = remaining.as_secs();
    let mins = total_secs.div_ceil(60); // Round up to nearest minute

    if mins == 0 {
        "Timer".to_string()
    } else if mins >= 60 {
        let hours = mins / 60;
        let remaining_mins = mins % 60;
        if remaining_mins == 0 {
            format!("Timer: {}h", hours)
        } else {
            format!("Timer: {}h {}m", hours, remaining_mins)
        }
    } else {
        format!("Timer: {}m", mins)
    }
}

// Store references to menu items that need updating
struct MenuItems {
    status: MenuItem<tauri::Wry>,
    timer: MenuItem<tauri::Wry>,
    patrol: MenuItem<tauri::Wry>,
}

// Track previous display text for change detection
struct DisplayState {
    last_status_text: String,
    last_timer_text: String,
    last_no_sleep_active: Option<bool>,
}

impl DisplayState {
    fn new() -> Self {
        Self {
            last_status_text: String::new(),
            last_timer_text: String::new(),
            last_no_sleep_active: None,
        }
    }
}

// Data extracted from state for UI updates (to avoid holding mutex during UI ops)
struct StateSnapshot {
    no_sleep_active: bool,
    activated_at: Option<Instant>,
    timer_duration: Option<Duration>,
}

fn compute_status_text(state: &StateSnapshot) -> String {
    if state.no_sleep_active {
        if let Some(activated_at) = state.activated_at {
            let elapsed = activated_at.elapsed();
            format_duration(elapsed)
        } else {
            "● Awake".to_string()
        }
    } else {
        "○ Off".to_string()
    }
}

fn compute_timer_text(state: &StateSnapshot) -> String {
    if let (Some(activated_at), Some(timer_duration)) = (state.activated_at, state.timer_duration) {
        let elapsed = activated_at.elapsed();
        format_remaining(elapsed, timer_duration)
    } else {
        "Set Timer".to_string()
    }
}

fn update_status_item_if_changed(
    menu_items: &MenuItems,
    display_state: &mut DisplayState,
    state: &StateSnapshot,
) {
    let text = compute_status_text(state);
    if display_state.last_status_text != text {
        display_state.last_status_text = text.clone();
        let _ = menu_items.status.set_text(&text);
    }
}

fn update_timer_item_if_changed(
    menu_items: &MenuItems,
    display_state: &mut DisplayState,
    state: &StateSnapshot,
) {
    let text = compute_timer_text(state);
    if display_state.last_timer_text != text {
        display_state.last_timer_text = text.clone();
        let _ = menu_items.timer.set_text(&text);
    }
}

// Only call on the main thread. No worker may hold these locks while waiting for UI work.
fn refresh_tray(app: &tauri::AppHandle) {
    let snapshot = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().expect("Failed to lock AppState");
        StateSnapshot {
            no_sleep_active: state.no_sleep_active,
            activated_at: state.activated_at,
            timer_duration: state.timer_duration,
        }
    };
    let menu_items = app.state::<Arc<MenuItems>>();
    let display_state = app.state::<Arc<Mutex<DisplayState>>>();
    let mut display = display_state.lock().expect("Failed to lock DisplayState");
    update_status_item_if_changed(&menu_items, &mut display, &snapshot);
    update_timer_item_if_changed(&menu_items, &mut display, &snapshot);
    let (patrol_text, enabled) = patrol::menu_text(app);
    let _ = menu_items.patrol.set_text(patrol_text);
    let _ = menu_items.patrol.set_enabled(enabled);

    if display.last_no_sleep_active != Some(snapshot.no_sleep_active) {
        if let Some(tray) = app.tray_by_id("main") {
            let icon_bytes: &[u8] = if snapshot.no_sleep_active {
                include_bytes!("../icons-on/64x64.png")
            } else {
                include_bytes!("../icons-off/64x64.png")
            };
            let icon = Image::from_bytes(icon_bytes).expect("Failed to decode tray icon");
            if tray.set_icon(Some(icon)).is_ok() {
                display.last_no_sleep_active = Some(snapshot.no_sleep_active);
            }
        }
    }
}

fn queue_tray_refresh(app: &tauri::AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || refresh_tray(&handle));
}

#[cfg_attr(mobile, tauri::mobile_entrypoint)]
pub fn run() {
    #[cfg(debug_assertions)]
    let handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
        hide_window,
        show_splash,
        activate,
        deactivate,
        set_timer_duration,
        cancel_timer,
        get_timer_remaining_ms,
        open_timer_window,
        patrol::get_patrol_setup,
        patrol::update_patrol_setup,
        patrol::start_patrol,
        patrol::cancel_patrol_setup,
        patrol::stop_patrol,
        patrol::request_patrol_permissions,
        startup_presentation::startup_context,
        startup_presentation::start_intro,
        startup_presentation::intro_config,
        startup_presentation::intro_ready,
        startup_presentation::finish_intro,
        startup_presentation::show_compact,
        startup_presentation::hide_compact,
        startup_presentation::preview_context,
        startup_presentation::preview_present,
        startup_presentation::preview_ready,
        startup_presentation::preview_finish,
        startup_presentation::preview_control,
        startup_presentation::preview_sample
    ];
    #[cfg(not(debug_assertions))]
    let handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
        hide_window,
        show_splash,
        activate,
        deactivate,
        set_timer_duration,
        cancel_timer,
        get_timer_remaining_ms,
        open_timer_window,
        patrol::get_patrol_setup,
        patrol::update_patrol_setup,
        patrol::start_patrol,
        patrol::cancel_patrol_setup,
        patrol::stop_patrol,
        patrol::request_patrol_permissions,
        startup_presentation::startup_context,
        startup_presentation::start_intro,
        startup_presentation::intro_config,
        startup_presentation::intro_ready,
        startup_presentation::finish_intro,
        startup_presentation::show_compact,
        startup_presentation::hide_compact
    ];
    let builder = tauri::Builder::default();
    #[cfg(windows)]
    let builder = builder.device_event_filter(tauri::DeviceEventFilter::Always);
    builder
        .plugin(patrol::process_plugin())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .on_page_load(|window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Started)
                && window.label().starts_with("patrol-")
            {
                patrol::page_loaded(window.app_handle(), window.label());
            }
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed)
                && window.label().starts_with("patrol-")
            {
                patrol::window_destroyed(window.app_handle(), window.label());
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if patrol::window_closed(window.app_handle(), window.label()) {
                    api.prevent_close();
                    return;
                }
                match window.label() {
                    "main" => {
                        startup_presentation::hide_main(window.app_handle());
                        let _ = window.emit("startup-main-hidden", ());
                        api.prevent_close();
                    }
                    "startup-preview" => startup_presentation::preview_closed(window.app_handle()),
                    _ => {}
                }
            }
        })
        .setup(|app| {
            let app_state = AppState::new();
            app.manage(Mutex::new(app_state));
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            startup_presentation::setup(app.handle());
            patrol::setup(app.handle());

            // Status item (shows duration when active)
            let status_i = MenuItem::with_id(app, "status", "● Awake (0)", false, None::<&str>)?;

            // Timer item (shows remaining time or "Set Timer...")
            let timer_i = MenuItem::with_id(app, "timer", "Set Timer", true, None::<&str>)?;

            let patrol_i =
                MenuItem::with_id(app, "patrol", "Cursor Patrolling…", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let separator_i = PredefinedMenuItem::separator(app)?;

            #[cfg(debug_assertions)]
            let menu = {
                let preview_i =
                    MenuItem::with_id(app, "intro_preview", "Intro Preview", true, None::<&str>)?;
                Menu::with_items(
                    app,
                    &[
                        &status_i,
                        &separator_i,
                        &timer_i,
                        &patrol_i,
                        &separator_i,
                        &preview_i,
                        &quit_i,
                    ],
                )?
            };

            #[cfg(not(debug_assertions))]
            let menu = Menu::with_items(
                app,
                &[
                    &status_i,
                    &separator_i,
                    &timer_i,
                    &patrol_i,
                    &separator_i,
                    &quit_i,
                ],
            )?;

            // Store menu items for later updates
            let menu_items = Arc::new(MenuItems {
                status: status_i,
                timer: timer_i,
                patrol: patrol_i,
            });

            // Track display state for change detection
            let display_state = Arc::new(Mutex::new(DisplayState::new()));

            // icons
            let on_icon_bytes = include_bytes!("../icons-on/64x64.png");
            let on_icon = Image::from_bytes(on_icon_bytes).unwrap();

            // Tauri >= 2.12.5 includes tray-icon's macOS 27 click forwarding fix.
            let tray_builder = TrayIconBuilder::with_id("main")
                .icon(on_icon)
                .menu(&menu)
                .show_menu_on_left_click(false);

            app.manage(menu_items);
            app.manage(display_state);

            let _tray = tray_builder
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } = event
                    {
                        use tauri::tray::MouseButton;
                        let app = tray.app_handle();
                        startup_presentation::dismiss_intro(app);
                        match (button, button_state) {
                            (MouseButton::Left, MouseButtonState::Up) => {
                                patrol::queue_awake(app, patrol::AwakeOp::Toggle, None);
                                refresh_tray(app);
                            }
                            (MouseButton::Right, MouseButtonState::Down) => {
                                refresh_tray(app);
                            }
                            _ => {}
                        }
                    }
                })
                .on_menu_event(|app, e| match e.id.as_ref() {
                    #[cfg(debug_assertions)]
                    "intro_preview" => startup_presentation::open_preview(app),
                    "patrol" => patrol::menu(app),
                    "timer" => {
                        if let Some(window) = app.get_webview_window("timer") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        } else {
                            let handle = app.app_handle().to_owned();
                            open_timer_window_internal(handle);
                        }
                    }
                    "quit" => {
                        patrol::quit(app);
                    }
                    _ => {}
                })
                .build(app)?;

            refresh_tray(app.handle());

            // The worker only schedules ticks. State transitions and UI updates run together
            // on the main thread, so timer expiry cannot deadlock a click or paint stale state.
            let app_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(1));
                let handle = app_handle.clone();
                if app_handle
                    .run_on_main_thread(move || {
                        patrol::refresh(&handle);
                        refresh_tray(&handle);
                    })
                    .is_err()
                {
                    break;
                }
            });

            Ok(())
        })
        .invoke_handler(handler)
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = &event {
                if *code == Some(tauri::RESTART_EXIT_CODE) {
                    patrol::invalidate_for_exit(app);
                } else if !patrol::exit_allowed(app) {
                    api.prevent_exit();
                    patrol::request_exit(app, code.unwrap_or(0));
                }
            }
            if matches!(event, tauri::RunEvent::Exit) {
                patrol::invalidate_for_exit(app);
                startup_presentation::shutdown(app);
            }
        });
}

// Exercise the real sleep inhibitor on macOS without launching the UI or updater.
#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn can_reactivate_after_timer_expiry_repeatedly() {
        let mut state = AppState::new();
        let duration = Duration::from_secs(60);

        for _ in 0..3 {
            state.set_timer(duration).unwrap();
            let started_at = state.activated_at.unwrap();
            state
                .expire_timer(started_at + duration - Duration::from_millis(1))
                .unwrap();
            assert!(state.no_sleep_active);

            state.expire_timer(started_at + duration).unwrap();
            assert!(!state.no_sleep_active);
            assert!(state.activated_at.is_none());
            assert!(state.timer_duration.is_none());

            state.activate().unwrap();
            assert!(state.no_sleep_active);
            assert!(state.activated_at.is_some());
            assert!(state.timer_duration.is_none());

            // The previous deadline must not switch a new, untimed session off.
            state
                .expire_timer(started_at + duration + duration)
                .unwrap();
            assert!(state.no_sleep_active);
        }
        state.deactivate().unwrap();
    }

    #[test]
    fn manual_toggle_clears_timer_and_can_restart() {
        let mut state = AppState::new();
        state.set_timer(Duration::from_secs(60)).unwrap();
        state.deactivate().unwrap();
        assert!(!state.no_sleep_active);
        assert!(state.timer_duration.is_none());
        state.deactivate().unwrap();
        state.activate().unwrap();
        let started_at = state.activated_at;
        state.activate().unwrap();
        assert_eq!(state.activated_at, started_at);
        assert!(state.no_sleep_active);
        assert!(state.timer_duration.is_none());
        state.deactivate().unwrap();
    }

    #[test]
    fn timer_can_be_replaced_cancelled_and_started_while_off() {
        let mut state = AppState::new();
        state.deactivate().unwrap();
        state.set_timer(Duration::from_secs(60)).unwrap();
        assert!(state.no_sleep_active);
        state.set_timer(Duration::from_secs(120)).unwrap();
        let started_at = state.activated_at.unwrap();
        state
            .expire_timer(started_at + Duration::from_secs(60))
            .unwrap();
        assert!(state.no_sleep_active);

        state.timer_duration = None;
        state
            .expire_timer(started_at + Duration::from_secs(120))
            .unwrap();
        assert!(state.no_sleep_active);
        assert!(state.timer_duration.is_none());
        state.deactivate().unwrap();
    }
}
