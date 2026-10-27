//! Target-specific, focus-preserving compact window presentation.
use tauri::WebviewWindow;

pub(super) fn show_compact_window(window: &WebviewWindow) -> Result<(), String> {
    if window.is_visible().map_err(|e| e.to_string())? {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        window.set_focusable(false).map_err(|e| e.to_string())?;
        let shown = window.show().map_err(|e| e.to_string());
        let restored = window.set_focusable(true).map_err(|e| e.to_string());
        if shown.is_err() || restored.is_err() {
            let _ = window.hide();
        }
        shown.and(restored)
    }
    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::*;
        let native = window.gtk_window().map_err(|e| e.to_string())?;
        native.set_focus_on_map(false);
        native.set_accept_focus(true);
        window.show().map_err(|e| e.to_string())
    }
    #[cfg(windows)]
    {
        windows_focus::show(window)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        Err("Unsupported compact window platform".into())
    }
}

#[cfg(windows)]
mod windows_focus {
    use super::*;
    use std::cell::Cell;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetWindowThreadProcessId, IsChild, SetWindowsHookExW, UnhookWindowsHookEx,
        HCBT_ACTIVATE, HCBT_SETFOCUS, HHOOK, WH_CBT,
    };
    thread_local! { static TARGET: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) }; static PENDING: Cell<Option<HHOOK>> = const { Cell::new(None) }; }
    unsafe extern "system" fn callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let target = TARGET.with(Cell::get);
        let candidate = HWND(wparam.0 as *mut std::ffi::c_void);
        if code >= 0
            && !target.0.is_null()
            && ((code == HCBT_ACTIVATE as i32 && candidate == target)
                || (code == HCBT_SETFOCUS as i32
                    && (candidate == target || IsChild(target, candidate).as_bool())))
        {
            return LRESULT(1);
        }
        CallNextHookEx(None, code, wparam, lparam)
    }
    struct Guard(Option<HHOOK>);
    impl Guard {
        fn remove(&mut self) -> Result<(), String> {
            TARGET.with(|target| target.set(HWND(std::ptr::null_mut())));
            if let Some(hook) = self.0.take() {
                if let Err(error) = unsafe { UnhookWindowsHookEx(hook) } {
                    PENDING.with(|pending| pending.set(Some(hook)));
                    return Err(error.to_string());
                }
            }
            Ok(())
        }
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = self.remove();
        }
    }
    pub fn show(window: &WebviewWindow) -> Result<(), String> {
        if !TARGET.with(Cell::get).0.is_null() {
            return Err("Nested compact show".into());
        }
        if let Some(hook) = PENDING.with(Cell::take) {
            if let Err(error) = unsafe { UnhookWindowsHookEx(hook) } {
                PENDING.with(|pending| pending.set(Some(hook)));
                return Err(error.to_string());
            }
        }
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        let thread = unsafe { GetCurrentThreadId() };
        if unsafe { GetWindowThreadProcessId(hwnd, None) } != thread {
            return Err("Compact show must run on its owning UI thread".into());
        }
        let hook = unsafe { SetWindowsHookExW(WH_CBT, Some(callback), None, thread) }
            .map_err(|e| e.to_string())?;
        let mut guard = Guard(Some(hook));
        TARGET.with(|target| target.set(hwnd));
        let shown = window.show().map_err(|e| e.to_string());
        let removed = guard.remove();
        if shown.is_err() || removed.is_err() {
            let _ = window.hide();
        }
        shown.and(removed)
    }
}
