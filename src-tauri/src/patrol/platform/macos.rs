//! Public AppKit/Core Graphics adapter. The event tap and its callback storage share one owner thread.
use super::*;
use objc2::{msg_send, MainThreadMarker};
use objc2_app_kit::{
    NSScreen, NSWindow, NSWorkspace, NSWorkspaceScreensDidSleepNotification,
    NSWorkspaceSessionDidResignActiveNotification, NSWorkspaceWillSleepNotification,
};
use objc2_core_foundation::{
    kCFBooleanTrue, kCFRunLoopDefaultMode, CFMachPort, CFRunLoop, CFString, CGPoint,
};
use objc2_core_graphics::*;
use objc2_foundation::{ns_string, NSOperationQueue};
use std::{
    ffi::{c_char, c_void},
    ptr::NonNull,
    thread::{self, JoinHandle},
    time::Duration,
};

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
}
extern "C" {
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}
// These two 10.15+ functions are optional. Resolve rather than creating a load-time OS requirement.
fn access(request: bool) -> Option<bool> {
    let name = if request {
        c"CGRequestListenEventAccess"
    } else {
        c"CGPreflightListenEventAccess"
    };
    // RTLD_DEFAULT is the Darwin SDK's -2 pseudo handle; the function has the public bool(void) ABI.
    let symbol = unsafe { dlsym((-2isize) as *mut c_void, name.as_ptr()) };
    if symbol.is_null() {
        None
    } else {
        Some(unsafe {
            std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> bool>(symbol)()
        })
    }
}
fn unavailable(message: &str) -> Availability {
    Availability {
        state: AvailabilityState::NativeUnavailable,
        message: Some(message.into()),
    }
}
pub fn availability() -> Availability {
    if MainThreadMarker::new().is_none() {
        return unavailable("Input availability must be checked on the application thread");
    }
    let Some(allowed) = access(false) else {
        return unavailable("Input monitoring requires macOS 10.15 or later");
    };
    if !allowed {
        return Availability{state:AvailabilityState::PermissionRequired,message:Some("Allow Awake in System Settings → Privacy & Security → Input Monitoring, then retry".into())};
    }
    let Some(session) = CGSessionCopyCurrentDictionary() else {
        return unavailable("No active desktop session");
    };
    // Public dictionary keys are CFStrings; copied dictionary owns all queried values for this scope.
    let yes = unsafe { kCFBooleanTrue }
        .map(|v| v as *const _ as *const c_void)
        .unwrap_or(std::ptr::null());
    let active = ["kCGSSessionOnConsoleKey", "kCGSessionLoginDoneKey"]
        .iter()
        .all(|key| {
            let key = CFString::from_str(key);
            unsafe { session.value(&*key as *const _ as *const c_void) == yes }
        });
    if !active {
        return unavailable("Unlock and return to the active desktop session");
    }
    // Carbon explicitly requires serialized use; all calls to this function are on the main thread.
    if unsafe { IsSecureEventInputEnabled() } {
        return unavailable("Protected keyboard input is active; close it and retry");
    }
    Availability {
        state: AvailabilityState::Ready,
        message: None,
    }
}
pub fn request_permissions() -> Availability {
    if access(false) == Some(false) {
        let _ = access(true);
    }
    availability()
}
fn failure(message: &str) -> Failure {
    Failure::new(ErrorCode::NativeUnavailable, message)
}
pub fn displays() -> Result<Vec<NativeDisplay>, Failure> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| failure("Display enumeration requires the application thread"))?;
    let mut out = vec![];
    for screen in NSScreen::screens(mtm).iter() {
        let description = screen.deviceDescription();
        let number = description
            .objectForKey(ns_string!("NSScreenNumber"))
            .ok_or_else(|| failure("Missing display identity"))?;
        // NSScreenNumber is an NSNumber per AppKit's public device-description contract.
        let id: u32 = unsafe { msg_send![&*number, unsignedIntValue] };
        let bounds = CGDisplayBounds(id);
        let scale = screen.backingScaleFactor();
        let mirror = CGDisplayMirrorsDisplay(id);
        out.push(NativeDisplay {
            identity: format!(
                "{}:{}:{}:{}",
                CGDisplayVendorNumber(id),
                CGDisplayModelNumber(id),
                CGDisplaySerialNumber(id),
                id
            ),
            surface: format!("{}", if mirror == 0 { id } else { mirror }),
            name: screen.localizedName().to_string(),
            primary: id == CGMainDisplayID(),
            bounds: Rect {
                x: bounds.origin.x,
                y: bounds.origin.y,
                width: bounds.size.width,
                height: bounds.size.height,
            },
            scale: XY { x: scale, y: scale },
            rotation: CGDisplayRotation(id),
            native_id: id as usize,
            placement_id: id as usize,
        });
    }
    if out.is_empty() {
        return Err(failure("No usable desktop displays"));
    }
    out.sort_by(|a, b| a.identity.cmp(&b.identity));
    Ok(out)
}
pub fn cursor() -> Result<XY, Failure> {
    let event = CGEvent::new(None).ok_or_else(|| failure("Cannot read the cursor position"))?;
    let p = CGEvent::location(Some(&event));
    Ok(XY { x: p.x, y: p.y })
}
pub fn buttons_down() -> bool {
    (0..32).any(|b| {
        CGEventSource::button_state(CGEventSourceStateID::CombinedSessionState, CGMouseButton(b))
    })
}
pub fn place(window: &tauri::WebviewWindow, display: &NativeDisplay) -> Result<Mapping, Failure> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| failure("Window placement requires the application thread"))?;
    for screen in NSScreen::screens(mtm).iter() {
        let description = screen.deviceDescription();
        let Some(number) = description.objectForKey(ns_string!("NSScreenNumber")) else {
            continue;
        };
        let id: u32 = unsafe { msg_send![&*number, unsignedIntValue] };
        if id as usize != display.placement_id {
            continue;
        }
        // Tauri owns the NSWindow. Borrow it only on the main thread, without extending its lifetime.
        let ptr = window.ns_window().map_err(|e| failure(&e.to_string()))?;
        let native = unsafe { &*(ptr as *const NSWindow) };
        let desired = screen.frame();
        native.setFrame_display(desired, false);
        let frame = native.frame();
        let content = native.contentRectForFrameRect(frame);
        if frame != desired || content != desired {
            return Err(failure("The setup window does not cover its display"));
        }
        return Ok(Mapping {
            origin: XY {
                x: display.bounds.x,
                y: display.bounds.y,
            },
            width: content.size.width,
            height: content.size.height,
        });
    }
    Err(failure("The selected display disappeared"))
}
struct Shared {
    router: Arc<InputRouter>,
    stop: AtomicBool,
    state: Mutex<SessionState>,
}
struct Session {
    shared: Arc<Shared>,
    join: Mutex<Option<JoinHandle<()>>>,
}
// Callback context is an Arc retained by the owner thread until the tap is disabled and unregistered.
unsafe extern "C-unwind" fn input(
    _: CGEventTapProxy,
    kind: CGEventType,
    event: NonNull<CGEvent>,
    context: *mut c_void,
) -> *mut CGEvent {
    let shared = unsafe { &*(context as *const Shared) };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match kind.0 {
        0xffff_fffe | 0xffff_ffff => shared.router.failed(failure(
            "Input monitoring stopped; reopen Cursor Patrolling to retry",
        )),
        10 => {
            if CGEvent::integer_value_field(
                Some(unsafe { event.as_ref() }),
                CGEventField::KeyboardEventKeycode,
            ) == 53
            {
                shared.router.input(Input::Escape);
            }
        }
        5 | 6 | 7 | 27 => shared.router.input(Input::Motion),
        1 | 3 | 25 => shared.router.input(Input::Button),
        _ => {}
    }));
    if outcome.is_err() {
        shared.stop.store(true, Ordering::Release);
    }
    event.as_ptr()
}
unsafe extern "C-unwind" fn display(
    native_id: u32,
    flags: CGDisplayChangeSummaryFlags,
    context: *mut c_void,
) {
    let shared = unsafe { &*(context as *const Shared) };
    let retired = flags
        .intersects(
            CGDisplayChangeSummaryFlags::RemoveFlag | CGDisplayChangeSummaryFlags::DisabledFlag,
        )
        .then_some(native_id as usize);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        shared.router.display_changed(retired)
    }));
}
pub fn start(router: Arc<InputRouter>) -> Arc<dyn InputSession> {
    let shared = Arc::new(Shared {
        router,
        stop: AtomicBool::new(false),
        state: Mutex::new(SessionState::Preparing),
    });
    let owner = shared.clone();
    let join = thread::spawn(move || {
        objc2::rc::autoreleasepool(|_| {
            let context = Arc::as_ptr(&owner) as *mut c_void;
            let mask = [10, 5, 6, 7, 27, 1, 3, 25]
                .iter()
                .fold(0u64, |mask, bit| mask | (1u64 << bit));
            // Storage lives through all run-loop callbacks and display callback deregistration below.
            let Some(tap) = (unsafe {
                CGEvent::tap_create(
                    CGEventTapLocation::SessionEventTap,
                    CGEventTapPlacement::HeadInsertEventTap,
                    CGEventTapOptions::ListenOnly,
                    mask,
                    Some(input),
                    context,
                )
            }) else {
                *owner.state.lock().expect("session") = SessionState::Failed(failure(
                    "Cannot create the input observer; check Input Monitoring access",
                ));
                return;
            };
            let Some(source) = CFMachPort::new_run_loop_source(None, Some(&tap), 0) else {
                *owner.state.lock().expect("session") =
                    SessionState::Failed(failure("Cannot create the input run loop"));
                return;
            };
            let Some(run_loop) = CFRunLoop::current() else {
                *owner.state.lock().expect("session") =
                    SessionState::Failed(failure("Cannot obtain the input run loop"));
                return;
            };
            let mode = unsafe { kCFRunLoopDefaultMode };
            run_loop.add_source(Some(&source), mode);
            let registered =
                unsafe { CGDisplayRegisterReconfigurationCallback(Some(display), context) }
                    == CGError::Success;
            if !registered {
                *owner.state.lock().expect("session") =
                    SessionState::Failed(failure("Cannot observe display changes"));
            } else {
                *owner.state.lock().expect("session") = SessionState::Ready;
            }
            while registered && !owner.stop.load(Ordering::Acquire) {
                CFRunLoop::run_in_mode(mode, 0.025, false);
                if !CGEvent::tap_is_enabled(&tap) || access(false) != Some(true) {
                    let error =
                        failure("Input monitoring was lost; reopen Cursor Patrolling to retry");
                    *owner.state.lock().expect("session") = SessionState::Failed(error.clone());
                    owner.router.failed(error);
                    break;
                }
                owner.router.health(Instant::now());
            }
            CGEvent::tap_enable(&tap, false);
            if registered {
                while unsafe { CGDisplayRemoveReconfigurationCallback(Some(display), context) }
                    != CGError::Success
                {
                    *owner.state.lock().expect("session") =
                        SessionState::Failed(failure("Input observer cleanup is still pending"));
                    // Never free callback storage while Core Graphics may still reference it.
                    thread::sleep(Duration::from_millis(HEALTH_MS));
                }
            }
            run_loop.remove_source(Some(&source), mode);
            tap.invalidate();
            *owner.state.lock().expect("session") = SessionState::Closed;
        })
    });
    Arc::new(Session {
        shared,
        join: Mutex::new(Some(join)),
    })
}
impl InputSession for Session {
    fn status(&self) -> SessionState {
        self.shared.state.lock().expect("session").clone()
    }
    fn movement(&self, _: &[XY]) -> Result<Box<dyn Movement>, Failure> {
        if !matches!(self.status(), SessionState::Ready) {
            return Err(failure("Input observer is not ready"));
        }
        Ok(Box::new(MacMovement))
    }
    fn shutdown(&self) -> Result<(), Failure> {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.lock().expect("join").take() {
            join.join()
                .map_err(|_| failure("Input observer cleanup failed; restart Awake"))?;
        }
        Ok(())
    }
}
struct MacMovement;
impl Movement for MacMovement {
    fn prepare(&mut self, _: &[XY]) -> Result<XY, Failure> {
        if buttons_down() {
            return Err(Failure::new(
                ErrorCode::Busy,
                "Release all mouse buttons before starting",
            ));
        }
        cursor()
    }
    fn position(
        &mut self,
        point: XY,
        before_call: &dyn Fn() -> Result<(), StopReason>,
    ) -> Result<(), StopReason> {
        before_call()?;
        if CGWarpMouseCursorPosition(CGPoint::new(point.x, point.y)) == CGError::Success {
            Ok(())
        } else {
            Err(StopReason::NativeFailure)
        }
    }
}
/// Install public sleep/session notifications on the main thread; retain tokens until cleanup there.
pub struct Notifications {
    center: objc2::rc::Retained<objc2_foundation::NSNotificationCenter>,
    tokens: Vec<
        objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_foundation::NSObjectProtocol>>,
    >,
}
impl Notifications {
    pub fn new(router: Arc<InputRouter>) -> Self {
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        let mut tokens = vec![];
        for name in unsafe {
            [
                NSWorkspaceWillSleepNotification,
                NSWorkspaceScreensDidSleepNotification,
                NSWorkspaceSessionDidResignActiveNotification,
            ]
        } {
            let router = router.clone();
            let block =
                block2::RcBlock::new(move |_: NonNull<objc2_foundation::NSNotification>| {
                    router.cancel(StopReason::SessionInactive)
                });
            // Observer token and block are retained by the notification center. Remove before dropping them.
            tokens.push(unsafe {
                center.addObserverForName_object_queue_usingBlock(
                    Some(name),
                    None,
                    Some(&NSOperationQueue::mainQueue()),
                    &block,
                )
            });
        }
        Self { center, tokens }
    }
}
impl Drop for Notifications {
    fn drop(&mut self) {
        for token in &self.tokens {
            unsafe {
                self.center
                    .removeObserver(AsRef::<objc2::runtime::AnyObject>::as_ref(&**token))
            };
        }
    }
}

thread_local! {static NOTIFICATIONS:std::cell::RefCell<Option<Notifications>>=const{std::cell::RefCell::new(None)};}
pub fn notifications(router: Option<Arc<InputRouter>>) {
    NOTIFICATIONS.with(|slot| *slot.borrow_mut() = router.map(Notifications::new));
}
