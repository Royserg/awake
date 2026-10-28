//! Deterministic motion and the cancellation/publication boundary shared with native workers.
use super::model::*;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread::{self, Thread},
    time::{Duration, Instant},
};

const NO_REASON: u8 = u8::MAX;
const REASONS: [StopReason; 16] = [
    StopReason::Escape,
    StopReason::ManualMovement,
    StopReason::MouseButton,
    StopReason::TrayStop,
    StopReason::AwakeOff,
    StopReason::TimerExpired,
    StopReason::DisplayChanged,
    StopReason::SessionInactive,
    StopReason::SecureInput,
    StopReason::ObserverLost,
    StopReason::AvailabilityExpired,
    StopReason::PermissionLost,
    StopReason::CursorConstrained,
    StopReason::NativeFailure,
    StopReason::ExecutionStalled,
    StopReason::AppExit,
];

pub struct RunControl {
    pub run_id: Id,
    origin: Instant,
    cancelled: AtomicBool,
    reason: AtomicU8,
    observed_ns: AtomicU64,
    acknowledged_ns: AtomicU64,
    acknowledged: AtomicBool,
    display_changed: AtomicBool,
    worker: OnceLock<Thread>,
    observer: Mutex<Option<Instant>>,
    availability: Mutex<Option<Instant>>,
}
impl RunControl {
    pub fn new(run_id: Id) -> Arc<Self> {
        Arc::new(Self {
            run_id,
            origin: Instant::now(),
            cancelled: AtomicBool::new(false),
            reason: AtomicU8::new(NO_REASON),
            observed_ns: AtomicU64::new(0),
            acknowledged_ns: AtomicU64::new(0),
            acknowledged: AtomicBool::new(false),
            display_changed: AtomicBool::new(false),
            worker: OnceLock::new(),
            observer: Mutex::new(None),
            availability: Mutex::new(None),
        })
    }
    pub fn attach_worker(&self) {
        let _ = self.worker.set(thread::current());
    }
    pub fn wake(&self) {
        if let Some(worker) = self.worker.get() {
            worker.unpark();
        }
    }
    pub fn cancel(&self, reason: StopReason) {
        if reason == StopReason::DisplayChanged {
            self.display_changed.store(true, Ordering::Release);
        }
        // Reserve the first record, then publish a complete timestamp/reason before cancellation.
        let stamp = self.origin.elapsed().as_nanos().min((u64::MAX - 1) as u128) as u64 + 1;
        if self
            .observed_ns
            .compare_exchange(0, stamp, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let code = REASONS
                .iter()
                .position(|r| *r == reason)
                .expect("complete stop reason table") as u8;
            self.reason.store(code, Ordering::Relaxed);
            self.cancelled.store(true, Ordering::Release);
        }
        self.wake();
    }
    pub fn reason(&self) -> Option<StopReason> {
        if !self.cancelled.load(Ordering::Acquire) {
            return None;
        }
        REASONS
            .get(self.reason.load(Ordering::Relaxed) as usize)
            .copied()
    }
    pub fn saw_display_change(&self) -> bool {
        self.display_changed.load(Ordering::Acquire)
    }
    pub fn publish_observer(&self, generation: Id, at: Instant) {
        if generation == self.run_id && self.reason().is_none() {
            *self.observer.lock().expect("observer timestamp") = Some(at);
            self.wake();
        }
    }
    pub fn publish_availability(&self, generation: Id, at: Instant) {
        if generation == self.run_id && self.reason().is_none() {
            *self.availability.lock().expect("availability timestamp") = Some(at);
            self.wake();
        }
    }
    fn leases(&self) -> (Option<Instant>, Option<Instant>) {
        (
            *self.observer.lock().expect("observer timestamp"),
            *self.availability.lock().expect("availability timestamp"),
        )
    }
    pub fn eligibility(&self, now: Instant) -> Result<(), StopReason> {
        if let Some(reason) = self.reason() {
            return Err(reason);
        }
        let (observer, availability) = self.leases();
        if observer
            .is_none_or(|at| now.saturating_duration_since(at) >= Duration::from_millis(LEASE_MS))
        {
            return Err(StopReason::ObserverLost);
        }
        if availability
            .is_none_or(|at| now.saturating_duration_since(at) >= Duration::from_millis(LEASE_MS))
        {
            return Err(StopReason::AvailabilityExpired);
        }
        Ok(())
    }
    pub fn healthy(&self, now: Instant) -> bool {
        self.eligibility(now).is_ok()
    }
    pub fn next_health_wait(&self, now: Instant) -> Duration {
        let (observer, availability) = self.leases();
        [observer, availability]
            .into_iter()
            .map(|stamp| {
                stamp.map_or(Duration::ZERO, |at| {
                    Duration::from_millis(LEASE_MS)
                        .saturating_sub(now.saturating_duration_since(at))
                })
            })
            .min()
            .unwrap_or(Duration::ZERO)
            .min(Duration::from_millis(HEALTH_MS))
    }
    pub fn acknowledge(&self) {
        self.acknowledged_ns.store(
            self.origin.elapsed().as_nanos().min(u64::MAX as u128) as u64,
            Ordering::Relaxed,
        );
        self.acknowledged.store(true, Ordering::Release);
        self.wake();
    }
    pub fn is_acknowledged(&self) -> bool {
        self.acknowledged.load(Ordering::Acquire)
    }
    pub fn stop_duration(&self) -> Option<Duration> {
        if !self.is_acknowledged() {
            return None;
        }
        let observed = self.observed_ns.load(Ordering::Acquire).checked_sub(1)?;
        Some(Duration::from_nanos(
            self.acknowledged_ns
                .load(Ordering::Relaxed)
                .saturating_sub(observed),
        ))
    }
}

