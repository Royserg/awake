//! Only this boundary knows native input/display identities. Public snapshots use session tokens.
use super::{
    engine::{Movement, RunControl},
    model::*,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

/// Windows enumeration separates the size probe from a populated snapshot. Both a null probe
/// and a concurrent registration change may report the required size instead of copying data.
#[cfg(any(windows, test))]
pub enum RegistrationRead {
    Required(usize),
    Filled(usize),
}

#[cfg(any(windows, test))]
pub fn registration_read(
    copied: u32,
    required: u32,
    probe: bool,
    insufficient_buffer: bool,
) -> Option<RegistrationRead> {
    if copied == u32::MAX {
        insufficient_buffer.then_some(RegistrationRead::Required(required as usize))
    } else if probe {
        Some(RegistrationRead::Required(required as usize))
    } else {
        Some(RegistrationRead::Filled(copied as usize))
    }
}

#[cfg(any(windows, test))]
pub fn read_raw_input_registrations<T: Default + Clone>(
    mut query: impl FnMut(Option<&mut [T]>) -> Result<RegistrationRead, Failure>,
) -> Result<Vec<T>, Failure> {
    let invalid = || {
        Failure::new(
            ErrorCode::NativeUnavailable,
            "Invalid Raw Input registration query result",
        )
    };
    let mut count = match query(None)? {
        RegistrationRead::Required(count) => count,
        RegistrationRead::Filled(_) => return Err(invalid()),
    };
    for _ in 0..3 {
        if count > 4096 {
            return Err(invalid());
        }
        if count == 0 {
            return Ok(Vec::new());
        }
        let mut devices = vec![T::default(); count];
        match query(Some(&mut devices))? {
            RegistrationRead::Filled(written) if written <= devices.len() => {
                devices.truncate(written);
                return Ok(devices);
            }
            RegistrationRead::Filled(_) => return Err(invalid()),
            RegistrationRead::Required(required) => count = required,
        }
    }
    Err(Failure::new(
        ErrorCode::NativeUnavailable,
        "Awake's Raw Input registrations kept changing; retry input monitoring",
    ))
}

/// Windows reports an INPUTSINK|DEVNOTIFY registration back as INPUTSINK alone (observed on
/// Windows 11 build 26200), so ownership accepts the sink flag with or without the notify flag.
#[cfg(any(windows, test))]
pub fn sink_registration_flags(reported: u32, sink: u32, notify: u32) -> bool {
    reported & !notify == sink
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeDisplay {
    pub identity: String,
    pub surface: String,
    pub name: String,
    pub primary: bool,
    pub bounds: Rect,
    pub scale: XY,
    pub rotation: f64,
    pub native_id: usize,
    pub placement_id: usize,
}
impl NativeDisplay {
    pub fn same_geometry(&self, other: &Self) -> bool {
        self.primary == other.primary
            && self.identity == other.identity
            && self.surface == other.surface
            && self.bounds == other.bounds
            && self.scale == other.scale
            && self.rotation == other.rotation
    }
}
#[derive(Clone, Debug)]
pub enum SessionState {
    Preparing,
    Ready,
    Failed(Failure),
    Closed,
}
#[derive(Clone, Copy)]
pub enum Input {
    Escape,
    Motion,
    Button,
}
#[derive(Clone, Debug)]
pub enum Event {
    EscapeSetup(Id),
    DisplayChanged(Option<usize>),
    ObserverFailed(Failure),
}
struct Target {
    setup: Option<Id>,
    control: Option<Arc<RunControl>>,
    takeover: bool,
    escape_latched: bool,
}
pub struct InputRouter {
    target: Mutex<Target>,
    enabled: AtomicBool,
    event: Box<dyn Fn(Event) + Send + Sync>,
}
impl InputRouter {
    pub fn new(setup: Id, event: impl Fn(Event) + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            target: Mutex::new(Target {
                setup: Some(setup),
                control: None,
                takeover: false,
                escape_latched: false,
            }),
            enabled: AtomicBool::new(true),
            event: Box::new(event),
        })
    }
    pub fn input(&self, input: Input) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        let setup = {
            let mut target = self.target.lock().expect("input routing");
            match input {
                Input::Escape => {
                    target.escape_latched = true;
                    if let Some(control) = &target.control {
                        control.cancel(StopReason::Escape);
                    }
                    target.setup
                }
                Input::Motion | Input::Button => {
                    if target.takeover {
                        if let Some(control) = &target.control {
                            control.cancel(if matches!(input, Input::Motion) {
                                StopReason::ManualMovement
                            } else {
                                StopReason::MouseButton
                            });
                        }
                    }
                    None
                }
            }
        };
        if let Some(setup) = setup {
            (self.event)(Event::EscapeSetup(setup));
        }
    }
    pub fn set_setup(&self, setup: Id) {
        let mut target = self.target.lock().expect("input routing");
        target.setup = Some(setup);
        target.control = None;
        target.takeover = false;
        target.escape_latched = false;
    }
    pub fn reserve_run(&self, control: Arc<RunControl>) -> bool {
        let mut target = self.target.lock().expect("input routing");
        if target.escape_latched {
            control.cancel(StopReason::Escape);
            return false;
        }
        target.control = Some(control);
        target.takeover = false;
        true
    }
    pub fn arm(&self, run: Id) -> bool {
        let mut target = self.target.lock().expect("input routing");
        if target.escape_latched
            || target
                .control
                .as_ref()
                .is_none_or(|c| c.run_id != run || c.reason().is_some())
        {
            return false;
        }
        target.takeover = true;
        true
    }
    pub fn committed(&self, run: Id) {
        let mut target = self.target.lock().expect("input routing");
        if target.control.as_ref().is_some_and(|c| c.run_id == run) {
            target.setup = None;
        }
    }
    pub fn clear_run(&self, run: Id) {
        let mut target = self.target.lock().expect("input routing");
        if target.control.as_ref().is_some_and(|c| c.run_id == run) {
            target.control = None;
            target.takeover = false;
        }
    }
    pub fn health(&self, at: Instant) {
        let control = self.target.lock().expect("input routing").control.clone();
        if let Some(control) = control {
            control.publish_observer(control.run_id, at);
        }
    }
    pub fn cancel(&self, reason: StopReason) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        let control = self.target.lock().expect("input routing").control.clone();
        if let Some(control) = control {
            control.cancel(reason);
        }
    }
    pub fn display_changed(&self, retired: Option<usize>) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        self.cancel(StopReason::DisplayChanged);
        (self.event)(Event::DisplayChanged(retired));
    }
    pub fn failed(&self, failure: Failure) {
        self.cancel(StopReason::ObserverLost);
        if self.enabled.load(Ordering::Acquire) {
            (self.event)(Event::ObserverFailed(failure));
        }
    }
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
    }
}
pub trait InputSession: Send + Sync {
    fn status(&self) -> SessionState;
    fn movement(&self, route: &[XY]) -> Result<Box<dyn Movement>, Failure>;
    /// Executed off the UI thread; must retain callback storage until its thread is joined.
    fn shutdown(&self) -> Result<(), Failure>;
}

