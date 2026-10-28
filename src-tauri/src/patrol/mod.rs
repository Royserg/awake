//! Main-thread effect coordinator. Native callbacks only touch RunControl/InputRouter and enqueue work.
mod controller;
mod engine;
mod model;
mod overlays;
mod platform;
#[cfg(test)]
mod tests;
use controller::{Controller, Effect, StartDisposition};
use engine::{Deadline, EmissionGate, RunControl};
use model::*;
use platform::{InputRouter, InputSession, NativeDisplay, SessionState};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Condvar, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewWindow};
type Response = Result<Reply, PatrolError>;
type Sender = mpsc::Sender<Response>;
type Work = Box<dyn FnOnce(&tauri::AppHandle, &mut Core) + Send>;
#[derive(Clone)]
pub enum AwakeOp {
    Activate,
    Deactivate,
    Toggle,
    SetTimer(u64),
    CancelTimer,
    Expire(u64),
}
struct AwakeWork {
    op: AwakeOp,
    reply: Option<mpsc::Sender<Result<(), String>>>,
}
struct Core {
    controller: Controller,
    native: Vec<(Id, NativeDisplay)>,
    last_display: Instant,
    session: Option<Arc<dyn InputSession>>,
    router: Option<Arc<InputRouter>>,
    session_epoch: u64,
    cleaning: bool,
    creating: bool,
    control: Option<Arc<RunControl>>,
    worker_started: bool,
    worker_failure: Option<(Id, Failure)>,
    menu_phase: Option<Phase>,
    tray_dirty: bool,
    last_published: Option<(Id, Id)>,
    gate: Arc<EmissionGate>,
    starts: BTreeMap<Id, Vec<(String, Sender)>>,
    stops: Vec<(Id, String, Sender, Instant)>,
    closes: Vec<(String, Sender)>,
    close_failure: Option<Failure>,
    awake: VecDeque<AwakeWork>,
    timer_revision: u64,
    rebuild: bool,
    destroying: bool,
    hidden: bool,
    quitting: Option<Instant>,
    exit_allowed: bool,
    exit_code: i32,
    expected_destroys: BTreeSet<String>,
}
pub struct PatrolRuntime {
    core: Mutex<Core>,
    wake: Arc<(Mutex<bool>, Condvar)>,
    tick_queued: AtomicBool,
    dispatch: mpsc::Sender<Work>,
}
impl PatrolRuntime {
    fn new(dispatch: mpsc::Sender<Work>) -> Self {
        let controller = Controller::new();
        let gate = EmissionGate::new(controller.layout_revision);
        Self {
            core: Mutex::new(Core {
                controller,
                native: vec![],
                last_display: Instant::now(),
                session: None,
                router: None,
                session_epoch: 0,
                cleaning: false,
                creating: false,
                control: None,
                worker_started: false,
                worker_failure: None,
                menu_phase: None,
                tray_dirty: false,
                last_published: None,
                gate,
                starts: BTreeMap::new(),
                stops: vec![],
                closes: vec![],
                close_failure: None,
                awake: VecDeque::new(),
                timer_revision: 0,
                rebuild: false,
                destroying: false,
                hidden: false,
                quitting: None,
                exit_allowed: false,
                exit_code: 0,
                expected_destroys: BTreeSet::new(),
            }),
            wake: Arc::new((Mutex::new(false), Condvar::new())),
            tick_queued: AtomicBool::new(false),
            dispatch,
        }
    }
    fn wake(&self) {
        let (lock, cv) = &*self.wake;
        *lock.lock().expect("patrol wake") = true;
        cv.notify_one();
    }
}
fn runtime(app: &tauri::AppHandle) -> tauri::State<'_, PatrolRuntime> {
    app.state::<PatrolRuntime>()
}
fn main(app: &tauri::AppHandle, work: impl FnOnce(&tauri::AppHandle, &mut Core) + Send + 'static) {
    // Tauri executes run_on_main_thread inline when already on that thread. Always pass through
    // our dispatcher so a synchronous native/window callback can never re-enter the Core mutex.
    let _ = runtime(app).dispatch.send(Box::new(work));
}
fn run_work(app: &tauri::AppHandle, work: Work) {
    let state = runtime(app);
    let mut core = state.core.lock().expect("patrol coordinator");
    work(app, &mut core);
    core.finish(app);
    let dirty = std::mem::take(&mut core.tray_dirty);
    drop(core);
    if dirty {
        crate::refresh_tray(app);
    }
    state.wake();
}
impl Core {
    fn reply(&self, label: &str, receipt: Option<Receipt>) -> Reply {
        Reply {
            snapshot: self.controller.snapshot(label),
            receipt,
        }
    }
    fn error(&self, label: &str, failure: Failure) -> PatrolError {
        self.controller.error(label, failure)
    }
    fn failure(&mut self, error: Failure) {
        let _ = self.controller.failure(error);
    }
    fn publish_awake(&mut self, app: &tauri::AppHandle) {
        let state = app.state::<Mutex<crate::AppState>>();
        let state = state.lock().expect("Awake state");
        let now = Instant::now();
        let deadline =
            state
                .activated_at
                .zip(state.timer_duration)
                .map(|(started_at, duration)| Deadline {
                    revision: self.timer_revision,
                    started_at,
                    duration,
                });
        let value = Awake {
            active: state.no_sleep_active,
            timer_remaining_ms: deadline
                .as_ref()
                .map(|d| Counter(d.remaining(now).as_millis().min(u64::MAX as u128) as u64)),
        };
        let mut gate = self.gate.state.lock().expect("admission");
        gate.awake_active = value.active;
        gate.deadline = deadline;
        gate.layout_revision = self.controller.layout_revision;
        drop(gate);
        drop(state);
        let _ = self.controller.publish_awake(value);
    }
    fn cancel(&mut self, reason: StopReason) {
        if let Some(control) = &self.control {
            control.cancel(reason);
        }
        if let Some(run) = self.controller.status.run_id {
            if let Ok(effects) = self.controller.request_stop(run, reason) {
                for effect in effects {
                    if let Effect::Stop(_, reason) = effect {
                        if let Some(control) = &self.control {
                            control.cancel(reason);
                        }
                    }
                }
            }
        }
        if !self.worker_started {
            if let Some(control) = &self.control {
                control.acknowledge();
            }
        }
    }
    fn session_open(&mut self, app: &tauri::AppHandle, setup: Id) {
        if self.cleaning || self.session.is_some() {
            if let Some(router) = &self.router {
                router.set_setup(setup);
            }
            return;
        }
        self.session_epoch = self
            .session_epoch
            .checked_add(1)
            .expect("session generations exhausted");
        let epoch = self.session_epoch;
        let handle = app.clone();
        let router = InputRouter::new(setup, move |event| {
            main(&handle, move |app, core| {
                if core.session_epoch != epoch {
                    return;
                }
                match event {
                    platform::Event::EscapeSetup(id) => {
                        if core.controller.setup_id() == Some(id) {
                            core.close(app, None);
                        }
                    }
                    platform::Event::DisplayChanged(retired) => {
                        core.cancel(StopReason::DisplayChanged);
                        core.refresh_displays(app, true, retired);
                    }
                    platform::Event::ObserverFailed(error) => {
                        core.failure(error.clone());
                        core.cancel(StopReason::ObserverLost);
                    }
                }
            });
        });
        platform::notifications(Some(router.clone()));
        self.session = Some(platform::start(router.clone()));
        self.router = Some(router);
    }
    fn session_close(&mut self, app: &tauri::AppHandle) {
        let Some(session) = self.session.take() else {
            return;
        };
        if let Some(router) = self.router.take() {
            router.disable();
        }
        platform::notifications(None);
        self.cleaning = true;
        let handle = app.clone();
        let epoch = self.session_epoch;
        thread::spawn(move || {
            let cleanup = session.shutdown();
            main(&handle, move |app, core| {
                if core.session_epoch == epoch {
                    if let Err(error) = cleanup {
                        core.failure(error);
                        return;
                    }
                    core.cleaning = false;
                    if let Some(setup) = core.controller.setup_id() {
                        if !core.controller.closing_setup {
                            core.session_open(app, setup);
                        }
                    }
                }
            });
        });
    }
    fn refresh_displays(
        &mut self,
        app: &tauri::AppHandle,
        discontinuity: bool,
        retired: Option<usize>,
    ) {
        self.last_display = Instant::now();
        let incoming = match platform::displays() {
            Ok(d) => d,
            Err(error) => {
                self.failure(error);
                vec![]
            }
        };
        let unchanged = incoming.len() == self.native.len()
            && incoming
                .iter()
                .all(|d| self.native.iter().any(|(_, old)| old.same_geometry(d)));
        if unchanged && !discontinuity {
            return;
        }
        let old = std::mem::take(&mut self.native);
        let mut native = vec![];
        for display in incoming {
            // A native callback without an observed continuous identity is conservatively a new attachment.
            let id = if retired != Some(usize::MAX) {
                old.iter()
                    .find(|(_, d)| d.identity == display.identity && retired != Some(d.native_id))
                    .map(|(id, _)| *id)
            } else {
                None
            };
            let id = match id
                .map(Ok)
                .unwrap_or_else(|| self.controller.next_display_id())
            {
                Ok(id) => id,
                Err(error) => {
                    self.failure(error);
                    return;
                }
            };
            native.push((id, display));
        }
        let mut displays = vec![];
        for (id, d) in &native {
            let canonical = native
                .iter()
                .filter(|(_, other)| other.surface == d.surface)
                .map(|(id, _)| *id)
                .min()
                .unwrap_or(*id);
            displays.push(Display {
                display_id: *id,
                name: d.name.clone(),
                native_bounds: d.bounds,
                unit: if cfg!(windows) {
                    Unit::WindowsPixel
                } else {
                    Unit::MacosPoint
                },
                scale: d.scale,
                rotation_degrees: d.rotation,
                mirror_of: (canonical != *id).then_some(canonical),
            });
        }
        self.native = native;
        match self.controller.layout_changed(displays) {
            Ok(effects) => self.effects(app, effects),
            Err(error) => self.failure(error),
        }
        self.gate.state.lock().expect("admission").layout_revision =
            self.controller.layout_revision;
        if self.controller.setup.is_some() {
            self.rebuild = true;
        }
    }
    fn open(&mut self, app: &tauri::AppHandle) {
        if self.controller.status.run_id.is_some() || self.creating {
            return;
        }
        if self.controller.setup.is_some() && !self.controller.closing_setup && !self.rebuild {
            if let Err(error) = overlays::show(app, &self.controller.labels()) {
                self.failure(error);
            }
            self.hidden = false;
            return;
        }
        if self.cleaning || self.controller.closing_setup {
            return;
        }
        self.refresh_displays(app, false, None);
        match self.controller.open() {
            Ok(setup) => {
                self.session_open(app, setup);
                self.create_overlays(app);
            }
            Err(error) => self.failure(error),
        }
    }
    fn create_overlays(&mut self, app: &tauri::AppHandle) {
        #[cfg(not(windows))]
        if let Err(error) = overlays::create(app, &mut self.controller, &self.native) {
            self.fail_setup(app, error);
        }
        #[cfg(windows)]
        {
            let plans = match overlays::plan(&mut self.controller, &self.native) {
                Ok(plans) => plans,
                Err(error) => {
                    self.fail_setup(app, error);
                    return;
                }
            };
            let setup = self.controller.setup_id();
            let layout = self.controller.layout_revision;
            self.creating = true;
            let handle = app.clone();
            // WebView2 explicitly requires window construction outside a synchronous UI event handler.
            // Native placement, mapping and visibility still happen on the main actor after construction.
            thread::spawn(move || {
                let result = overlays::build(&handle, &plans);
                main(&handle, move |app, core| {
                    core.creating = false;
                    if core.controller.closing_setup {
                        return;
                    }
                    if core.controller.setup_id() != setup
                        || core.controller.layout_revision != layout
                    {
                        core.rebuild = true;
                        return;
                    }
                    if let Err(error) = result
                        .and_then(|_| overlays::place_and_show(app, &mut core.controller, &plans))
                    {
                        core.fail_setup(app, error);
                    }
                });
            });
        }
    }
    fn fail_setup(&mut self, app: &tauri::AppHandle, error: Failure) {
        self.close_failure = Some(error.clone());
        self.failure(error);
        self.close(app, None);
    }
    fn close(&mut self, app: &tauri::AppHandle, reply: Option<(String, Sender)>) {
        if let Some(reply) = reply {
            self.closes.push(reply);
        }
        let _ = self.controller.begin_close();
        self.cancel(StopReason::TrayStop);
        self.try_close(app);
    }
    fn destroy(&mut self, app: &tauri::AppHandle) -> Result<(), Failure> {
        let labels = self.controller.labels();
        for label in &labels {
            if app.get_webview_window(label).is_some() {
                self.expected_destroys.insert(label.clone());
            }
        }
        overlays::destroy(app, &labels)
    }
    fn try_close(&mut self, app: &tauri::AppHandle) {
        if !self.controller.closing_setup
            || self.controller.status.run_id.is_some()
            || self.creating
        {
            return;
        }
        self.destroying = true;
        let result = self.destroy(app);
        self.destroying = false;
        match result {
            Ok(()) => {
                let _ = self.controller.finish_close();
                if let Some(error) = self.close_failure.take() {
                    self.failure(error);
                }
                self.hidden = false;
                self.rebuild = false;
                self.session_close(app);
                for (label, tx) in self.closes.drain(..) {
                    let _ = tx.send(Ok(Reply {
                        snapshot: self.controller.snapshot(&label),
                        receipt: None,
                    }));
                }
            }
            Err(error) => self.failure(error),
        }
    }
    fn effects(&mut self, app: &tauri::AppHandle, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Arm(run) => self.arm(app, run),
                Effect::StartFailed(run, error) => {
                    self.resolve_start(run, Err(error));
                    if self.control.as_ref().is_some_and(|c| c.run_id == run) {
                        self.control.take();
                        self.worker_started = false;
                        if let Some(router) = &self.router {
                            router.clear_run(run);
                        }
                    }
                }
                Effect::Stop(run, reason) => {
                    if let Some(control) = &self.control {
                        if control.run_id == run {
                            control.cancel(reason);
                            if !self.worker_started {
                                control.acknowledge();
                            }
                        }
                    }
                }
            }
        }
    }
    fn resolve_start(&mut self, run: Id, result: Result<Receipt, Failure>) {
        if let Some(waiters) = self.starts.remove(&run) {
            for (label, tx) in waiters {
                let answer = match &result {
                    Ok(receipt) => Ok(self.reply(&label, Some(receipt.clone()))),
                    Err(error) => Err(self.error(&label, error.clone())),
                };
                let _ = tx.send(answer);
            }
        }
    }
    fn arm(&mut self, app: &tauri::AppHandle, run: Id) {
        if !self.controller.arming(run) || self.worker_started {
            return;
        }
        if !self.awake.is_empty() || self.quitting.is_some() {
            self.arm_failed(
                app,
                run,
                Failure::new(
                    ErrorCode::Busy,
                    "An Awake transition is pending; start again when it finishes",
                ),
            );
            return;
        }
        let Some(control) = self.control.clone() else {
            return;
        };
        if !self.router.as_ref().is_some_and(|r| r.arm(run)) {
            self.arm_failed(
                app,
                run,
                Failure::new(ErrorCode::StartCanceled, "Patrol Start was canceled"),
            );
            return;
        }
        self.hidden = true;
        if let Err(error) = overlays::hide(app, &self.controller.labels()) {
            self.arm_failed(app, run, error);
            return;
        }
        self.hidden = true;
        let available = platform::availability();
        if available.state != AvailabilityState::Ready {
            self.arm_failed(
                app,
                run,
                Failure::new(
                    ErrorCode::NativeUnavailable,
                    available.message.unwrap_or_default(),
                ),
            );
            return;
        }
        control.publish_availability(run, Instant::now());
        // No worker has started, hence no emission can overlap this activation. Existing timed sessions stay intact.
        let (result, was_active) = {
            let managed = app.state::<Mutex<crate::AppState>>();
            let mut state = managed.lock().expect("Awake");
            let was_active = state.no_sleep_active;
            (state.activate(), was_active)
        };
        if result.is_ok() && !was_active {
            self.timer_revision = self
                .timer_revision
                .checked_add(1)
                .expect("timer revisions exhausted");
        }
        if let Err(error) = result {
            self.arm_failed(
                app,
                run,
                Failure::new(ErrorCode::AwakeActivationFailed, error),
            );
            return;
        }
        self.publish_awake(app);
        let plan = match self.controller.route_plan(run) {
            Ok(plan) => plan,
            Err(error) => {
                self.arm_failed(app, run, error);
                return;
            }
        };
        let movement = match self
            .session
            .as_ref()
            .ok_or_else(|| Failure::new(ErrorCode::NativeUnavailable, "No input observer"))
            .and_then(|s| s.movement(&plan.points))
        {
            Ok(m) => m,
            Err(e) => {
                self.arm_failed(app, run, e);
                return;
            }
        };
        self.worker_started = true;
        let handle = app.clone();
        let gate = self.gate.clone();
        thread::spawn(move || {
            control.attach_worker();
            let mut movement = movement;
            let prepared = movement.prepare(&plan.points).and_then(|origin| {
                // A warm Setup observer must publish a fresh observation for this reserved run.
                // Wait off the UI thread, before any cursor call; a pre-reservation lease cannot authorize it.
                let started = Instant::now();
                while !control.healthy(Instant::now()) {
                    if control.reason().is_some() {
                        return Err(Failure::new(
                            ErrorCode::StartCanceled,
                            "Patrol Start was canceled",
                        ));
                    }
                    if started.elapsed() >= Duration::from_millis(PREPARATION_MS) {
                        return Err(Failure::new(
                            ErrorCode::NativeUnavailable,
                            "The input observer did not confirm this Start",
                        ));
                    }
                    thread::park_timeout(Duration::from_millis(HEALTH_MS));
                }
                Ok(origin)
            });
            let (tx, rx) = mpsc::channel();
            let main_control = control.clone();
            main(&handle, move |app, core| {
                let result =
                    prepared.and_then(|origin| core.commit(app, run, origin, &main_control));
                let _ = tx.send(result);
            });
            let outcome = rx.recv().unwrap_or_else(|_| {
                Err(Failure::new(
                    ErrorCode::StartCanceled,
                    "The application closed",
                ))
            });
            let reason = match outcome {
                Ok(origin) => {
                    let release_handle = handle.clone();
                    engine::run_worker(plan, origin, movement, control.clone(), gate, move || {
                        main(&release_handle, |app, core| core.flush_awake(app))
                    })
                }
                Err(error) => {
                    main(&handle, move |_, core| {
                        core.failure(error.clone());
                        core.worker_failure = Some((run, error));
                    });
                    control.cancel(StopReason::NativeFailure);
                    control.acknowledge();
                    control.reason().unwrap_or(StopReason::NativeFailure)
                }
            };
            main(&handle, move |app, core| core.completed(app, run, reason));
        });
    }
    fn commit(
        &mut self,
        app: &tauri::AppHandle,
        run: Id,
        origin: XY,
        control: &Arc<RunControl>,
    ) -> Result<XY, Failure> {
        if !self.controller.arming(run) || control.reason().is_some() {
            return Err(Failure::new(
                ErrorCode::StartCanceled,
                "Patrol Start was canceled",
            ));
        }
        if !self.awake.is_empty() || self.quitting.is_some() {
            return Err(Failure::new(
                ErrorCode::Busy,
                "An Awake transition is pending",
            ));
        }
        self.publish_awake(app);
        let available = platform::availability();
        if available.state != AvailabilityState::Ready {
            return Err(Failure::new(
                ErrorCode::NativeUnavailable,
                available.message.unwrap_or_default(),
            ));
        }
        control.publish_availability(run, Instant::now());
        if !control.healthy(Instant::now()) {
            return Err(Failure::new(
                ErrorCode::NativeUnavailable,
                "The input observer did not confirm its readiness",
            ));
        }
        // Recheck the current fingerprint after worker preflight and before accepting the route.
        self.refresh_displays(app, false, None);
        if !self.controller.arming(run) || control.reason().is_some() {
            return Err(Failure::new(
                ErrorCode::StartCanceled,
                "The display layout changed",
            ));
        }
        // Destroy before commit: failures cannot leave a falsely Running patrol behind a desktop overlay.
        self.destroying = true;
        let result = self.destroy(app);
        self.destroying = false;
        if let Err(error) = result {
            self.rebuild = true;
            return Err(error);
        }
        self.publish_awake(app);
        if control.reason().is_some() || !control.healthy(Instant::now()) {
            self.rebuild = true;
            return Err(Failure::new(
                ErrorCode::StartCanceled,
                "Patrol Start lost its input authorization",
            ));
        }
        let receipt = match self.controller.commit(run) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.rebuild = true;
                return Err(error);
            }
        };
        {
            let mut gate = self.gate.state.lock().expect("admission");
            gate.armed = true;
            gate.transition_pending = false;
        }
        if let Some(router) = &self.router {
            router.committed(run);
        }
        self.resolve_start(run, Ok(receipt));
        Ok(origin)
    }
    fn arm_failed(&mut self, app: &tauri::AppHandle, run: Id, error: Failure) {
        if let Some(control) = &self.control {
            control.cancel(StopReason::NativeFailure);
            control.acknowledge();
        }
        let _ = self.controller.start_failed(run, error.clone());
        self.resolve_start(run, Err(error));
        if let Some(router) = &self.router {
            router.clear_run(run);
        }
        self.control = None;
        self.worker_started = false;
        if self.hidden {
            if let Err(error) = overlays::show(app, &self.controller.labels()) {
                self.failure(error);
                self.rebuild = true;
            }
            self.hidden = false;
        }
    }
    fn completed(&mut self, app: &tauri::AppHandle, run: Id, reason: StopReason) {
        let Some(control) = self.control.as_ref().filter(|c| c.run_id == run).cloned() else {
            return;
        };
        if !control.is_acknowledged() {
            return;
        }
        if let Some(elapsed) = control.stop_duration() {
            eprintln!(
                "Patrol native stop acknowledged in {} ms",
                elapsed.as_millis()
            );
        }
        if control.saw_display_change() {
            self.controller.review_required = true;
        }
        let specific = self
            .worker_failure
            .take()
            .filter(|(generation, _)| *generation == run)
            .map(|(_, error)| error);
        let failure = specific.or_else(|| match reason {
            StopReason::CursorConstrained => Some(Failure::new(
                ErrorCode::CursorConstrained,
                "Another application constrained the cursor. Release the constraint, review the route and start again",
            )),
            StopReason::NativeFailure | StopReason::ObserverLost |
            StopReason::AvailabilityExpired | StopReason::ExecutionStalled => Some(Failure::new(
                ErrorCode::NativeUnavailable,
                "Patrol stopped because native input or movement was unavailable. Reopen setup to retry",
            )),
            _ => None,
        });
        let awake_failure = self
            .controller
            .status
            .last_failure
            .as_ref()
            .filter(|failure| failure.code == ErrorCode::AwakeDeactivationFailed)
            .cloned();
        let start_failure = self.controller.stopped(run, reason, failure).ok().flatten();
        // Preserve sleep-prevention shutdown failure independently of the original Start result.
        if let Some(error) = awake_failure {
            self.failure(error);
        }
        if let Some(error) = start_failure {
            self.resolve_start(run, Err(error));
        }
        if let Some(router) = &self.router {
            router.clear_run(run);
        }
        self.control = None;
        self.worker_started = false;
        self.gate.state.lock().expect("admission").armed = false;
        if self.controller.setup.is_none() {
            self.session_close(app);
        } else if self.hidden && !self.controller.closing_setup {
            if let Err(error) = overlays::show(app, &self.controller.labels()) {
                self.failure(error);
                self.rebuild = true;
            }
            self.hidden = false;
        }
        if reason == StopReason::TimerExpired {
            self.awake.push_back(AwakeWork {
                op: AwakeOp::Expire(self.timer_revision),
                reply: None,
            });
            self.gate.defer_transition();
        }
    }
    fn flush_awake(&mut self, app: &tauri::AppHandle) {
        if self.awake.is_empty() {
            return;
        }
        if !self.gate.transition_ready(self.control.as_deref()) {
            return;
        }
        while let Some(work) = self.awake.pop_front() {
            let managed = app.state::<Mutex<crate::AppState>>();
            let mut state = managed.lock().expect("Awake");
            let was_active = state.no_sleep_active;
            let result = match work.op {
                AwakeOp::Activate => state.activate(),
                AwakeOp::Deactivate => state.deactivate(),
                AwakeOp::Toggle => {
                    if state.no_sleep_active {
                        state.deactivate()
                    } else {
                        state.activate()
                    }
                }
                AwakeOp::SetTimer(ms) => state.set_timer(Duration::from_millis(ms)),
                AwakeOp::CancelTimer => {
                    state.timer_duration = None;
                    Ok(())
                }
                AwakeOp::Expire(revision) => {
                    if revision == self.timer_revision {
                        state.expire_timer(Instant::now())
                    } else {
                        Ok(())
                    }
                }
            };
            if result.is_ok() && (!matches!(work.op, AwakeOp::Activate) || !was_active) {
                self.timer_revision = self
                    .timer_revision
                    .checked_add(1)
                    .expect("timer revisions exhausted");
            }
            let inactive = !state.no_sleep_active;
            drop(state);
            if result.is_ok() && inactive {
                let _ = self
                    .controller
                    .clear_failure(ErrorCode::AwakeDeactivationFailed);
            }
            if let Err(error) = &result {
                let code = if matches!(work.op, AwakeOp::Deactivate | AwakeOp::Expire(_))
                    || matches!(work.op, AwakeOp::Toggle) && was_active
                {
                    ErrorCode::AwakeDeactivationFailed
                } else {
                    ErrorCode::AwakeActivationFailed
                };
                self.failure(Failure::new(code, error));
            }
            if let Some(reply) = work.reply {
                let _ = reply.send(result);
            }
        }
        self.publish_awake(app);
        self.gate
            .state
            .lock()
            .expect("admission")
            .transition_pending = false;
        if let Some(control) = &self.control {
            control.wake();
        }
        self.tray_dirty = true;
    }
    fn tick(&mut self, app: &tauri::AppHandle) {
        if let Ok(effects) = self.controller.expire_preparation(Instant::now()) {
            self.effects(app, effects);
        }
        if self.controller.setup.is_some() || self.control.is_some() {
            let mut available = platform::availability();
            if available.state == AvailabilityState::Ready {
                available = match self.session.as_ref().map(|s| s.status()) {
                    Some(SessionState::Ready) => available,
                    Some(SessionState::Failed(error)) => Availability {
                        state: AvailabilityState::NativeUnavailable,
                        message: Some(error.message),
                    },
                    Some(SessionState::Closed) => Availability {
                        state: AvailabilityState::NativeUnavailable,
                        message: Some("Input monitoring stopped; reopen setup to retry".into()),
                    },
                    _ => Availability {
                        state: AvailabilityState::Checking,
                        message: Some("Preparing input monitoring…".into()),
                    },
                };
            }
            if let Some(control) = &self.control {
                if available.state == AvailabilityState::Ready {
                    control.publish_availability(control.run_id, Instant::now());
                } else {
                    control.cancel(
                        if available.state == AvailabilityState::PermissionRequired {
                            StopReason::PermissionLost
                        } else {
                            StopReason::SessionInactive
                        },
                    );
                }
            }
            let _ = self.controller.publish_availability(available);
            if self.last_display.elapsed() >= Duration::from_millis(DISPLAY_CHECK_MS) {
                self.refresh_displays(app, false, None);
            }
        }
        self.publish_awake(app);
        let expired = {
            self.gate
                .state
                .lock()
                .expect("admission")
                .deadline
                .as_ref()
                .filter(|d| d.expired(Instant::now()))
                .map(|d| d.revision)
        };
        if let Some(expired_revision) = expired.filter(|_| {
            !self
                .awake
                .iter()
                .any(|w| matches!(w.op, AwakeOp::Expire(_)))
        }) {
            self.cancel(StopReason::TimerExpired);
            self.gate.defer_transition();
            self.awake.push_back(AwakeWork {
                op: AwakeOp::Expire(expired_revision),
                reply: None,
            });
        }
        if let Some(control) = self.control.clone() {
            if let Some(reason) = control.reason() {
                if self.controller.status.phase != Phase::Stopping {
                    self.cancel(reason);
                }
                if control.is_acknowledged() {
                    self.completed(app, control.run_id, reason);
                }
            }
        }
    }
    fn publication_changed(&mut self) -> bool {
        let next = self
            .controller
            .setup_id()
            .map(|setup_id| (setup_id, self.controller.snapshot("").state_version));
        if self.last_published == next {
            return false;
        }
        self.last_published = next;
        true
    }
    fn finish(&mut self, app: &tauri::AppHandle) {
        self.flush_awake(app);
        self.try_close(app);
        if self.rebuild
            && self.controller.status.run_id.is_none()
            && !self.controller.closing_setup
            && !self.creating
        {
            self.destroying = true;
            let cleanup = self.destroy(app);
            self.destroying = false;
            if let Err(error) = cleanup {
                self.failure(error);
            } else if let Ok(setup) = self.controller.rebuild() {
                self.rebuild = false;
                self.session_open(app, setup);
                self.create_overlays(app);
            }
        }
        let mut waiting = vec![];
        for (run, label, tx, at) in std::mem::take(&mut self.stops) {
            if self.controller.status.run_id != Some(run) {
                let _ = tx.send(Ok(self.reply(&label, None)));
            } else if at.elapsed() >= Duration::from_secs(2) {
                let _ = tx.send(Err(self.error(
                    &label,
                    Failure::new(
                        ErrorCode::StopPending,
                        "A native call is still finishing; patrol remains Stopping",
                    ),
                )));
            } else {
                waiting.push((run, label, tx, at));
            }
        }
        self.stops = waiting;
        if self.publication_changed() {
            overlays::publish(app, &self.controller);
        }
        if self.menu_phase != Some(self.controller.status.phase) {
            self.menu_phase = Some(self.controller.status.phase);
            self.tray_dirty = true;
        }
        if self.quitting.is_some_and(|at| {
            at.elapsed() >= Duration::from_secs(2)
                || (self.controller.status.run_id.is_none()
                    && !self.cleaning
                    && !self.creating
                    && !self.controller.closing_setup)
        }) {
            self.exit_allowed = true;
            if self.exit_code == tauri::RESTART_EXIT_CODE {
                app.request_restart();
            } else {
                app.exit(self.exit_code);
            }
        }
    }
    fn active(&self) -> bool {
        self.controller.setup.is_some()
            || self.control.is_some()
            || self.cleaning
            || self.creating
            || !self.awake.is_empty()
            || self.quitting.is_some()
    }
}
pub fn setup(app: &tauri::AppHandle) {
    let (dispatch, commands) = mpsc::channel::<Work>();
    app.manage(PatrolRuntime::new(dispatch));
    let dispatcher = app.clone();
    thread::spawn(move || {
        for work in commands {
            let handle = dispatcher.clone();
            if dispatcher
                .run_on_main_thread(move || run_work(&handle, work))
                .is_err()
            {
                break;
            }
        }
    });
    let handle = app.clone();
    let wake = runtime(app).wake.clone();
    thread::spawn(move || loop {
        let (lock, cv) = &*wake;
        let mut pending = lock.lock().expect("patrol wake");
        while !*pending {
            pending = cv.wait(pending).expect("patrol wake");
        }
        *pending = false;
        drop(pending);
        let state = runtime(&handle);
        if state.tick_queued.swap(true, Ordering::AcqRel) {
            continue;
        }
        let app = handle.clone();
        if handle
            .run_on_main_thread(move || {
                let state = runtime(&app);
                state.tick_queued.store(false, Ordering::Release);
                let mut core = state.core.lock().expect("patrol");
                core.tick(&app);
                core.finish(&app);
                let active = core.active();
                let dirty = std::mem::take(&mut core.tray_dirty);
                drop(core);
                if dirty {
                    crate::refresh_tray(&app);
                }
                if active {
                    state.wake();
                }
            })
            .is_err()
        {
            break;
        }
        thread::sleep(Duration::from_millis(HEALTH_MS));
    });
}
pub fn open(app: &tauri::AppHandle) {
    main(app, |app, core| core.open(app));
}
pub fn stop(app: &tauri::AppHandle) {
    main(app, |_, core| core.cancel(StopReason::TrayStop));
}
pub fn queue_awake(
    app: &tauri::AppHandle,
    op: AwakeOp,
    reply: Option<mpsc::Sender<Result<(), String>>>,
) {
    main(app, move |app, core| {
        let off = matches!(op, AwakeOp::Deactivate | AwakeOp::Expire(_))
            || matches!(op, AwakeOp::Toggle)
                && app
                    .state::<Mutex<crate::AppState>>()
                    .lock()
                    .expect("Awake")
                    .no_sleep_active;
        if off {
            core.cancel(if matches!(op, AwakeOp::Expire(_)) {
                StopReason::TimerExpired
            } else {
                StopReason::AwakeOff
            });
        }
        core.gate.defer_transition();
        core.awake.push_back(AwakeWork { op, reply });
        core.flush_awake(app);
    });
}
pub async fn awake(app: tauri::AppHandle, op: AwakeOp) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    queue_awake(&app, op, Some(tx));
    tauri::async_runtime::spawn_blocking(move || {
        rx.recv()
            .unwrap_or_else(|_| Err("Application closed".into()))
    })
    .await
    .map_err(|e| e.to_string())?
}
pub fn refresh(app: &tauri::AppHandle) {
    main(app, |app, core| core.tick(app));
}
pub fn window_closed(app: &tauri::AppHandle, label: &str) -> bool {
    close_event(app, label, false)
}
pub fn window_destroyed(app: &tauri::AppHandle, label: &str) {
    close_event(app, label, true);
}
fn close_event(app: &tauri::AppHandle, label: &str, unexpected: bool) -> bool {
    if !label.starts_with("patrol-") {
        return false;
    }
    let label = label.to_owned();
    main(app, move |app, core| {
        if core.expected_destroys.remove(&label) {
            return;
        }
        if core.controller.labels().contains(&label) && !core.destroying {
            if unexpected {
                core.fail_setup(
                    app,
                    Failure::new(
                        ErrorCode::OverlayFailed,
                        "A setup window disappeared; reopen Cursor Patrolling",
                    ),
                );
            } else {
                core.close(app, None);
            }
        }
    });
    true
}
pub fn page_loaded(app: &tauri::AppHandle, label: &str) {
    let label = label.to_owned();
    main(app, move |app, core| {
        if core.controller.page_started(&label) {
            core.fail_setup(
                app,
                Failure::new(
                    ErrorCode::OverlayFailed,
                    "A setup window reloaded; reopen Cursor Patrolling",
                ),
            );
        }
    });
}
pub fn quit(app: &tauri::AppHandle) {
    request_exit(app, 0);
}
pub fn request_exit(app: &tauri::AppHandle, code: i32) {
    main(app, move |app, core| {
        if core.quitting.is_none() {
            core.quitting = Some(Instant::now());
            core.exit_code = code;
            core.cancel(StopReason::AppExit);
            if core.controller.setup.is_some() {
                core.close(app, None);
            } else if core.control.is_none() {
                core.session_close(app);
            }
        }
    });
}
pub fn exit_allowed(app: &tauri::AppHandle) -> bool {
    runtime(app).core.lock().expect("patrol").exit_allowed
}
pub fn menu_text(app: &tauri::AppHandle) -> (&'static str, bool) {
    if !cfg!(any(target_os = "macos", windows)) {
        return ("Cursor Patrolling (macOS / Windows)", false);
    }
    let state = runtime(app);
    let core = state.core.lock().expect("patrol");
    match core.controller.status.phase {
        Phase::Running => ("Stop Cursor Patrolling", true),
        Phase::Starting => ("Stop Cursor Patrolling (starting…)", true),
        Phase::Stopping => ("Cursor Patrolling (stopping…)", false),
        Phase::ReviewRequired => ("Cursor Patrolling — Review Points…", true),
        _ => ("Cursor Patrolling…", true),
    }
}
pub fn menu(app: &tauri::AppHandle) {
    let active = runtime(app)
        .core
        .lock()
        .expect("patrol")
        .controller
        .status
        .run_id
        .is_some();
    if active {
        stop(app)
    } else {
        open(app)
    }
}
fn parse<T: serde::de::DeserializeOwned>(request: tauri::ipc::Request<'_>) -> Result<T, Failure> {
    match request.body() {
        tauri::ipc::InvokeBody::Json(value) => serde_json::from_value(value.clone())
            .map_err(|_| Failure::new(ErrorCode::InvalidPayload, "Invalid cursor patrol request")),
        _ => Err(Failure::new(
            ErrorCode::InvalidPayload,
            "Expected a JSON object",
        )),
    }
}
async fn receive(rx: mpsc::Receiver<Response>) -> Response {
    tauri::async_runtime::spawn_blocking(move || {
        rx.recv().unwrap_or_else(|_| {
            Err(PatrolError {
                code: ErrorCode::InternalError,
                message: "Application closed".into(),
                snapshot: None,
            })
        })
    })
    .await
    .unwrap_or_else(|e| {
        Err(PatrolError {
            code: ErrorCode::InternalError,
            message: e.to_string(),
            snapshot: None,
        })
    })
}
fn command(
    app: &tauri::AppHandle,
    label: String,
    work: impl FnOnce(&tauri::AppHandle, &mut Core, &str, Sender) + Send + 'static,
) -> mpsc::Receiver<Response> {
    let (tx, rx) = mpsc::channel();
    main(app, move |app, core| {
        if !core.controller.has_source(&label) {
            let _ = tx.send(Err(core.error(
                &label,
                Failure::new(
                    ErrorCode::StaleSetup,
                    "This setup window is no longer registered",
                ),
            )));
            return;
        }
        if let Ok(effects) = core.controller.expire_preparation(Instant::now()) {
            core.effects(app, effects);
        }
        work(app, core, &label, tx);
    });
    rx
}
#[tauri::command]
pub async fn get_patrol_setup(window: WebviewWindow, request: tauri::ipc::Request<'_>) -> Response {
    let input = parse::<EmptyInput>(request);
    let rx = command(
        window.app_handle(),
        window.label().into(),
        move |_, core, label, tx| {
            let answer = input
                .and_then(|_| core.controller.get(label))
                .map_err(|e| core.error(label, e));
            let _ = tx.send(answer);
        },
    );
    receive(rx).await
}
#[tauri::command]
pub async fn update_patrol_setup(
    window: WebviewWindow,
    request: tauri::ipc::Request<'_>,
) -> Response {
    let input = parse::<UpdateInput>(request);
    let rx = command(
        window.app_handle(),
        window.label().into(),
        move |app, core, label, tx| {
            let answer =
                input.and_then(|input| core.controller.update(label, input, Instant::now()));
            match answer {
                Ok(update) => {
                    let reply = core.reply(label, Some(update.receipt));
                    let _ = tx.send(Ok(reply));
                    core.effects(app, update.effects);
                }
                Err(error) => {
                    if error.code == ErrorCode::Busy {
                        if let Ok(effects) = core.controller.reject_late_edit(label) {
                            core.effects(app, effects);
                        }
                    }
                    let _ = tx.send(Err(core.error(label, error)));
                }
            }
        },
    );
    receive(rx).await
}
#[tauri::command]
pub async fn start_patrol(window: WebviewWindow, request: tauri::ipc::Request<'_>) -> Response {
    let input = parse::<StartInput>(request);
    let rx = command(
        window.app_handle(),
        window.label().into(),
        move |_, core, label, tx| {
            if !core.awake.is_empty() || core.cleaning || core.creating || core.quitting.is_some() {
                let _ = tx.send(Err(core.error(
                    label,
                    Failure::new(ErrorCode::Busy, "An application transition is pending"),
                )));
                return;
            }
            match input.and_then(|input| {
                core.controller
                    .start(label, input, Instant::now(), platform::buttons_down())
            }) {
                Ok(StartDisposition::Cached(result)) => {
                    let answer = result
                        .map(|r| core.reply(label, Some(r)))
                        .map_err(|e| core.error(label, e));
                    let _ = tx.send(answer);
                }
                Ok(StartDisposition::Pending(run)) => {
                    core.starts.entry(run).or_default().push((label.into(), tx));
                    if core.control.is_none() {
                        let control = RunControl::new(run);
                        if !core
                            .router
                            .as_ref()
                            .is_some_and(|r| r.reserve_run(control.clone()))
                        {
                            control.cancel(StopReason::Escape);
                        }
                        core.control = Some(control);
                    }
                }
                Err(error) => {
                    let _ = tx.send(Err(core.error(label, error)));
                }
            }
        },
    );
    receive(rx).await
}
#[tauri::command]
pub async fn cancel_patrol_setup(
    window: WebviewWindow,
    request: tauri::ipc::Request<'_>,
) -> Response {
    let input = parse::<SetupInput>(request);
    let rx = command(
        window.app_handle(),
        window.label().into(),
        move |app, core, label, tx| match input
            .and_then(|i| core.controller.validate_setup(label, i.setup_id))
        {
            Ok(()) => core.close(app, Some((label.into(), tx))),
            Err(e) => {
                let _ = tx.send(Err(core.error(label, e)));
            }
        },
    );
    receive(rx).await
}
#[tauri::command]
pub async fn stop_patrol(window: WebviewWindow, request: tauri::ipc::Request<'_>) -> Response {
    let input = parse::<StopInput>(request);
    let rx = command(
        window.app_handle(),
        window.label().into(),
        move |app, core, label, tx| match input.and_then(|i| {
            core.controller
                .request_stop(i.run_id, StopReason::TrayStop)
                .map(|effects| (i.run_id, effects))
        }) {
            Ok((run, effects)) => {
                core.effects(app, effects);
                core.stops.push((run, label.into(), tx, Instant::now()));
            }
            Err(e) => {
                let _ = tx.send(Err(core.error(label, e)));
            }
        },
    );
    receive(rx).await
}
#[tauri::command]
pub async fn request_patrol_permissions(
    window: WebviewWindow,
    request: tauri::ipc::Request<'_>,
) -> Response {
    let input = parse::<SetupInput>(request);
    let rx = command(
        window.app_handle(),
        window.label().into(),
        move |app, core, label, tx| {
            let result = input
                .and_then(|i| core.controller.validate_setup(label, i.setup_id))
                .and_then(|_| core.controller.is_control(label))
                .and_then(|_| {
                    if core.controller.status.run_id.is_some() {
                        Err(Failure::new(
                            ErrorCode::Busy,
                            "Stop patrol before requesting input access",
                        ))
                    } else {
                        overlays::hide(app, &core.controller.labels())
                    }
                });
            match result {
                Ok(()) => {
                    core.hidden = true;
                    core.session_close(app);
                    let available = platform::request_permissions();
                    let _ = core.controller.publish_availability(available);
                    let _ = tx.send(Ok(core.reply(label, None)));
                }
                Err(error) => {
                    let _ = tx.send(Err(core.error(label, error)));
                }
            }
        },
    );
    receive(rx).await
}

/// Keep the process plugin's command names and capability boundary, but finish native movement first.
pub fn process_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("process")
        .invoke_handler(tauri::generate_handler![
            process_commands::exit,
            process_commands::restart
        ])
        .build()
}
mod process_commands {
    #[tauri::command]
    pub fn exit(app: tauri::AppHandle, code: i32) {
        super::request_exit(&app, code);
    }
    #[tauri::command]
    pub fn restart(app: tauri::AppHandle) {
        super::request_exit(&app, tauri::RESTART_EXIT_CODE);
    }
}
/// Tauri cannot defer its reserved restart exit code (including installer-owned restarts).
/// Invalidate movement synchronously before the framework proceeds with that process exit.
pub fn invalidate_for_exit(app: &tauri::AppHandle) {
    let state = runtime(app);
    let mut core = state.core.lock().expect("patrol");
    core.cancel(StopReason::AppExit);
    if let Some(router) = &core.router {
        router.disable();
    }
    if core.control.is_none() {
        core.session_close(app);
    }
}