#[derive(Clone, Debug)]
pub struct Deadline {
    pub revision: u64,
    pub started_at: Instant,
    pub duration: Duration,
}
impl Deadline {
    pub fn remaining(&self, now: Instant) -> Duration {
        self.duration
            .saturating_sub(now.saturating_duration_since(self.started_at))
    }
    pub fn expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started_at) >= self.duration
    }
}
pub struct GateState {
    pub awake_active: bool,
    pub deadline: Option<Deadline>,
    pub layout_revision: Id,
    pub emitting: bool,
    pub transition_pending: bool,
    pub armed: bool,
}
pub struct EmissionGate {
    pub state: Mutex<GateState>,
}
impl EmissionGate {
    pub fn new(layout_revision: Id) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(GateState {
                awake_active: false,
                deadline: None,
                layout_revision,
                emitting: false,
                transition_pending: false,
                armed: false,
            }),
        })
    }
    pub fn check(&self, control: &RunControl, layout: Id, now: Instant) -> Result<(), StopReason> {
        control.eligibility(now)?;
        let gate = self.state.lock().expect("emission admission");
        if !gate.awake_active {
            return Err(StopReason::AwakeOff);
        }
        if gate.layout_revision != layout {
            return Err(StopReason::DisplayChanged);
        }
        if gate.deadline.as_ref().is_some_and(|d| d.expired(now)) {
            return Err(StopReason::TimerExpired);
        }
        Ok(())
    }
    /// Sample time after all state copies/locks, including any native port prechecks.
    pub fn check_now(
        &self,
        control: &RunControl,
        layout: Id,
        clock: &dyn Fn() -> Instant,
    ) -> Result<(), StopReason> {
        let (observer, availability) = control.leases();
        let (active, revision, deadline) = {
            let gate = self.state.lock().expect("emission admission");
            (
                gate.awake_active,
                gate.layout_revision,
                gate.deadline.clone(),
            )
        };
        let now = clock();
        if let Some(reason) = control.reason() {
            return Err(reason);
        }
        if observer
            .is_none_or(|at| now.saturating_duration_since(at) >= Duration::from_millis(LEASE_MS))
        {
            return Err(StopReason::ObserverLost);
        }
        if availability
            .is_none_or(|at| now.saturating_duration_since(at) >= Duration::from_millis(LEASE_MS))
        {
            return Err(StopReason::AvailabilityExpired);
        }
        if !active {
            return Err(StopReason::AwakeOff);
        }
        if revision != layout {
            return Err(StopReason::DisplayChanged);
        }
        if deadline.is_some_and(|d| d.expired(now)) {
            return Err(StopReason::TimerExpired);
        }
        Ok(())
    }
    /// The main actor closes admission and defers its state transition while a call is reserved.
    /// It never waits on a native call while holding AppState or running the UI thread.
    /// Called on the main actor after closing admission. No native call or UI wait is under this lock.
    pub fn transition_ready(&self, control: Option<&RunControl>) -> bool {
        !self.state.lock().expect("emission admission").emitting
            && !control
                .is_some_and(|control| control.reason().is_some() && !control.is_acknowledged())
    }
    pub fn defer_transition(&self) -> bool {
        let mut gate = self.state.lock().expect("emission admission");
        gate.transition_pending = true;
        gate.emitting
    }
    pub fn reserve(
        self: &Arc<Self>,
        control: &RunControl,
        layout: Id,
        now: Instant,
    ) -> Result<Option<Emission>, StopReason> {
        control.eligibility(now)?;
        let mut gate = self.state.lock().expect("emission admission");
        if gate.transition_pending || !gate.armed {
            return Ok(None);
        }
        if !gate.awake_active {
            return Err(StopReason::AwakeOff);
        }
        if gate.layout_revision != layout {
            return Err(StopReason::DisplayChanged);
        }
        if gate.deadline.as_ref().is_some_and(|d| d.expired(now)) {
            return Err(StopReason::TimerExpired);
        }
        assert!(!gate.emitting, "one native emitter per run");
        gate.emitting = true;
        Ok(Some(Emission { gate: self.clone() }))
    }
    pub fn deadline_wait(&self, now: Instant) -> Option<Duration> {
        self.state
            .lock()
            .expect("emission admission")
            .deadline
            .as_ref()
            .map(|d| d.remaining(now))
    }
}
pub struct Emission {
    gate: Arc<EmissionGate>,
}
impl Drop for Emission {
    fn drop(&mut self) {
        self.gate.state.lock().expect("emission admission").emitting = false;
    }
}

