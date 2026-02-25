use nosleep::{NoSleep, NoSleepType};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State, WebviewUrl, WebviewWindowBuilder,
};

#[tauri::command]
fn hide_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    // (MacOS) Hide from Dock when window is hidden
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

#[tauri::command]
fn show_splash(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn activate(state: State<'_, Mutex<AppState>>) -> String {
    let mut state = state.lock().unwrap();
    state
        .no_sleep
        .start(NoSleepType::PreventUserIdleDisplaySleep)
        .expect("Failed to start NoSleep");
    state.no_sleep_active = true;
    state.activated_at = Some(Instant::now());
    format!("activated")
}

#[tauri::command]
fn deactivate(state: State<'_, Mutex<AppState>>) -> String {
    let mut state = state.lock().unwrap();
    state.no_sleep.stop().expect("Failed to stop NoSleep");
    state.no_sleep_active = false;
    state.activated_at = None;
    state.timer_duration = None;
    format!("deactivated")
}

#[tauri::command]
fn set_timer_duration(app: tauri::AppHandle, duration_ms: u64, state: State<'_, Mutex<AppState>>) {
    let mut state = state.lock().unwrap();

    // Always reset activated_at when setting a timer (timer starts from now)
    state.activated_at = Some(Instant::now());

    // Activate no-sleep if not already active
    if !state.no_sleep_active {
        state
            .no_sleep
            .start(NoSleepType::PreventUserIdleDisplaySleep)
            .expect("Failed to start NoSleep");
        state.no_sleep_active = true;

        // Update tray icon to on
        if let Some(tray) = app.tray_by_id("main") {
            let on_icon_bytes = include_bytes!("../icons-on/64x64.png");
            let on_icon = Image::from_bytes(on_icon_bytes).unwrap();
            let _ = tray.set_icon(Some(on_icon));
        }
    }

    state.timer_duration = Some(Duration::from_millis(duration_ms));

    // Close the timer window
    if let Some(window) = app.get_webview_window("timer") {
        let _ = window.destroy();
    }
}

#[tauri::command]
fn cancel_timer(state: State<'_, Mutex<AppState>>) {
    let mut state = state.lock().unwrap();
    state.timer_duration = None;
}

#[tauri::command]
fn get_timer_remaining_ms(state: State<'_, Mutex<AppState>>) -> Option<u64> {
    let state = state.lock().unwrap();
    if let (Some(activated_at), Some(timer_duration)) = (state.activated_at, state.timer_duration) {
        let elapsed = activated_at.elapsed();
        if elapsed < timer_duration {
            Some((timer_duration - elapsed).as_millis() as u64)
        } else {
            None
        }
    } else {
        None
    }
}

#[tauri::command]
fn open_timer_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("timer") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        // Create window if it doesn't exist
        let _ = WebviewWindowBuilder::new(&app, "timer", WebviewUrl::App("/timer".into()))
            .title("Set Timer")
            .inner_size(280.0, 200.0)
            .resizable(false)
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
    let mins = (total_secs + 59) / 60; // Round up to nearest minute

    if mins <= 0 {
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
}

fn update_status_item(menu_items: &MenuItems, state: &AppState) {
    let text = if state.no_sleep_active {
        if let Some(activated_at) = state.activated_at {
            let elapsed = activated_at.elapsed();
            format_duration(elapsed)
        } else {
            "● Awake".to_string()
        }
    } else {
        "○ Off".to_string()
    };

    let _ = menu_items.status.set_text(&text);
}

fn update_timer_item(menu_items: &MenuItems, state: &AppState) {
    let text = if let (Some(activated_at), Some(timer_duration)) =
        (state.activated_at, state.timer_duration)
    {
        let elapsed = activated_at.elapsed();
        format_remaining(elapsed, timer_duration)
    } else {
        "Set Timer...".to_string()
    };

    let _ = menu_items.timer.set_text(&text);
}

#[cfg_attr(mobile, tauri::mobile_entrypoint)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // For timer window, allow it to close normally
                if window.label() == "timer" {
                    // Let it close - don't prevent
                } else {
                    let _ = window.hide();
                    // (MacOS) Hide from Dock when window is closed
                    #[cfg(target_os = "macos")]
                    let _ = window
                        .app_handle()
                        .set_activation_policy(tauri::ActivationPolicy::Accessory);
                    api.prevent_close();
                }
            }
            _ => {}
        })
        .setup(|app| {
            let app_state = AppState::new();
            app.manage(Mutex::new(app_state));

            // Status item (shows duration when active)
            let status_i = MenuItem::with_id(app, "status", "● Awake (0)", false, None::<&str>)?;

            // Timer item (shows remaining time or "Set Timer...")
            let timer_i = MenuItem::with_id(app, "timer", "Set Timer...", true, None::<&str>)?;

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let separator_i = PredefinedMenuItem::separator(app)?;

            #[cfg(debug_assertions)]
            let menu = {
                let show_splash_i =
                    MenuItem::with_id(app, "show_splash", "Show Splash", true, None::<&str>)?;
                Menu::with_items(
                    app,
                    &[
                        &status_i,
                        &separator_i,
                        &timer_i,
                        &separator_i,
                        &show_splash_i,
                        &quit_i,
                    ],
                )?
            };

            #[cfg(not(debug_assertions))]
            let menu = Menu::with_items(
                app,
                &[&status_i, &separator_i, &timer_i, &separator_i, &quit_i],
            )?;

            // Store menu items for later updates
            let menu_items = Arc::new(MenuItems {
                status: status_i,
                timer: timer_i,
            });

            // icons
            let on_icon_bytes = include_bytes!("../icons-on/64x64.png");
            let on_icon = Image::from_bytes(on_icon_bytes).unwrap();

            let menu_items_clone = menu_items.clone();
            let _tray = TrayIconBuilder::with_id("main")
                .icon(on_icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(move |tray, e| {
                    if let TrayIconEvent::Click { button_state, .. } = e {
                        // Proceed only with one of the states
                        // MouseClick triggers 2 events: Up and Down one after another
                        // this prevents immediate switch back
                        if let MouseButtonState::Down = button_state {
                            return;
                        };

                        let handle = tray.app_handle();
                        let state_guard = handle.state::<Mutex<AppState>>();

                        let mut state = state_guard.lock().expect("Failed to lock AppState");

                        let is_no_sleep_active = state.no_sleep_active;

                        if !is_no_sleep_active {
                            // Activate
                            state
                                .no_sleep
                                .start(NoSleepType::PreventUserIdleDisplaySleep)
                                .expect("Failed to start NoSleep");
                            state.no_sleep_active = true;
                            state.activated_at = Some(Instant::now());
                            // Keep the timer_duration if previously set

                            let on_icon_bytes = include_bytes!("../icons-on/64x64.png");
                            let on_icon = Image::from_bytes(on_icon_bytes).unwrap();
                            tray.set_icon(Some(on_icon)).unwrap();
                        } else {
                            // Deactivate
                            state.no_sleep.stop().expect("Failed to stop NoSleep");
                            state.no_sleep_active = false;
                            state.activated_at = None;
                            state.timer_duration = None;

                            let off_icon_bytes = include_bytes!("../icons-off/64x64.png");
                            let off_icon = Image::from_bytes(off_icon_bytes).unwrap();
                            tray.set_icon(Some(off_icon)).unwrap();
                        }

                        // Update status item
                        update_status_item(&menu_items_clone, &state);
                        update_timer_item(&menu_items_clone, &state);
                    }
                })
                .on_menu_event(|app, e| match e.id.as_ref() {
                    #[cfg(debug_assertions)]
                    "show_splash" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "timer" => {
                        if let Some(window) = app.get_webview_window("timer") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        } else {
                            let handle = app.app_handle().to_owned();
                            open_timer_window(handle);
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // Manage menu items so they're accessible from background thread
            app.manage(menu_items);

            // Spawn background task to update duration display and check timer expiration
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(1));

                    let state_guard = app_handle.state::<Mutex<AppState>>();
                    let menu_items_guard = app_handle.state::<Arc<MenuItems>>();

                    let mut state = state_guard.lock().expect("Failed to lock AppState");
                    let menu_items = menu_items_guard.inner().clone();

                    if state.no_sleep_active {
                        // Update duration display
                        update_status_item(&menu_items, &state);
                        update_timer_item(&menu_items, &state);

                        // Check timer expiration
                        if let (Some(activated_at), Some(timer_duration)) =
                            (state.activated_at, state.timer_duration)
                        {
                            if activated_at.elapsed() >= timer_duration {
                                // Timer expired - deactivate
                                let _ = state.no_sleep.stop();
                                state.no_sleep_active = false;
                                state.activated_at = None;
                                state.timer_duration = None;

                                // Update tray icon
                                if let Some(tray) = app_handle.tray_by_id("main") {
                                    let off_icon_bytes = include_bytes!("../icons-off/64x64.png");
                                    let off_icon = Image::from_bytes(off_icon_bytes).unwrap();
                                    let _ = tray.set_icon(Some(off_icon));
                                }

                                // Update status item
                                update_status_item(&menu_items, &state);
                                update_timer_item(&menu_items, &state);
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hide_window,
            show_splash,
            activate,
            deactivate,
            set_timer_duration,
            cancel_timer,
            get_timer_remaining_ms,
            open_timer_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