pub fn availability() -> Availability {
    #[cfg(target_os = "macos")]
    {
        macos::availability()
    }
    #[cfg(windows)]
    {
        windows::availability()
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Availability {
            state: AvailabilityState::UnsupportedPlatform,
            message: Some("Cursor patrolling is available on macOS and Windows".into()),
        }
    }
}
pub fn displays() -> Result<Vec<NativeDisplay>, Failure> {
    #[cfg(target_os = "macos")]
    {
        macos::displays()
    }
    #[cfg(windows)]
    {
        windows::displays()
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Err(Failure::new(
            ErrorCode::UnsupportedPlatform,
            "Cursor patrolling is unavailable on this platform",
        ))
    }
}
pub fn cursor() -> Result<XY, Failure> {
    #[cfg(target_os = "macos")]
    {
        macos::cursor()
    }
    #[cfg(windows)]
    {
        windows::cursor()
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Err(Failure::new(
            ErrorCode::UnsupportedPlatform,
            "Cursor patrolling is unavailable on this platform",
        ))
    }
}
pub fn buttons_down() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::buttons_down()
    }
    #[cfg(windows)]
    {
        windows::buttons_down()
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        false
    }
}
pub fn place(window: &tauri::WebviewWindow, display: &NativeDisplay) -> Result<Mapping, Failure> {
    #[cfg(target_os = "macos")]
    {
        macos::place(window, display)
    }
    #[cfg(windows)]
    {
        windows::place(window, display)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = (window, display);
        Err(Failure::new(
            ErrorCode::UnsupportedPlatform,
            "Cursor patrolling is unavailable on this platform",
        ))
    }
}
pub fn start(router: Arc<InputRouter>) -> Arc<dyn InputSession> {
    #[cfg(target_os = "macos")]
    {
        macos::start(router)
    }
    #[cfg(windows)]
    {
        windows::start(router)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = router;
        Arc::new(Unsupported)
    }
}
pub fn request_permissions() -> Availability {
    #[cfg(target_os = "macos")]
    {
        macos::request_permissions()
    }
    #[cfg(not(target_os = "macos"))]
    {
        availability()
    }
}
#[cfg(not(any(target_os = "macos", windows)))]
struct Unsupported;
#[cfg(not(any(target_os = "macos", windows)))]
impl InputSession for Unsupported {
    fn status(&self) -> SessionState {
        SessionState::Failed(Failure::new(
            ErrorCode::UnsupportedPlatform,
            "Cursor patrolling is unavailable on this platform",
        ))
    }
    fn movement(&self, _: &[XY]) -> Result<Box<dyn Movement>, Failure> {
        Err(Failure::new(
            ErrorCode::UnsupportedPlatform,
            "Cursor patrolling is unavailable on this platform",
        ))
    }
    fn shutdown(&self) -> Result<(), Failure> {
        Ok(())
    }
}

/// Main-thread notification ownership never crosses the managed runtime mutex.
pub fn notifications(router: Option<Arc<InputRouter>>) {
    #[cfg(target_os = "macos")]
    macos::notifications(router);
    #[cfg(not(target_os = "macos"))]
    let _ = router;
}
