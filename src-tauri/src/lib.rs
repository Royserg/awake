use nosleep::{NoSleep, NoSleepType};
use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State, WindowEvent,
};

#[tauri::command]
fn hide_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    // (MacOS) Hide from Dock when window is hidden
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
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
    format!("activated")
}

#[tauri::command]
fn deactivate(state: State<'_, Mutex<AppState>>) -> String {
    let state = state.lock().unwrap();
    state.no_sleep.stop().expect("Failed to stop NoSleep");
    format!("deactivated")
}

struct AppState {
    no_sleep_active: bool,
    no_sleep: NoSleep,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                let _ = window.hide();
                // (MacOS) Hide from Dock when window is closed
                #[cfg(target_os = "macos")]
                window.app_handle().set_activation_policy(tauri::ActivationPolicy::Accessory);
                api.prevent_close();
            }
            _ => {}
        })
        .setup(|app| {
            app.manage(Mutex::new(AppState {
                no_sleep: NoSleep::new().unwrap(),
                no_sleep_active: false,
            }));

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            #[cfg(debug_assertions)]
            let menu = {
                let show_splash_i =
                    MenuItem::with_id(app, "show_splash", "Show Splash", true, None::<&str>)?;
                Menu::with_items(app, &[&show_splash_i, &quit_i])?
            };

            #[cfg(not(debug_assertions))]
            let menu = Menu::with_items(app, &[&quit_i])?;

            // icons
            let off_icon_bytes = include_bytes!("../icons-off/64x64.png");
            let off_icon = Image::from_bytes(off_icon_bytes).unwrap();

            let _tray = TrayIconBuilder::new()
                .icon(off_icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button_state, .. } = e {
                        // TODO: check against buttons state ->
                        // if button_state

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

                            let on_icon_bytes = include_bytes!("../icons-on/64x64.png");
                            let on_icon = Image::from_bytes(on_icon_bytes).unwrap();
                            tray.set_icon(Some(on_icon)).unwrap();

                            state.no_sleep_active = true;
                        } else {
                            // Deactivate
                            state.no_sleep.stop().expect("Failed to stop NoSleep");

                            let off_icon_bytes = include_bytes!("../icons-off/64x64.png");
                            let off_icon = Image::from_bytes(off_icon_bytes).unwrap();
                            tray.set_icon(Some(off_icon)).unwrap();

                            state.no_sleep_active = false;
                        }
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
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hide_window,
            show_splash,
            activate,
            deactivate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
