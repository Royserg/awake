use std::sync::Mutex;
use nosleep::{NoSleep, NoSleepType};
use tauri::{menu::{Menu, MenuItem}, tray::{TrayIconBuilder, TrayIconEvent}, Manager, State};

#[tauri::command]
fn activate(state: State<'_, Mutex<AppState>>) -> String {
    let mut state = state.lock().unwrap();
    state.no_sleep.start(NoSleepType::PreventUserIdleDisplaySleep).expect("Failed to start NoSleep");
    format!("activated")
}

#[tauri::command]
fn deactivate(state: State<'_, Mutex<AppState>>) -> String {
    let state = state.lock().unwrap();
    state.no_sleep.stop().expect("Failed to stop NoSleep");
    format!("deactivated")
}


struct AppState {
    no_sleep: NoSleep
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {

            app.manage(Mutex::new(
                AppState {
                    no_sleep: NoSleep::new().unwrap(),
                }
            ));

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_i])?;

            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, e| {
                    match e {
                        TrayIconEvent::Click { id, position, rect, button, button_state } => {
                            // TODO: toggle awake state
                            println!("Tray icon clicked");
                        },
                        _ => {
                            println!("unhandled event {e:?}");
                        }
                    }
                })
                .on_menu_event(|app, e| {
                    match e.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![activate, deactivate])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
