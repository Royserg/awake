//! Pure setup protocol. Effects are executed by the main-thread runtime after releasing this state.
use super::{engine::RoutePlan, model::*};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
struct AcceptedAction {
    sequence: Id,
    action: Action,
    confirmation_pair: Option<ReviewPair>,
}
#[derive(Clone, Debug)]
struct CachedStart {
    request: StartInput,
    outcome: Option<Result<Id, Failure>>,
}
#[derive(Clone, Debug)]
struct Source {
    caller: Caller,
    bound_layout: Id,
    last_action: Option<AcceptedAction>,
    last_start: Option<CachedStart>,
    initialized: bool,
    document_loads: u32,
}
#[derive(Clone, Debug)]
struct Start {
    run_id: Id,
    label: String,
    request: StartInput,
    deadline: Instant,
    aborted: Option<Failure>,
}
#[derive(Clone, Debug)]
pub enum Effect {
    Arm(Id),
    StartFailed(Id, Failure),
    Stop(Id, StopReason),
}
#[derive(Debug)]
pub struct Update {
    pub receipt: Receipt,
    pub effects: Vec<Effect>,
}
#[derive(Debug)]
pub enum StartDisposition {
    Pending(Id),
    Cached(Result<Receipt, Failure>),
}
#[derive(Clone)]
pub struct Registration {
    pub label: String,
}

