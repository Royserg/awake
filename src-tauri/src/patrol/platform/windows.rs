//! Owned Raw Input window. No hooks, event injection, desktop switching, or confinement changes.
use super::*;
use ::windows::{
    core::{s, w, BOOL, PCWSTR, PWSTR},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{
            LibraryLoader::*, Power::*, RemoteDesktop::*, StationsAndDesktops::*, Threading::*,
        },
        UI::{
            HiDpi::*,
            Input::{KeyboardAndMouse::*, *},
            WindowsAndMessaging::*,
        },
    },
};
use std::{
    collections::HashMap,
    ffi::c_void,
    mem::size_of,
    thread::{self, JoinHandle},
    time::Duration,
};
fn failure(message: impl Into<String>) -> Failure {
    Failure::new(ErrorCode::NativeUnavailable, message)
}
fn native_error(error: impl std::fmt::Display) -> Failure {
    failure(error.to_string())
}
fn wide(value: &[u16]) -> String {
    String::from_utf16_lossy(&value[..value.iter().position(|c| *c == 0).unwrap_or(value.len())])
}
type DpiFn = unsafe extern "system" fn(DPI_AWARENESS_CONTEXT) -> DPI_AWARENESS_CONTEXT;
struct Dpi {
    set: DpiFn,
    old: DPI_AWARENESS_CONTEXT,
}
impl Dpi {
    fn enter() -> Result<Self, Failure> {
        unsafe {
            let module = GetModuleHandleW(w!("user32.dll")).map_err(native_error)?;
            let symbol = GetProcAddress(module, s!("SetThreadDpiAwarenessContext"))
                .ok_or_else(|| failure("Cursor patrolling requires per-monitor DPI support"))?;
            // The optional User32 export has the SDK's context(context) ABI; save/restore on this same thread.
            let set = std::mem::transmute::<unsafe extern "system" fn() -> isize, DpiFn>(symbol);
            let old = set(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            if old.0.is_null() {
                return Err(failure("Cannot establish per-monitor cursor coordinates"));
            }
            Ok(Self { set, old })
        }
    }
}
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe { (self.set)(self.old) };
    }
}
fn desktop_name(desktop: HDESK) -> Result<String, Failure> {
    unsafe {
        let mut needed = 0;
        let _ = GetUserObjectInformationW(HANDLE(desktop.0), UOI_NAME, None, 0, Some(&mut needed));
        if needed == 0 || needed > 65536 {
            return Err(failure("Cannot identify the input desktop"));
        }
        let mut name = vec![0u16; needed as usize / 2 + 1];
        GetUserObjectInformationW(
            HANDLE(desktop.0),
            UOI_NAME,
            Some(name.as_mut_ptr().cast()),
            needed,
            Some(&mut needed),
        )
        .map_err(native_error)?;
        Ok(wide(&name))
    }
}
fn desktop_active() -> Result<(), Failure> {
    unsafe {
        let own = GetThreadDesktop(GetCurrentThreadId()).map_err(native_error)?;
        let own_name = desktop_name(own)?;
        let input = OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS)
            .map_err(|_| failure("Unlock and return to Awake's desktop"))?;
        let name = desktop_name(input);
        let _ = CloseDesktop(input);
        if name? != own_name {
            return Err(failure("The active desktop changed"));
        }
        let mut buffer = PWSTR::null();
        let mut length = 0;
        WTSQuerySessionInformationW(
            None,
            WTS_CURRENT_SESSION,
            WTSConnectState,
            &mut buffer,
            &mut length,
        )
        .map_err(native_error)?;
        let active = length >= size_of::<WTS_CONNECTSTATE_CLASS>() as u32
            && !buffer.is_null()
            && *(buffer.0.cast::<WTS_CONNECTSTATE_CLASS>()) == WTSActive;
        WTSFreeMemory(buffer.0.cast());
        if !active {
            return Err(failure("The desktop session is inactive"));
        }
        Ok(())
    }
}
pub fn availability() -> Availability {
    match Dpi::enter().and_then(|_| desktop_active()) {
        Ok(()) => Availability {
            state: AvailabilityState::Ready,
            message: None,
        },
        Err(e) => Availability {
            state: AvailabilityState::NativeUnavailable,
            message: Some(e.message),
        },
    }
}
pub fn cursor() -> Result<XY, Failure> {
    let _dpi = Dpi::enter()?;
    unsafe {
        let mut p = POINT::default();
        GetCursorPos(&mut p).map_err(native_error)?;
        Ok(XY {
            x: p.x as f64,
            y: p.y as f64,
        })
    }
}
pub fn buttons_down() -> bool {
    [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON, VK_XBUTTON1, VK_XBUTTON2]
        .iter()
        .any(|key| unsafe { GetAsyncKeyState(key.0 as i32) } < 0)
}
fn confinement(route: &[XY], origin: Option<XY>) -> Result<(), Failure> {
    unsafe {
        let mut clip = RECT::default();
        GetClipCursor(&mut clip).map_err(native_error)?;
        let rect = Rect {
            x: clip.left as f64,
            y: clip.top as f64,
            width: (i64::from(clip.right) - i64::from(clip.left)) as f64,
            height: (i64::from(clip.bottom) - i64::from(clip.top)) as f64,
        };
        if route
            .iter()
            .chain(origin.iter())
            .any(|p| !rect.contains(*p))
        {
            return Err(Failure::new(ErrorCode::CursorConstrained,"Another application constrained the cursor. Release that constraint and start again"));
        }
        Ok(())
    }
}
fn monitor_dpi(monitor: HMONITOR) -> (u32, u32) {
    unsafe {
        let Ok(module) = LoadLibraryExW(w!("shcore.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32)
        else {
            return (96, 96);
        };
        type Query = unsafe extern "system" fn(
            HMONITOR,
            MONITOR_DPI_TYPE,
            *mut u32,
            *mut u32,
        ) -> ::windows::core::HRESULT;
        let mut x = 96;
        let mut y = 96;
        if let Some(symbol) = GetProcAddress(module, s!("GetDpiForMonitor")) {
            let query = std::mem::transmute::<unsafe extern "system" fn() -> isize, Query>(symbol);
            if query(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y).is_err() {
                x = 96;
                y = 96;
            }
        }
        let _ = FreeLibrary(module);
        (x, y)
    }
}
pub fn displays() -> Result<Vec<NativeDisplay>, Failure> {
    let _dpi = Dpi::enter()?;
    let mut out: Vec<NativeDisplay> = vec![];
    unsafe extern "system" fn enumerate(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let out = unsafe { &mut *(data.0 as *mut Vec<NativeDisplay>) };
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        if !unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.as_bool() {
            return BOOL(0);
        }
        let rect = info.monitorInfo.rcMonitor;
        let device = wide(&info.szDevice);
        let mut physical = DISPLAY_DEVICEW {
            cb: size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        let identified = unsafe {
            EnumDisplayDevicesW(
                PCWSTR(info.szDevice.as_ptr()),
                0,
                &mut physical,
                EDD_GET_DEVICE_INTERFACE_NAME,
            )
        }
        .as_bool();
        let mut mode = DEVMODEW {
            dmSize: size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        let has_mode = unsafe {
            EnumDisplaySettingsW(
                PCWSTR(info.szDevice.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut mode,
            )
        }
        .as_bool();
        let rotation = if has_mode {
            (unsafe { mode.Anonymous1.Anonymous2.dmDisplayOrientation.0 }) as f64 * 90.0
        } else {
            0.0
        };
        let (x, y) = monitor_dpi(monitor);
        out.push(NativeDisplay {
            primary: info.monitorInfo.dwFlags & 1 != 0,
            identity: if identified {
                format!("{}:{}", device, wide(&physical.DeviceID))
            } else {
                format!("{}:{}", device, monitor.0 as usize)
            },
            surface: format!("{}:{}:{}:{}", rect.left, rect.top, rect.right, rect.bottom),
            name: if identified {
                wide(&physical.DeviceString)
            } else {
                device
            },
            bounds: Rect {
                x: rect.left as f64,
                y: rect.top as f64,
                width: (i64::from(rect.right) - i64::from(rect.left)) as f64,
                height: (i64::from(rect.bottom) - i64::from(rect.top)) as f64,
            },
            scale: XY {
                x: x as f64 / 96.0,
                y: y as f64 / 96.0,
            },
            rotation,
            native_id: monitor.0 as usize,
            placement_id: monitor.0 as usize,
        });
        BOOL(1)
    }
    unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(enumerate),
            LPARAM(&mut out as *mut _ as isize),
        )
    }
    .ok()
    .map_err(native_error)?;
    if out.is_empty() {
        return Err(failure("No desktop monitors are available"));
    }
    out.sort_by(|a, b| a.identity.cmp(&b.identity));
    Ok(out)
}
pub fn place(window: &tauri::WebviewWindow, display: &NativeDisplay) -> Result<Mapping, Failure> {
    let _dpi = Dpi::enter()?;
    let hwnd = window.hwnd().map_err(native_error)?;
    let b = display.bounds;
    unsafe {
        SetWindowPos(
            HWND(hwnd.0),
            Some(HWND_TOPMOST),
            b.x as i32,
            b.y as i32,
            b.width as i32,
            b.height as i32,
            SWP_NOACTIVATE,
        )
        .map_err(native_error)?;
        let mut rect = RECT::default();
        GetClientRect(HWND(hwnd.0), &mut rect).map_err(native_error)?;
        let mut origin = POINT {
            x: rect.left,
            y: rect.top,
        };
        ClientToScreen(HWND(hwnd.0), &mut origin)
            .ok()
            .map_err(native_error)?;
        let mapping = Mapping {
            origin: XY {
                x: origin.x as f64,
                y: origin.y as f64,
            },
            width: (rect.right - rect.left) as f64,
            height: (rect.bottom - rect.top) as f64,
        };
        if mapping.rect() != b {
            return Err(failure(
                "The setup window does not cover the assigned monitor",
            ));
        }
        Ok(mapping)
    }
}
struct Shared {
    router: Arc<InputRouter>,
    stop: AtomicBool,
    state: Mutex<SessionState>,
    route: Mutex<Vec<XY>>,
    cleanup_failure: Mutex<Option<Failure>>,
}
struct Session {
    shared: Arc<Shared>,
    join: Mutex<Option<JoinHandle<()>>>,
}
struct Context {
    shared: Arc<Shared>,
    absolute: HashMap<usize, (i32, i32)>,
}
fn registered() -> Result<Vec<RAWINPUTDEVICE>, Failure> {
    read_raw_input_registrations(|buffer: Option<&mut [RAWINPUTDEVICE]>| unsafe {
        let probe = buffer.is_none();
        let mut count = buffer.as_ref().map_or(0, |buffer| buffer.len() as u32);
        let pointer = buffer.map(|buffer| buffer.as_mut_ptr());
        let copied =
            GetRegisteredRawInputDevices(pointer, &mut count, size_of::<RAWINPUTDEVICE>() as u32);
        // The null size probe normally returns -1/ERROR_INSUFFICIENT_BUFFER. Capture a real
        // failure immediately; GetLastError after a successful call may contain an unrelated error.
        let error = (copied == u32::MAX).then(|| GetLastError());
        registration_read(
            copied,
            count,
            probe,
            error == Some(ERROR_INSUFFICIENT_BUFFER),
        )
        .ok_or_else(|| {
            failure(format!(
                "Cannot query Awake's Raw Input registrations (Windows error {})",
                error.map_or(0, |error| error.0)
            ))
        })
    })
}
fn owns(devices: &[RAWINPUTDEVICE], hwnd: HWND, usage: u16) -> bool {
    devices.iter().any(|d| {
        d.usUsagePage == 1
            && d.usUsage == usage
            && d.hwndTarget == hwnd
            && d.dwFlags == (RIDEV_INPUTSINK | RIDEV_DEVNOTIFY)
    })
}
unsafe fn read_input(context: &mut Context, lparam: LPARAM) {
    let mut bytes = 0;
    let header = size_of::<RAWINPUTHEADER>() as u32;
    let handle = HRAWINPUT(lparam.0 as *mut c_void);
    if unsafe { GetRawInputData(handle, RID_INPUT, None, &mut bytes, header) } == u32::MAX
        || bytes < header
        || bytes > 1024 * 1024
    {
        context
            .shared
            .router
            .failed(failure("Cannot read a complete Raw Input packet"));
        context.shared.stop.store(true, Ordering::Release);
        return;
    }
    // Vec<usize> gives pointer alignment at least as strict as RAWINPUT, including the header's HANDLE.
    let mut data = vec![0usize; (bytes as usize).div_ceil(size_of::<usize>())];
    let capacity = bytes;
    let read = unsafe {
        GetRawInputData(
            handle,
            RID_INPUT,
            Some(data.as_mut_ptr().cast()),
            &mut bytes,
            header,
        )
    };
    if read == u32::MAX || read != bytes || bytes > capacity || bytes < header {
        context
            .shared
            .router
            .failed(failure("Cannot read a complete Raw Input packet"));
        context.shared.stop.store(true, Ordering::Release);
        return;
    }
    let raw = unsafe { &*(data.as_ptr() as *const RAWINPUT) };
    if raw.header.dwSize != bytes || raw.header.dwSize < header {
        context
            .shared
            .router
            .failed(failure("Cannot read a complete Raw Input packet"));
        context.shared.stop.store(true, Ordering::Release);
        return;
    }
    if raw.header.dwType == RIM_TYPEKEYBOARD.0
        && bytes as usize >= size_of::<RAWINPUTHEADER>() + size_of::<RAWKEYBOARD>()
    {
        let keyboard = unsafe { raw.data.keyboard };
        if keyboard.VKey == VK_ESCAPE.0 && keyboard.Flags & 1 == 0 {
            context.shared.router.input(Input::Escape);
        }
    } else if raw.header.dwType == RIM_TYPEMOUSE.0
        && bytes as usize >= size_of::<RAWINPUTHEADER>() + size_of::<RAWMOUSE>()
    {
        let mouse = unsafe { raw.data.mouse };
        let flags = unsafe { mouse.Anonymous.Anonymous.usButtonFlags };
        if flags & 0x0155 != 0 {
            context.shared.router.input(Input::Button);
        }
        let motion = if mouse.usFlags.0 & MOUSE_MOVE_ABSOLUTE.0 != 0 {
            let position = (mouse.lLastX, mouse.lLastY);
            context
                .absolute
                .insert(raw.header.hDevice.0 as usize, position)
                != Some(position)
        } else {
            mouse.lLastX != 0 || mouse.lLastY != 0
        };
        if motion {
            context.shared.router.input(Input::Motion);
        }
    }
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize) };
    }
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut Context;
    if !ptr.is_null() {
        let context = unsafe { &mut *ptr };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match message {
            WM_INPUT => unsafe { read_input(context, lparam) },
            WM_DISPLAYCHANGE | WM_DPICHANGED => {
                context.shared.router.display_changed(Some(usize::MAX))
            }
            WM_INPUT_DEVICE_CHANGE => {
                context.absolute.remove(&(lparam.0 as usize));
            }
            WM_WTSSESSION_CHANGE => {
                if [
                    WTS_SESSION_LOCK,
                    WTS_CONSOLE_DISCONNECT,
                    WTS_REMOTE_DISCONNECT,
                    WTS_SESSION_LOGOFF,
                ]
                .contains(&(wparam.0 as u32))
                {
                    context.shared.router.cancel(StopReason::SessionInactive);
                }
            }
            WM_POWERBROADCAST if wparam.0 as u32 == PBT_APMSUSPEND => {
                context.shared.router.cancel(StopReason::SessionInactive);
            }
            _ => {}
        }));
        if outcome.is_err() {
            context.shared.stop.store(true, Ordering::Release);
        }
    }
    // Default processing performs WM_INPUT cleanup and preserves normal window behavior.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}