#[derive(Clone, Debug)]
pub struct RoutePlan {
    pub points: Vec<XY>,
    pub displays: Vec<Display>,
    pub pause_ms: u64,
    pub layout_revision: Id,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Decision {
    Wait(Duration),
    Emit(XY),
    Stopped(StopReason),
    InFlight,
}
#[derive(Clone, Copy, Debug)]
enum Motion {
    Travel {
        from: XY,
        target: usize,
        started: Instant,
    },
    Pause {
        point: usize,
        started: Instant,
    },
}
pub struct Engine {
    plan: RoutePlan,
    phase: Motion,
    last_wake: Instant,
    last_completed: Option<Instant>,
    pending: Option<(XY, bool)>,
    stopped: Option<StopReason>,
}
impl Engine {
    pub fn new(plan: RoutePlan, origin: XY, now: Instant) -> Result<Self, ErrorCode> {
        if !(MIN_POINTS..=MAX_POINTS).contains(&plan.points.len())
            || !pause_valid(plan.pause_ms)
            || !origin.valid()
            || plan.points.iter().any(|p| !p.valid())
            || !plan.points.iter().any(|p| *p != plan.points[0])
            || plan
                .points
                .iter()
                .any(|p| canonical_position(*p, &plan.displays).is_none())
        {
            return Err(ErrorCode::InvalidRoute);
        }
        let phase = if origin == plan.points[0] {
            Motion::Pause {
                point: 0,
                started: now,
            }
        } else {
            Motion::Travel {
                from: origin,
                target: 0,
                started: now,
            }
        };
        Ok(Self {
            plan,
            phase,
            last_wake: now,
            last_completed: None,
            pending: None,
            stopped: None,
        })
    }
    pub fn stop(&mut self, reason: StopReason) {
        if self.stopped.is_none() {
            self.stopped = Some(reason);
        }
    }
    pub fn poll(&mut self, now: Instant) -> Decision {
        if let Some(reason) = self.stopped {
            return Decision::Stopped(reason);
        }
        if self.pending.is_some() {
            return Decision::InFlight;
        }
        if now.saturating_duration_since(self.last_wake) > Duration::from_millis(STALL_MS) {
            self.stop(StopReason::ExecutionStalled);
            return Decision::Stopped(StopReason::ExecutionStalled);
        }
        self.last_wake = now;
        if let Motion::Pause { point, started } = self.phase {
            let elapsed = now.saturating_duration_since(started);
            let pause = Duration::from_millis(self.plan.pause_ms);
            if elapsed < pause {
                return Decision::Wait(pause - elapsed);
            }
            self.phase = Motion::Travel {
                from: self.plan.points[point],
                target: (point + 1) % self.plan.points.len(),
                started: now,
            };
        }
        if let Some(last) = self.last_completed {
            let elapsed = now.saturating_duration_since(last);
            let spacing = Duration::from_nanos(SAMPLE_NS);
            if elapsed < spacing {
                return Decision::Wait(spacing - elapsed);
            }
        }
        let Motion::Travel {
            from,
            target,
            started,
        } = self.phase
        else {
            unreachable!()
        };
        let t = (now.saturating_duration_since(started).as_secs_f64()
            / Duration::from_millis(TRAVEL_MS).as_secs_f64())
        .min(1.0);
        let eased = 3.0 * t * t - 2.0 * t * t * t;
        let endpoint = self.plan.points[target];
        let raw = if t >= 1.0 {
            endpoint
        } else {
            XY {
                x: from.x + (endpoint.x - from.x) * eased,
                y: from.y + (endpoint.y - from.y) * eased,
            }
        };
        let Some((_, position)) = canonical_position(raw, &self.plan.displays) else {
            if t >= 1.0 {
                self.stop(StopReason::DisplayChanged);
                return Decision::Stopped(StopReason::DisplayChanged);
            }
            return Decision::Wait(Duration::from_nanos(SAMPLE_NS));
        };
        self.pending = Some((position, t >= 1.0));
        Decision::Emit(position)
    }
    pub fn complete(&mut self, completed: Instant, success: bool) {
        let Some((_, endpoint)) = self.pending.take() else {
            return;
        };
        if !success {
            self.stop(StopReason::NativeFailure);
            return;
        }
        self.last_completed = Some(completed);
        if endpoint {
            let Motion::Travel { target, .. } = self.phase else {
                unreachable!()
            };
            self.phase = Motion::Pause {
                point: target,
                started: completed,
            };
        }
    }
}

/// Native code must recheck `before_call` after any OS prechecks and immediately before positioning.
pub trait Movement: Send {
    fn prepare(&mut self, route: &[XY]) -> Result<XY, Failure>;
    fn position(
        &mut self,
        point: XY,
        before_call: &dyn Fn() -> Result<(), StopReason>,
    ) -> Result<(), StopReason>;
}

pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    fn wait(&self, duration: Duration);
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
    fn wait(&self, duration: Duration) {
        thread::park_timeout(duration);
    }
}

