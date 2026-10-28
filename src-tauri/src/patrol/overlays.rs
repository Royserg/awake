//! Main-thread-only window effects. The controller registers callers before windows become visible.
use super::{
    controller::Controller,
    model::*,
    platform::{self, NativeDisplay},
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
fn failed(e: impl std::fmt::Display) -> Failure {
    Failure::new(ErrorCode::OverlayFailed, e.to_string())
}
#[derive(Clone)]
pub struct Planned {
    pub label: String,
    pub display: NativeDisplay,
}
pub fn plan(
    controller: &mut Controller,
    native: &[(Id, NativeDisplay)],
) -> Result<Vec<Planned>, Failure> {
    let cursor = platform::cursor().ok();
    let primary = native
        .iter()
        .find(|(_, display)| display.primary)
        .map(|(id, _)| *id);
    let control = control_display(&controller.displays, cursor, primary);
    let mut plans = vec![];
    for (id, display) in native {
        let display_dto = controller
            .displays
            .iter()
            .find(|d| d.display_id == *id)
            .ok_or_else(|| failed("Display is no longer current"))?;
        if display_dto.mirror_of.is_some() {
            continue;
        }
        let mapping = Mapping {
            origin: XY {
                x: display.bounds.x,
                y: display.bounds.y,
            },
            width: display.bounds.width,
            height: display.bounds.height,
        };
        let registration = controller.register(*id, mapping, control == Some(*id))?;
        plans.push(Planned {
            label: registration.label,
            display: display.clone(),
        });
    }
    Ok(plans)
}
pub fn build(app: &tauri::AppHandle, plans: &[Planned]) -> Result<(), Failure> {
    for plan in plans {
        let builder =
            WebviewWindowBuilder::new(app, &plan.label, WebviewUrl::App("/patrol".into()))
                .title("Cursor Patrolling")
                .decorations(false)
                .resizable(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .visible(false)
                .focused(false)
                .skip_taskbar(true);
        #[cfg(target_os = "macos")]
        let builder = builder
            .accept_first_mouse(true)
            .visible_on_all_workspaces(true);
        builder.build().map_err(failed)?;
    }
    Ok(())
}
pub fn place_and_show(
    app: &tauri::AppHandle,
    controller: &mut Controller,
    plans: &[Planned],
) -> Result<(), Failure> {
    for plan in plans {
        let window = app
            .get_webview_window(&plan.label)
            .ok_or_else(|| failed("A setup window disappeared"))?;
        let mapping = platform::place(&window, &plan.display)?;
        controller.set_mapping(&plan.label, mapping)?;
        window.set_ignore_cursor_events(false).map_err(failed)?;
    }
    show(app, &controller.labels())?;
    if let Some(window) = controller
        .labels()
        .iter()
        .find(|label| {
            controller
                .snapshot(label)
                .caller
                .is_some_and(|c| c.is_control_panel)
        })
        .and_then(|label| app.get_webview_window(label))
    {
        window.set_focus().map_err(failed)?;
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn create(
    app: &tauri::AppHandle,
    controller: &mut Controller,
    native: &[(Id, NativeDisplay)],
) -> Result<(), Failure> {
    let plans = plan(controller, native)?;
    build(app, &plans)?;
    place_and_show(app, controller, &plans)
}
pub fn show(app: &tauri::AppHandle, labels: &[String]) -> Result<(), Failure> {
    for label in labels {
        let window = app
            .get_webview_window(label)
            .ok_or_else(|| failed("A setup window disappeared"))?;
        window.show().map_err(failed)?;
    }
    Ok(())
}
pub fn hide(app: &tauri::AppHandle, labels: &[String]) -> Result<(), Failure> {
    for label in labels {
        let window = app
            .get_webview_window(label)
            .ok_or_else(|| failed("A setup window disappeared"))?;
        window.hide().map_err(failed)?;
    }
    Ok(())
}
/// Missing windows are already cleaned up. A failed destroy keeps the controller in its closing state.
pub fn destroy(app: &tauri::AppHandle, labels: &[String]) -> Result<(), Failure> {
    for label in labels {
        if let Some(window) = app.get_webview_window(label) {
            window.destroy().map_err(failed)?;
        }
    }
    Ok(())
}
pub fn publish(app: &tauri::AppHandle, controller: &Controller) {
    use tauri::Emitter;
    if let Some(setup_id) = controller.setup_id() {
        let snapshot = controller.snapshot("");
        let payload = StateChanged {
            setup_id,
            state_version: snapshot.state_version,
        };
        for label in controller.labels() {
            let _ = app.emit_to(&label, "patrol-state-changed", &payload);
        }
    }
}