pub fn start(router: Arc<InputRouter>) -> Arc<dyn InputSession> {
    let shared = Arc::new(Shared {
        router,
        stop: AtomicBool::new(false),
        state: Mutex::new(SessionState::Preparing),
        route: Mutex::new(vec![]),
        cleanup_failure: Mutex::new(None),
    });
    let owner = shared.clone();
    let join = thread::spawn(move || {
        let result = (|| -> Result<(), Failure> {
            let _dpi = Dpi::enter()?;
            desktop_active()?;
            unsafe {
                if registered()?
                    .iter()
                    .any(|d| d.usUsagePage == 1 && [2, 6].contains(&d.usUsage))
                {
                    return Err(failure(
                        "Another component inside Awake already owns mouse or keyboard Raw Input. Restart Awake and retry",
                    ));
                }
                let instance = GetModuleHandleW(None).map_err(native_error)?;
                let class = w!("AwakeCursorPatrolInput");
                let wc = WNDCLASSW {
                    lpfnWndProc: Some(procedure),
                    hInstance: HINSTANCE(instance.0),
                    lpszClassName: class,
                    ..Default::default()
                };
                if RegisterClassW(&wc) == 0 {
                    return Err(failure("Cannot register the input window class"));
                }
                let mut context = Box::new(Context {
                    shared: owner.clone(),
                    absolute: HashMap::new(),
                });
                let window = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    class,
                    w!("Awake patrol input"),
                    WS_OVERLAPPED,
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    Some(HINSTANCE(instance.0)),
                    Some((&mut *context as *mut Context).cast()),
                );
                let hwnd = match window {
                    Ok(hwnd) => hwnd,
                    Err(error) => {
                        let _ = UnregisterClassW(class, Some(HINSTANCE(instance.0)));
                        return Err(native_error(error));
                    }
                };
                let devices = [2u16, 6].map(|usage| RAWINPUTDEVICE {
                    usUsagePage: 1,
                    usUsage: usage,
                    dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
                    hwndTarget: hwnd,
                });
                let registration =
                    RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32);
                let wts = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
                let power =
                    RegisterSuspendResumeNotification(HANDLE(hwnd.0), DEVICE_NOTIFY_WINDOW_HANDLE);
                let wts_registered = wts.is_ok();
                let setup = registration
                    .map_err(native_error)
                    .and_then(|_| wts.map_err(native_error))
                    .and_then(|_| power.as_ref().map(|_| ()).map_err(native_error));
                if setup.is_ok() {
                    *owner.state.lock().expect("input state") = SessionState::Ready;
                    while !owner.stop.load(Ordering::Acquire) {
                        let mut message = MSG::default();
                        let mut processed = 0;
                        while processed < 256
                            && !owner.stop.load(Ordering::Acquire)
                            && PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool()
                        {
                            processed += 1;
                            let _ = TranslateMessage(&message);
                            DispatchMessageW(&message);
                        }
                        let registrations = match registered() {
                            Ok(devices) => devices,
                            Err(error) => {
                                return_cleanup_error(&owner, error);
                                break;
                            }
                        };
                        if !owns(&registrations, hwnd, 2) || !owns(&registrations, hwnd, 6) {
                            return_cleanup_error(
                                &owner,
                                failure("Raw Input registration ownership was lost"),
                            );
                            break;
                        }
                        if desktop_active().is_err() {
                            owner.router.cancel(StopReason::SessionInactive);
                            break;
                        }
                        let route = owner.route.lock().expect("route").clone();
                        if let Err(error) = confinement(&route, None) {
                            owner
                                .router
                                .cancel(if error.code == ErrorCode::CursorConstrained {
                                    StopReason::CursorConstrained
                                } else {
                                    StopReason::NativeFailure
                                });
                            *owner.state.lock().expect("input state") = SessionState::Failed(error);
                            break;
                        }
                        owner.router.health(Instant::now());
                        thread::sleep(Duration::from_millis(25));
                    }
                }
                let registrations = match registered() {
                    Ok(devices) => devices,
                    Err(error) => {
                        *owner.cleanup_failure.lock().expect("cleanup") = Some(error);
                        Vec::new()
                    }
                };
                for usage in [2u16, 6] {
                    if owns(&registrations, hwnd, usage) {
                        if let Err(error) = RegisterRawInputDevices(
                            &[RAWINPUTDEVICE {
                                usUsagePage: 1,
                                usUsage: usage,
                                dwFlags: RIDEV_REMOVE,
                                hwndTarget: HWND::default(),
                            }],
                            size_of::<RAWINPUTDEVICE>() as u32,
                        ) {
                            *owner.cleanup_failure.lock().expect("cleanup") =
                                Some(native_error(error));
                        }
                    }
                }
                if let Err(error) = WTSUnRegisterSessionNotification(hwnd) {
                    if wts_registered {
                        *owner.cleanup_failure.lock().expect("cleanup") = Some(native_error(error));
                    }
                }
                if let Ok(power) = power {
                    if let Err(error) = UnregisterSuspendResumeNotification(power) {
                        *owner.cleanup_failure.lock().expect("cleanup") = Some(native_error(error));
                    }
                }
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                if let Err(error) = DestroyWindow(hwnd) {
                    *owner.cleanup_failure.lock().expect("cleanup") = Some(native_error(error));
                }
                if let Err(error) = UnregisterClassW(class, Some(HINSTANCE(instance.0))) {
                    *owner.cleanup_failure.lock().expect("cleanup") = Some(native_error(error));
                }
                drop(context);
                setup?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            return_cleanup_error(&owner, error);
        } else if !matches!(
            *owner.state.lock().expect("input state"),
            SessionState::Failed(_)
        ) {
            *owner.state.lock().expect("input state") = SessionState::Closed;
        }
    });
    Arc::new(Session {
        shared,
        join: Mutex::new(Some(join)),
    })
}
fn return_cleanup_error(shared: &Shared, error: Failure) {
    *shared.state.lock().expect("input state") = SessionState::Failed(error.clone());
    shared.router.failed(error);
}
impl InputSession for Session {
    fn status(&self) -> SessionState {
        self.shared.state.lock().expect("input state").clone()
    }
    fn movement(&self, route: &[XY]) -> Result<Box<dyn Movement>, Failure> {
        if !matches!(self.status(), SessionState::Ready) {
            return Err(failure("The input observer is not ready"));
        }
        *self.shared.route.lock().expect("route") = route.to_vec();
        Ok(Box::new(WindowsMovement {
            route: route.to_vec(),
        }))
    }
    fn shutdown(&self) -> Result<(), Failure> {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.lock().expect("join").take() {
            join.join()
                .map_err(|_| failure("Input observer cleanup failed; restart Awake"))?;
        }
        match self.shared.cleanup_failure.lock().expect("cleanup").clone() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
struct WindowsMovement {
    route: Vec<XY>,
}
impl Movement for WindowsMovement {
    fn prepare(&mut self, _: &[XY]) -> Result<XY, Failure> {
        let _dpi = Dpi::enter()?;
        desktop_active()?;
        if buttons_down() {
            return Err(Failure::new(
                ErrorCode::Busy,
                "Release all mouse buttons before starting",
            ));
        }
        let origin = cursor()?;
        confinement(&self.route, Some(origin))?;
        Ok(origin)
    }
    fn position(
        &mut self,
        point: XY,
        before_call: &dyn Fn() -> Result<(), StopReason>,
    ) -> Result<(), StopReason> {
        let _dpi = Dpi::enter().map_err(|_| StopReason::NativeFailure)?;
        desktop_active().map_err(|_| StopReason::SessionInactive)?;
        confinement(&self.route, Some(point)).map_err(|e| {
            if e.code == ErrorCode::CursorConstrained {
                StopReason::CursorConstrained
            } else {
                StopReason::NativeFailure
            }
        })?;
        let (x, y) = (
            quantize(point.x).ok_or(StopReason::NativeFailure)? as i32,
            quantize(point.y).ok_or(StopReason::NativeFailure)? as i32,
        );
        before_call()?;
        unsafe {
            SetCursorPos(x, y).map_err(|_| StopReason::NativeFailure)?;
            let mut actual = POINT::default();
            GetCursorPos(&mut actual).map_err(|_| StopReason::NativeFailure)?;
            if actual.x != x || actual.y != y {
                return Err(if confinement(&self.route, Some(point)).is_err() {
                    StopReason::CursorConstrained
                } else {
                    StopReason::NativeFailure
                });
            }
        }
        Ok(())
    }
}