pub fn run_worker(
    plan: RoutePlan,
    origin: XY,
    movement: Box<dyn Movement>,
    control: Arc<RunControl>,
    gate: Arc<EmissionGate>,
    gate_released: impl Fn() + Send + 'static,
) -> StopReason {
    run_worker_with_clock(
        plan,
        origin,
        movement,
        control,
        gate,
        gate_released,
        Arc::new(SystemClock),
    )
}

pub fn run_worker_with_clock(
    plan: RoutePlan,
    origin: XY,
    mut movement: Box<dyn Movement>,
    control: Arc<RunControl>,
    gate: Arc<EmissionGate>,
    gate_released: impl Fn() + Send + 'static,
    clock: Arc<dyn Clock>,
) -> StopReason {
    control.attach_worker();
    let layout = plan.layout_revision;
    let mut engine = match Engine::new(plan, origin, clock.now()) {
        Ok(engine) => engine,
        Err(_) => {
            control.cancel(StopReason::NativeFailure);
            control.acknowledge();
            return StopReason::NativeFailure;
        }
    };
    loop {
        if let Some(reason) = control.reason() {
            engine.stop(reason);
        }
        let now = clock.now();
        if let Err(reason) = gate.check(&control, layout, now) {
            control.cancel(reason);
            engine.stop(reason);
        }
        let decision = engine.poll(now);
        let wait = match decision {
            Decision::Stopped(reason) => {
                control.cancel(reason);
                break;
            }
            Decision::InFlight => {
                control.cancel(StopReason::NativeFailure);
                break;
            }
            Decision::Wait(wait) => wait,
            Decision::Emit(point) => match gate.reserve(&control, layout, clock.now()) {
                Err(reason) => {
                    control.cancel(reason);
                    break;
                }
                Ok(None) => {
                    // Admission is reserved for an ordered Awake transition. No native call occurred.
                    engine.pending = None;
                    Duration::from_millis(HEALTH_MS)
                }
                Ok(Some(emission)) => {
                    let result = movement.position(point, &|| {
                        gate.check_now(&control, layout, &|| clock.now())?;
                        if clock.now().saturating_duration_since(now)
                            > Duration::from_millis(STALL_MS)
                        {
                            return Err(StopReason::ExecutionStalled);
                        }
                        Ok(())
                    });
                    engine.complete(clock.now(), result.is_ok());
                    drop(emission);
                    if gate
                        .state
                        .lock()
                        .expect("emission admission")
                        .transition_pending
                    {
                        gate_released();
                    }
                    if let Err(reason) = result {
                        control.cancel(reason);
                    }
                    Duration::from_nanos(SAMPLE_NS)
                }
            },
        };
        if control.reason().is_some() {
            continue;
        }
        let now = clock.now();
        let wait = wait
            .min(control.next_health_wait(now))
            .min(gate.deadline_wait(now).unwrap_or(wait));
        clock.wait(wait);
    }
    // Returning from the synchronous position call and dropping its emission reservation precede this acknowledgment.
    control.acknowledge();
    control.reason().unwrap_or(StopReason::NativeFailure)
}