pub struct Controller {
    pub setup: Option<Setup>,
    pub status: Status,
    pub awake: Awake,
    pub availability: Availability,
    pub displays: Vec<Display>,
    pub layout_revision: Id,
    pub review_required: bool,
    pub closing_setup: bool,
    initialized_layout: bool,
    accepted_points: Vec<Point>,
    accepted_pause: u64,
    sources: BTreeMap<String, Source>,
    start: Option<Start>,
    last_stopped: Option<Id>,
    version: Id,
    route_revision: Id,
    versions: Ids,
    routes: Ids,
    layouts: Ids,
    setups: Ids,
    overlays: Ids,
    points: Ids,
    runs: Ids,
    preparations: Ids,
    display_ids: Ids,
}
impl Controller {
    pub fn new() -> Self {
        let mut versions = Ids::default();
        let mut routes = Ids::default();
        let mut layouts = Ids::default();
        Self {
            version: versions.next_id().expect("first version"),
            route_revision: routes.next_id().expect("first route"),
            layout_revision: layouts.next_id().expect("first layout"),
            versions,
            routes,
            layouts,
            setups: Ids::default(),
            overlays: Ids::default(),
            points: Ids::default(),
            runs: Ids::default(),
            preparations: Ids::default(),
            display_ids: Ids::default(),
            setup: None,
            status: Status {
                phase: Phase::Stopped,
                start_stage: None,
                run_id: None,
                stop_reason: None,
                last_failure: None,
            },
            awake: Awake {
                active: false,
                timer_remaining_ms: None,
            },
            availability: Availability {
                state: AvailabilityState::Checking,
                message: None,
            },
            displays: vec![],
            initialized_layout: false,
            review_required: false,
            closing_setup: false,
            accepted_points: vec![],
            accepted_pause: DEFAULT_PAUSE_MS,
            sources: BTreeMap::new(),
            start: None,
            last_stopped: None,
        }
    }
    fn internal() -> Failure {
        Failure::new(
            ErrorCode::InternalError,
            "Patrol counter exhausted; restart Awake",
        )
    }
    fn bump(&mut self) -> Result<(), Failure> {
        self.version = self.versions.next_id().map_err(|_| Self::internal())?;
        Ok(())
    }
    fn route_changed(&mut self) -> Result<(), Failure> {
        self.route_revision = self.routes.next_id().map_err(|_| Self::internal())?;
        if let Some(setup) = self.setup.as_mut() {
            setup.route_revision = self.route_revision;
            setup.review.confirmed_for = None;
            setup.revalidate();
        }
        Ok(())
    }
    fn idle_phase(&self) -> Phase {
        if self.review_required {
            Phase::ReviewRequired
        } else if self.setup.is_some() {
            Phase::Editing
        } else {
            Phase::Stopped
        }
    }
    fn fail(code: ErrorCode, text: &str) -> Failure {
        Failure::new(code, text)
    }
    pub fn next_display_id(&mut self) -> Result<Id, Failure> {
        self.display_ids.next_id().map_err(|_| Self::internal())
    }
    pub fn labels(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
    }
    pub fn setup_id(&self) -> Option<Id> {
        self.setup.as_ref().map(|s| s.setup_id)
    }
    pub fn has_source(&self, label: &str) -> bool {
        self.authorize(label).is_ok()
    }
    fn authorize(&self, label: &str) -> Result<&Source, Failure> {
        let source = self.sources.get(label).ok_or_else(|| {
            Self::fail(
                ErrorCode::StaleSetup,
                "This setup window is no longer registered",
            )
        })?;
        if source.bound_layout != self.layout_revision || self.setup.is_none() {
            return Err(Self::fail(
                ErrorCode::StaleSetup,
                "The display setup changed; use the current overlays",
            ));
        }
        Ok(source)
    }
    pub fn snapshot(&self, label: &str) -> Snapshot {
        Snapshot {
            state_version: self.version,
            status: self.status.clone(),
            awake: self.awake.clone(),
            availability: self.availability.clone(),
            setup: self.setup.clone(),
            caller: self.authorize(label).ok().map(|s| s.caller.clone()),
        }
    }
    pub fn error(&self, label: &str, failure: Failure) -> PatrolError {
        PatrolError {
            code: failure.code,
            message: failure.message,
            snapshot: self
                .authorize(label)
                .ok()
                .map(|_| Box::new(self.snapshot(label))),
        }
    }
    pub fn get(&mut self, label: &str) -> Result<Reply, Failure> {
        self.authorize(label)?;
        self.sources
            .get_mut(label)
            .expect("authorized source")
            .initialized = true;
        Ok(Reply {
            snapshot: self.snapshot(label),
            receipt: None,
        })
    }
    pub fn page_started(&mut self, label: &str) -> bool {
        let Some(source) = self.sources.get_mut(label) else {
            return false;
        };
        let reload = source.document_loads > 0 && source.initialized;
        source.document_loads = source.document_loads.saturating_add(1);
        reload
    }
    pub fn publish_awake(&mut self, value: Awake) -> Result<(), Failure> {
        if self.awake != value {
            self.awake = value;
            self.bump()?;
        }
        Ok(())
    }
    pub fn publish_availability(&mut self, value: Availability) -> Result<(), Failure> {
        if self.availability != value {
            self.availability = value;
            self.bump()?;
        }
        Ok(())
    }
    pub fn failure(&mut self, value: Failure) -> Result<(), Failure> {
        if self.status.last_failure.as_ref() != Some(&value) {
            self.status.last_failure = Some(value);
            self.bump()?;
        }
        Ok(())
    }
    pub fn clear_failure(&mut self, code: ErrorCode) -> Result<(), Failure> {
        if self
            .status
            .last_failure
            .as_ref()
            .is_some_and(|failure| failure.code == code)
        {
            self.status.last_failure = None;
            self.bump()?;
        }
        Ok(())
    }
    pub fn open(&mut self) -> Result<Id, Failure> {
        if self.status.run_id.is_some() {
            return Err(Self::fail(
                ErrorCode::StopPending,
                "Wait for patrol to stop before opening setup",
            ));
        }
        if self.closing_setup {
            return Err(Self::fail(
                ErrorCode::Busy,
                "The previous setup is still closing",
            ));
        }
        if let Some(setup) = &self.setup {
            return Ok(setup.setup_id);
        }
        let setup_id = self.setups.next_id().map_err(|_| Self::internal())?;
        let mut setup = Setup {
            setup_id,
            route_revision: self.route_revision,
            layout_revision: self.layout_revision,
            points: self.accepted_points.clone(),
            pause_ms: self.accepted_pause,
            displays: self.displays.clone(),
            invalid_points: vec![],
            route_validation: RouteValidation {
                valid: false,
                errors: vec![],
            },
            review: Review {
                required: self.review_required,
                confirmed_for: None,
            },
            overlays: vec![],
            preparation: None,
        };
        setup.revalidate();
        self.setup = Some(setup);
        self.sources.clear();
        self.status.phase = self.idle_phase();
        self.bump()?;
        Ok(setup_id)
    }
    pub fn register(
        &mut self,
        display_id: Id,
        mapping: Mapping,
        control: bool,
    ) -> Result<Registration, Failure> {
        if !mapping.rect().valid() {
            return Err(Self::fail(
                ErrorCode::InvalidPayload,
                "Invalid native drawing bounds",
            ));
        }
        let overlay_id = self.overlays.next_id().map_err(|_| Self::internal())?;
        let setup = self
            .setup
            .as_mut()
            .ok_or_else(|| Self::fail(ErrorCode::StaleSetup, "No open setup"))?;
        let label = format!("patrol-{}-{}", setup.setup_id, self.sources.len());
        let caller = Caller {
            overlay_id,
            display_id,
            is_control_panel: control,
            mapping,
            last_accepted_action_seq: Counter(0),
        };
        setup.overlays.push(OverlayEdit {
            overlay_id,
            display_id,
            state: EditState::Initializing,
            point_ids: vec![],
            reason: None,
        });
        self.sources.insert(
            label.clone(),
            Source {
                caller: caller.clone(),
                bound_layout: self.layout_revision,
                last_action: None,
                last_start: None,
                initialized: false,
                document_loads: 0,
            },
        );
        self.bump()?;
        Ok(Registration { label })
    }
    pub fn set_mapping(&mut self, label: &str, mapping: Mapping) -> Result<(), Failure> {
        if !mapping.rect().valid() {
            return Err(Self::fail(
                ErrorCode::OverlayFailed,
                "The overlay has invalid native content bounds",
            ));
        }
        let source = self
            .sources
            .get_mut(label)
            .ok_or_else(|| Self::fail(ErrorCode::StaleSetup, "Unknown overlay"))?;
        source.caller.mapping = mapping;
        self.bump()
    }
    pub fn layout_changed(&mut self, displays: Vec<Display>) -> Result<Vec<Effect>, Failure> {
        let meaningful = self.initialized_layout;
        self.initialized_layout = true;
        self.displays = displays;
        if !meaningful {
            self.bump()?;
            return Ok(vec![]);
        }
        self.layout_revision = self.layouts.next_id().map_err(|_| Self::internal())?;
        if self.setup.is_some() || !self.accepted_points.is_empty() || self.status.run_id.is_some()
        {
            self.review_required = true;
        }
        if let Some(setup) = self.setup.as_mut() {
            setup.layout_revision = self.layout_revision;
            setup.displays = self.displays.clone();
            setup.review = Review {
                required: self.review_required,
                confirmed_for: None,
            };
            setup.revalidate();
        }
        let effects = if let Some(run) = self.status.run_id {
            self.request_stop(run, StopReason::DisplayChanged)?
        } else {
            vec![]
        };
        if self.status.run_id.is_none() {
            self.status.phase = self.idle_phase();
        }
        self.bump()?;
        Ok(effects)
    }
    /// Call only after native windows for the old generation have been removed.
    pub fn rebuild(&mut self) -> Result<Id, Failure> {
        if self.status.run_id.is_some() {
            return Err(Self::fail(
                ErrorCode::StopPending,
                "Native stop is not complete",
            ));
        }
        let mut draft = self
            .setup
            .take()
            .ok_or_else(|| Self::fail(ErrorCode::StaleSetup, "No setup to rebuild"))?;
        draft.setup_id = self.setups.next_id().map_err(|_| Self::internal())?;
        draft.layout_revision = self.layout_revision;
        draft.displays = self.displays.clone();
        draft.overlays.clear();
        draft.preparation = None;
        draft.review = Review {
            required: self.review_required,
            confirmed_for: None,
        };
        draft.revalidate();
        let id = draft.setup_id;
        self.setup = Some(draft);
        self.sources.clear();
        self.status.phase = self.idle_phase();
        self.bump()?;
        Ok(id)
    }
    pub fn begin_close(&mut self) -> Result<(), Failure> {
        self.closing_setup = true;
        self.bump()
    }
    pub fn finish_close(&mut self) -> Result<(), Failure> {
        let changed = self
            .setup
            .as_ref()
            .is_some_and(|s| s.points != self.accepted_points || s.pause_ms != self.accepted_pause);
        self.setup = None;
        self.sources.clear();
        self.closing_setup = false;
        if changed {
            self.route_changed()?;
        }
        if self.status.run_id.is_none() {
            self.status.phase = self.idle_phase();
        }
        self.status.last_failure = None;
        self.bump()
    }
    pub fn validate_setup(&self, label: &str, id: Id) -> Result<(), Failure> {
        self.authorize(label)?;
        if self.setup_id() != Some(id) {
            return Err(Self::fail(
                ErrorCode::StaleSetup,
                "This setup has been replaced",
            ));
        }
        Ok(())
    }
    pub fn is_control(&self, label: &str) -> Result<(), Failure> {
        if self.authorize(label)?.caller.is_control_panel {
            Ok(())
        } else {
            Err(Self::fail(
                ErrorCode::Forbidden,
                "Use the control overlay for this action",
            ))
        }
    }
    fn cache_start_outcome(&mut self, run: Id, outcome: Result<Id, Failure>) {
        if let Some(start) = &self.start {
            if start.run_id == run {
                if let Some(cache) = self
                    .sources
                    .get_mut(&start.label)
                    .and_then(|s| s.last_start.as_mut())
                {
                    cache.outcome = Some(outcome);
                }
            }
        }
    }
    fn start_preconditions(&self, held_button: bool) -> Result<(), Failure> {
        if self.closing_setup {
            return Err(Self::fail(ErrorCode::Busy, "The setup is closing"));
        }
        let setup = self
            .setup
            .as_ref()
            .ok_or_else(|| Self::fail(ErrorCode::StaleSetup, "No open setup"))?;
        if !setup.route_validation.valid {
            return Err(Self::fail(ErrorCode::InvalidRoute,"Select 2–256 valid points in at least two distinct positions and a whole-second pause from 1 to 3600"));
        }
        if setup.review.required && setup.review.confirmed_for != Some(setup.pair()) {
            return Err(Self::fail(
                ErrorCode::ReviewRequired,
                "Review the current points before starting",
            ));
        }
        if setup
            .overlays
            .iter()
            .any(|s| matches!(s.state, EditState::Incomplete | EditState::Unresponsive))
        {
            return Err(Self::fail(
                ErrorCode::EditIncomplete,
                "Complete or remove the failed edit before starting",
            ));
        }
        if setup.overlays.is_empty() {
            return Err(Self::fail(
                ErrorCode::OverlayFailed,
                "No setup windows are registered",
            ));
        }
        if held_button {
            return Err(Self::fail(
                ErrorCode::Busy,
                "Release the mouse buttons before starting",
            ));
        }
        match self.availability.state {
            AvailabilityState::Ready => Ok(()),
            AvailabilityState::UnsupportedPlatform => Err(Self::fail(
                ErrorCode::UnsupportedPlatform,
                "Cursor patrolling is available on macOS and Windows",
            )),
            AvailabilityState::PermissionRequired => Err(Self::fail(
                ErrorCode::PermissionRequired,
                "Grant the required input access before starting",
            )),
            _ => Err(Self::fail(
                ErrorCode::NativeUnavailable,
                "The input observer is not ready",
            )),
        }
    }
    pub fn start(
        &mut self,
        label: &str,
        input: StartInput,
        now: Instant,
        held_button: bool,
    ) -> Result<StartDisposition, Failure> {
        self.validate_setup(label, input.setup_id)?;
        self.is_control(label)?;
        if let Some(start) = &self.start {
            if self.status.phase == Phase::Starting
                && start.label == label
                && start.request == input
            {
                return Ok(StartDisposition::Pending(start.run_id));
            }
        }
        if self.status.run_id.is_some() {
            return Err(Self::fail(
                ErrorCode::AlreadyRunning,
                "A patrol is starting, running or stopping",
            ));
        }
        if let Some(cache) = self.sources.get(label).and_then(|s| s.last_start.as_ref()) {
            if input.request_id == cache.request.request_id {
                if input != cache.request {
                    return Err(Self::fail(
                        ErrorCode::StaleAction,
                        "A Start ID cannot change its identity",
                    ));
                }
                if let Some(outcome) = &cache.outcome {
                    let caller = &self.authorize(label)?.caller;
                    return Ok(StartDisposition::Cached(outcome.clone().map(|run_id| {
                        Receipt::Start {
                            setup_id: input.setup_id,
                            overlay_id: caller.overlay_id,
                            request_id: input.request_id,
                            run_id,
                        }
                    })));
                }
            }
            if input.request_id < cache.request.request_id {
                return Err(Self::fail(
                    ErrorCode::StaleAction,
                    "This Start request is older than the last request",
                ));
            }
        }
        self.sources
            .get_mut(label)
            .expect("authorized control")
            .last_start = Some(CachedStart {
            request: input.clone(),
            outcome: None,
        });
        let setup = self.setup.as_ref().expect("authorized setup");
        let precondition = if input.route_revision != setup.route_revision
            || input.layout_revision != setup.layout_revision
        {
            Err(Self::fail(
                ErrorCode::StaleSnapshot,
                "The route or display layout changed; review it and press Start again",
            ))
        } else {
            self.start_preconditions(held_button)
        };
        if let Err(error) = precondition {
            self.sources
                .get_mut(label)
                .expect("control")
                .last_start
                .as_mut()
                .expect("cache")
                .outcome = Some(Err(error.clone()));
            self.status.last_failure = Some(error.clone());
            self.bump()?;
            return Err(error);
        }
        let run_id = self.runs.next_id().map_err(|_| Self::internal())?;
        let preparation_id = self.preparations.next_id().map_err(|_| Self::internal())?;
        let setup = self.setup.as_mut().expect("setup");
        let caller = self.sources.get(label).expect("control").caller.overlay_id;
        setup.preparation = Some(Preparation {
            preparation_id,
            requester_overlay_id: caller,
            request_id: input.request_id,
            route_revision: input.route_revision,
            layout_revision: input.layout_revision,
            pending_overlay_ids: setup.overlays.iter().map(|s| s.overlay_id).collect(),
            ready_overlay_ids: vec![],
        });
        self.start = Some(Start {
            run_id,
            label: label.into(),
            request: input,
            deadline: now + Duration::from_millis(PREPARATION_MS),
            aborted: None,
        });
        self.status.phase = Phase::Starting;
        self.status.start_stage = Some(StartStage::PreparingEdits);
        self.status.run_id = Some(run_id);
        self.status.last_failure = None;
        self.bump()?;
        Ok(StartDisposition::Pending(run_id))
    }
    fn abort_preparation(&mut self, failure: Failure) -> Result<Vec<Effect>, Failure> {
        let Some(start) = self.start.as_ref() else {
            return Ok(vec![]);
        };
        let run = start.run_id;
        if self.status.start_stage == Some(StartStage::Arming) {
            self.start.as_mut().expect("start").aborted = Some(failure.clone());
            self.status.last_failure = Some(failure);
            return self.request_stop(run, StopReason::TrayStop);
        }
        self.cache_start_outcome(run, Err(failure.clone()));
        self.start = None;
        if let Some(setup) = self.setup.as_mut() {
            setup.preparation = None;
        }
        self.status.run_id = None;
        self.status.start_stage = None;
        self.status.phase = self.idle_phase();
        self.status.last_failure = Some(failure.clone());
        self.bump()?;
        Ok(vec![Effect::StartFailed(run, failure)])
    }
    pub fn expire_preparation(&mut self, now: Instant) -> Result<Vec<Effect>, Failure> {
        if self.status.start_stage != Some(StartStage::PreparingEdits)
            || self.start.as_ref().is_none_or(|s| now < s.deadline)
        {
            return Ok(vec![]);
        }
        if let Some(setup) = self.setup.as_mut() {
            if let Some(preparation) = &setup.preparation {
                for source in &mut setup.overlays {
                    if preparation.pending_overlay_ids.contains(&source.overlay_id) {
                        source.state = EditState::Unresponsive;
                        source.reason = Some(EditReason::NotResponding);
                    }
                }
            }
        }
        self.abort_preparation(Self::fail(
            ErrorCode::EditPreparationTimedOut,
            "A setup window did not finish its edits within two seconds",
        ))
    }
    pub fn update(
        &mut self,
        label: &str,
        input: UpdateInput,
        now: Instant,
    ) -> Result<Update, Failure> {
        self.validate_setup(label, input.setup_id)?;
        let source = self.authorize(label)?.clone();
        let pair = ReviewPair {
            route_revision: input.route_revision,
            layout_revision: input.layout_revision,
        };
        let confirmation_pair = matches!(input.action, Action::ConfirmReview {}).then_some(pair);
        let receipt = Receipt::Update {
            setup_id: input.setup_id,
            overlay_id: source.caller.overlay_id,
            action_seq: input.action_seq,
        };
        if let Some(last) = &source.last_action {
            if input.action_seq == last.sequence {
                if input.action == last.action && confirmation_pair == last.confirmation_pair {
                    return Ok(Update {
                        receipt,
                        effects: vec![],
                    });
                }
                return Err(Self::fail(
                    ErrorCode::StaleAction,
                    "An accepted sequence cannot change its action or review pair",
                ));
            }
            if input.action_seq < last.sequence {
                return Err(Self::fail(
                    ErrorCode::StaleAction,
                    "This action is older than the accepted sequence",
                ));
            }
        }
        if self.closing_setup {
            return Err(Self::fail(ErrorCode::Busy, "The setup is closing"));
        }
        if self.status.start_stage == Some(StartStage::PreparingEdits)
            && self.start.as_ref().is_some_and(|s| now >= s.deadline)
        {
            // The runtime calls expire_preparation before dispatching a new mutation so the original Start resolves too.
            return Err(Self::fail(
                ErrorCode::EditPreparationTimedOut,
                "The edit preparation deadline has expired",
            ));
        }
        if self.status.start_stage == Some(StartStage::Arming)
            || self.status.phase == Phase::Stopping
        {
            return Err(Self::fail(
                ErrorCode::Busy,
                "Patrol is arming or stopping; this edit invalidates the Start",
            ));
        }
        let setup = self.setup.as_ref().expect("setup");
        if input.layout_revision != setup.layout_revision
            || (!input.action.metadata() && input.route_revision != setup.route_revision)
        {
            return Err(Self::fail(
                ErrorCode::StaleSnapshot,
                "The route or display layout changed",
            ));
        }
        if setup
            .preparation
            .as_ref()
            .is_some_and(|p| p.ready_overlay_ids.contains(&source.caller.overlay_id))
        {
            return Err(Self::fail(
                ErrorCode::Busy,
                "This window already acknowledged its final edits; the Start must be canceled",
            ));
        }
        if matches!(
            input.action,
            Action::SetPause { .. } | Action::ConfirmReview {}
        ) && !source.caller.is_control_panel
        {
            return Err(Self::fail(
                ErrorCode::Forbidden,
                "Use the control overlay for this action",
            ));
        }
        let mut changed = false;
        let mut negative_ack = false;
        let mut completed_ack = false;
        match &input.action {
            Action::AddPoint { u, v } => {
                if !(0.0..1.0).contains(u)
                    || !(0.0..1.0).contains(v)
                    || setup.points.len() >= MAX_POINTS
                {
                    return Err(Self::fail(
                        ErrorCode::InvalidRoute,
                        "Place a point within the screen; at most 256 points are allowed",
                    ));
                }
                let (display_id, position) =
                    resolve_sample(source.caller.mapping, *u, *v, &setup.displays).ok_or_else(
                        || {
                            Self::fail(
                                ErrorCode::InvalidRoute,
                                "The selected point is outside the usable displays",
                            )
                        },
                    )?;
                let display = setup
                    .displays
                    .iter()
                    .find(|d| d.display_id == display_id)
                    .expect("resolved display");
                let (x, y) = (
                    position.x - display.native_bounds.x,
                    position.y - display.native_bounds.y,
                );
                let point_id = self.points.next_id().map_err(|_| Self::internal())?;
                self.setup.as_mut().expect("setup").points.push(Point {
                    point_id,
                    display_id,
                    x,
                    y,
                });
                changed = true;
            }
            Action::MovePoint { point_id, u, v } => {
                let index = setup
                    .points
                    .iter()
                    .position(|p| p.point_id == *point_id)
                    .ok_or_else(|| {
                        Self::fail(ErrorCode::StaleSnapshot, "That point was removed")
                    })?;
                let (display_id, position) =
                    resolve_sample(source.caller.mapping, *u, *v, &setup.displays).ok_or_else(
                        || {
                            Self::fail(
                                ErrorCode::InvalidRoute,
                                "Release the point on a usable display",
                            )
                        },
                    )?;
                let display = setup
                    .displays
                    .iter()
                    .find(|d| d.display_id == display_id)
                    .expect("resolved display");
                let point = Point {
                    point_id: *point_id,
                    display_id,
                    x: position.x - display.native_bounds.x,
                    y: position.y - display.native_bounds.y,
                };
                changed = setup.points[index] != point;
                self.setup.as_mut().expect("setup").points[index] = point;
            }
            Action::RemovePoint { point_id } => {
                if !setup.points.iter().any(|p| p.point_id == *point_id) {
                    return Err(Self::fail(
                        ErrorCode::StaleSnapshot,
                        "That point was already removed",
                    ));
                }
                let setup = self.setup.as_mut().expect("setup");
                setup.points.retain(|p| p.point_id != *point_id);
                for row in &mut setup.overlays {
                    row.point_ids.retain(|id| id != point_id);
                }
                changed = true;
            }
            Action::SetPause { pause_ms } => {
                if !pause_valid(*pause_ms) {
                    return Err(Self::fail(
                        ErrorCode::InvalidRoute,
                        "Pause must be a whole number of seconds from 1 to 3600",
                    ));
                }
                changed = setup.pause_ms != *pause_ms;
                self.setup.as_mut().expect("setup").pause_ms = *pause_ms;
            }
            Action::ConfirmReview {} => {
                if !setup.route_validation.valid {
                    return Err(Self::fail(
                        ErrorCode::InvalidRoute,
                        "Correct or remove invalid points before confirming review",
                    ));
                }
                let setup = self.setup.as_mut().expect("setup");
                setup.review.confirmed_for = setup.review.required.then_some(pair);
            }
            Action::ReportEditState {
                state,
                point_ids,
                reason,
            } => {
                let duplicate = point_ids
                    .iter()
                    .enumerate()
                    .any(|(i, id)| point_ids[..i].contains(id));
                let valid = match state {
                    ReportedEditState::Ready => point_ids.is_empty() && reason.is_none(),
                    ReportedEditState::Editing => reason.is_none(),
                    ReportedEditState::Incomplete => {
                        reason.is_some() && *reason != Some(EditReason::NotResponding)
                    }
                };
                if !valid
                    || duplicate
                    || point_ids
                        .iter()
                        .any(|id| !setup.points.iter().any(|p| p.point_id == *id))
                {
                    return Err(Self::fail(
                        ErrorCode::InvalidPayload,
                        "Invalid edit-readiness report",
                    ));
                }
                let row = self
                    .setup
                    .as_mut()
                    .expect("setup")
                    .overlays
                    .iter_mut()
                    .find(|r| r.overlay_id == source.caller.overlay_id)
                    .expect("registered row");
                row.state = (*state).into();
                row.point_ids = point_ids.clone();
                row.reason = *reason;
            }
            Action::AckStartPreparation {
                preparation_id,
                outcome,
                drained_through,
            } => {
                let preparation = setup
                    .preparation
                    .as_ref()
                    .filter(|p| {
                        p.preparation_id == *preparation_id
                            && p.pending_overlay_ids.contains(&source.caller.overlay_id)
                    })
                    .ok_or_else(|| {
                        Self::fail(
                            ErrorCode::StalePreparation,
                            "That preparation is no longer pending for this window",
                        )
                    })?;
                if *drained_through != source.caller.last_accepted_action_seq {
                    return Err(Self::fail(
                        ErrorCode::StalePreparation,
                        "The acknowledgment did not drain this window's accepted actions",
                    ));
                }
                let _ = preparation;
                let setup = self.setup.as_mut().expect("setup");
                if *outcome == PreparationOutcome::Incomplete {
                    let row = setup
                        .overlays
                        .iter_mut()
                        .find(|r| r.overlay_id == source.caller.overlay_id)
                        .expect("row");
                    row.state = EditState::Incomplete;
                    row.reason.get_or_insert(EditReason::MutationFailed);
                    negative_ack = true;
                } else {
                    let row = setup
                        .overlays
                        .iter()
                        .find(|r| r.overlay_id == source.caller.overlay_id)
                        .expect("row");
                    if row.state != EditState::Ready {
                        return Err(Self::fail(
                            ErrorCode::EditIncomplete,
                            "The source must report ready after its queue settles",
                        ));
                    }
                    let preparation = setup.preparation.as_mut().expect("preparation");
                    preparation
                        .pending_overlay_ids
                        .retain(|id| *id != source.caller.overlay_id);
                    preparation.ready_overlay_ids.push(source.caller.overlay_id);
                    preparation.ready_overlay_ids.sort();
                    completed_ack = preparation.pending_overlay_ids.is_empty();
                }
            }
        }
        if changed {
            self.route_changed()?;
        }
        let source = self.sources.get_mut(label).expect("source");
        source.caller.last_accepted_action_seq = Counter(input.action_seq.get());
        source.last_action = Some(AcceptedAction {
            sequence: input.action_seq,
            action: input.action,
            confirmation_pair,
        });
        let effects = if negative_ack {
            self.abort_preparation(Self::fail(
                ErrorCode::EditIncomplete,
                "A setup window could not finalize its edit",
            ))?
        } else if completed_ack {
            let start = self.start.as_ref().expect("preparing Start");
            let setup = self.setup.as_ref().expect("setup");
            if now >= start.deadline {
                self.abort_preparation(Self::fail(
                    ErrorCode::EditPreparationTimedOut,
                    "The edit preparation expired",
                ))?
            } else if setup.pair()
                != (ReviewPair {
                    route_revision: start.request.route_revision,
                    layout_revision: start.request.layout_revision,
                })
            {
                self.abort_preparation(Self::fail(
                    ErrorCode::StaleSnapshot,
                    "Final edits changed the draft; review it and press Start again",
                ))?
            } else if let Err(error) = self.start_preconditions(false) {
                self.abort_preparation(error)?
            } else {
                let run = start.run_id;
                self.status.start_stage = Some(StartStage::Arming);
                self.setup.as_mut().expect("setup").preparation = None;
                vec![Effect::Arm(run)]
            }
        } else {
            vec![]
        };
        self.bump()?;
        Ok(Update { receipt, effects })
    }
    /// A non-deduplicated late edit must invalidate arming/positive-ack admission, even if rejected.
    pub fn reject_late_edit(&mut self, label: &str) -> Result<Vec<Effect>, Failure> {
        let Some(start) = self.start.as_ref() else {
            return Ok(vec![]);
        };
        let after_ack = self
            .setup
            .as_ref()
            .and_then(|s| s.preparation.as_ref())
            .is_some_and(|p| {
                self.sources
                    .get(label)
                    .is_some_and(|s| p.ready_overlay_ids.contains(&s.caller.overlay_id))
            });
        if self.status.start_stage == Some(StartStage::Arming) || after_ack {
            let _ = start;
            self.abort_preparation(Self::fail(
                ErrorCode::StartCanceled,
                "A new edit arrived after finalization; review the draft and start again",
            ))
        } else {
            Ok(vec![])
        }
    }
    pub fn arming(&self, run: Id) -> bool {
        self.status.run_id == Some(run)
            && self.status.phase == Phase::Starting
            && self.status.start_stage == Some(StartStage::Arming)
    }
    pub fn route_plan(&self, run: Id) -> Result<RoutePlan, Failure> {
        if !self.arming(run) {
            return Err(Self::fail(
                ErrorCode::StartCanceled,
                "This Start is no longer current",
            ));
        }
        let setup = self.setup.as_ref().expect("arming setup");
        let points = setup
            .points
            .iter()
            .map(|p| {
                resolve_point(p, &setup.displays)
                    .map_err(|_| Self::fail(ErrorCode::InvalidRoute, "A route point is invalid"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(RoutePlan {
            points,
            displays: setup.displays.clone(),
            pause_ms: setup.pause_ms,
            layout_revision: setup.layout_revision,
        })
    }
    pub fn commit(&mut self, run: Id) -> Result<Receipt, Failure> {
        if !self.arming(run) {
            return Err(Self::fail(
                ErrorCode::StartCanceled,
                "This Start was canceled",
            ));
        }
        self.start_preconditions(false)?;
        if !self.awake.active {
            return Err(Self::fail(
                ErrorCode::AwakeActivationFailed,
                "Sleep prevention is not active",
            ));
        }
        if self.awake.timer_remaining_ms == Some(Counter(0)) {
            return Err(Self::fail(
                ErrorCode::TimerExpired,
                "The Awake timer expired",
            ));
        }
        let start = self.start.as_ref().expect("start");
        let setup = self.setup.as_ref().expect("setup");
        if setup.route_revision != start.request.route_revision
            || setup.layout_revision != start.request.layout_revision
        {
            return Err(Self::fail(
                ErrorCode::StaleSnapshot,
                "The prepared route changed",
            ));
        }
        let source = self.sources.get(&start.label).expect("control");
        let receipt = Receipt::Start {
            setup_id: setup.setup_id,
            overlay_id: source.caller.overlay_id,
            request_id: start.request.request_id,
            run_id: run,
        };
        self.accepted_points = setup.points.clone();
        self.accepted_pause = setup.pause_ms;
        self.cache_start_outcome(run, Ok(run));
        self.start = None;
        self.setup = None;
        self.sources.clear();
        self.review_required = false;
        self.status.phase = Phase::Running;
        self.status.start_stage = None;
        self.status.last_failure = None;
        self.bump()?;
        Ok(receipt)
    }
    pub fn request_stop(&mut self, run: Id, reason: StopReason) -> Result<Vec<Effect>, Failure> {
        if self.status.run_id != Some(run) {
            if self.status.run_id.is_none() && self.last_stopped == Some(run) {
                return Ok(vec![]);
            }
            return Err(Self::fail(
                ErrorCode::StaleRun,
                "This is not the current patrol",
            ));
        }
        if reason == StopReason::DisplayChanged {
            self.review_required = true;
            if let Some(s) = self.setup.as_mut() {
                s.review.required = true;
                s.review.confirmed_for = None;
            }
        }
        self.status.phase = Phase::Stopping;
        self.status.start_stage = None;
        if let Some(setup) = self.setup.as_mut() {
            setup.preparation = None;
        }
        self.bump()?;
        Ok(vec![Effect::Stop(run, reason)])
    }
    pub fn stopped(
        &mut self,
        run: Id,
        reason: StopReason,
        failure: Option<Failure>,
    ) -> Result<Option<Failure>, Failure> {
        if self.status.run_id != Some(run) {
            return Ok(None);
        }
        if reason == StopReason::DisplayChanged {
            self.review_required = true;
        }
        let start_failure = self.start.as_ref().filter(|s| s.run_id == run).map(|s| {
            s.aborted.clone().or(failure.clone()).unwrap_or_else(|| {
                Self::fail(ErrorCode::StartCanceled, "Patrol Start was canceled")
            })
        });
        if let Some(error) = &start_failure {
            self.cache_start_outcome(run, Err(error.clone()));
        }
        self.start = None;
        self.last_stopped = Some(run);
        self.status.run_id = None;
        self.status.start_stage = None;
        self.status.stop_reason = Some(reason);
        self.status.last_failure = start_failure.clone().or(failure);
        if let Some(setup) = self.setup.as_mut() {
            setup.preparation = None;
            setup.review.required = self.review_required;
            if self.review_required {
                setup.review.confirmed_for = None;
            }
        }
        self.status.phase = self.idle_phase();
        self.bump()?;
        Ok(start_failure)
    }
    pub fn start_failed(&mut self, run: Id, failure: Failure) -> Result<(), Failure> {
        if self.status.run_id == Some(run) {
            self.stopped(run, StopReason::NativeFailure, Some(failure))?;
        }
        Ok(())
    }
}
