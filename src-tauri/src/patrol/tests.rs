use super::{controller::*, engine::*, model::*};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

fn id(value: u64) -> Id {
    Id::new(value).unwrap()
}
fn display(value: u64, x: f64, width: f64, unit: Unit) -> Display {
    Display {
        display_id: id(value),
        name: format!("Screen {value}"),
        native_bounds: Rect {
            x,
            y: 0.0,
            width,
            height: 100.0,
        },
        unit,
        scale: XY { x: 1.0, y: 1.0 },
        rotation_degrees: 0.0,
        mirror_of: None,
    }
}
struct Fixture {
    controller: Controller,
    control: String,
    peer: String,
    now: Instant,
}
impl Fixture {
    fn new() -> Self {
        let mut controller = Controller::new();
        controller
            .layout_changed(vec![
                display(1, 0.0, 100.0, Unit::MacosPoint),
                display(2, 200.0, 100.0, Unit::MacosPoint),
            ])
            .unwrap();
        controller
            .publish_availability(Availability {
                state: AvailabilityState::Ready,
                message: None,
            })
            .unwrap();
        controller
            .publish_awake(Awake {
                active: true,
                timer_remaining_ms: None,
            })
            .unwrap();
        controller.open().unwrap();
        let mut result = Self {
            controller,
            control: String::new(),
            peer: String::new(),
            now: Instant::now(),
        };
        result.register();
        result.act(true, Action::AddPoint { u: 0.2, v: 0.2 });
        result.act(false, Action::AddPoint { u: 0.8, v: 0.8 });
        result
    }
    fn register(&mut self) {
        self.control = self
            .controller
            .register(
                id(1),
                Mapping {
                    origin: XY { x: 0.0, y: 0.0 },
                    width: 100.0,
                    height: 100.0,
                },
                true,
            )
            .unwrap()
            .label;
        self.peer = self
            .controller
            .register(
                id(2),
                Mapping {
                    origin: XY { x: 200.0, y: 0.0 },
                    width: 100.0,
                    height: 100.0,
                },
                false,
            )
            .unwrap()
            .label;
        self.act(
            true,
            Action::ReportEditState {
                state: ReportedEditState::Ready,
                point_ids: vec![],
                reason: None,
            },
        );
        self.act(
            false,
            Action::ReportEditState {
                state: ReportedEditState::Ready,
                point_ids: vec![],
                reason: None,
            },
        );
    }
    fn input(&self, control: bool, action: Action) -> UpdateInput {
        let label = if control { &self.control } else { &self.peer };
        let snapshot = self.controller.snapshot(label);
        let setup = snapshot.setup.unwrap();
        UpdateInput {
            setup_id: setup.setup_id,
            route_revision: setup.route_revision,
            layout_revision: setup.layout_revision,
            action_seq: id(snapshot.caller.unwrap().last_accepted_action_seq.0 + 1),
            action,
        }
    }
    fn act(&mut self, control: bool, action: Action) -> Update {
        let input = self.input(control, action);
        self.controller
            .update(
                if control { &self.control } else { &self.peer },
                input,
                self.now,
            )
            .unwrap()
    }
    fn start_input(&self, request: u64) -> StartInput {
        let setup = self.controller.setup.as_ref().unwrap();
        StartInput {
            setup_id: setup.setup_id,
            route_revision: setup.route_revision,
            layout_revision: setup.layout_revision,
            request_id: id(request),
        }
    }
    fn start(&mut self, request: u64) -> Id {
        let input = self.start_input(request);
        match self
            .controller
            .start(&self.control, input, self.now, false)
            .unwrap()
        {
            StartDisposition::Pending(run) => run,
            _ => panic!("fresh Start"),
        }
    }
    fn ack(&mut self, control: bool) -> Update {
        let prep = self
            .controller
            .setup
            .as_ref()
            .unwrap()
            .preparation
            .as_ref()
            .unwrap()
            .preparation_id;
        let label = if control { &self.control } else { &self.peer };
        let drained = self
            .controller
            .snapshot(label)
            .caller
            .unwrap()
            .last_accepted_action_seq;
        self.act(
            control,
            Action::AckStartPreparation {
                preparation_id: prep,
                outcome: PreparationOutcome::Ready,
                drained_through: drained,
            },
        )
    }
    fn require_review(&mut self) {
        let mut displays = self.controller.displays.clone();
        displays[0].scale = XY { x: 2.0, y: 2.0 };
        self.controller.layout_changed(displays).unwrap();
        self.controller.rebuild().unwrap();
        self.register();
    }
}

#[test]
fn decimal_wire_values_preserve_all_u64_bits_and_reject_noncanonical_ids() {
    for number in [1, 9_007_199_254_740_993, u64::MAX] {
        let value = id(number);
        assert_eq!(
            serde_json::to_value(value).unwrap(),
            json!(number.to_string())
        );
        assert_eq!(
            serde_json::from_value::<Id>(json!(number.to_string())).unwrap(),
            value
        );
    }
    for value in [
        json!(0),
        json!(1),
        json!("0"),
        json!("01"),
        json!("+1"),
        json!("-1"),
        json!("1.0"),
        json!("18446744073709551616"),
        json!(null),
    ] {
        assert!(serde_json::from_value::<Id>(value).is_err());
    }
    assert_eq!(
        serde_json::from_value::<Counter>(json!("0")).unwrap(),
        Counter(0)
    );
    assert_eq!(
        serde_json::from_value::<Counter>(json!(u64::MAX.to_string())).unwrap(),
        Counter(u64::MAX)
    );
}
#[test]
fn wire_requires_explicit_nullable_fields_and_rejects_unknown_fields() {
    assert!(serde_json::from_value::<Awake>(json!({"active":true})).is_err());
    assert!(serde_json::from_value::<Awake>(
        json!({"active":true,"timerRemainingMs":null,"extra":1})
    )
    .is_err());
    assert_eq!(
        serde_json::from_value::<Awake>(json!({"active":false,"timerRemainingMs":null}))
            .unwrap()
            .timer_remaining_ms,
        None
    );
    assert!(
        serde_json::from_value::<Action>(json!({"type":"confirmReview","routeRevision":"5"}))
            .is_err()
    );
    assert!(serde_json::from_value::<Action>(json!({"type":"reportEditState","state":"unresponsive","pointIds":[],"reason":"notResponding"})).is_err());
    let fixture = Fixture::new();
    let reply = Reply {
        snapshot: fixture.controller.snapshot(&fixture.control),
        receipt: None,
    };
    assert_eq!(
        serde_json::from_value::<Reply>(serde_json::to_value(&reply).unwrap()).unwrap(),
        reply
    );
    let error = fixture.controller.error(
        &fixture.control,
        Failure::new(ErrorCode::StaleSnapshot, "Changed"),
    );
    assert_eq!(
        serde_json::from_value::<PatrolError>(serde_json::to_value(&error).unwrap()).unwrap(),
        error
    );
}
#[test]
fn windows_rounding_uses_positive_half_ties_and_rechecks_the_same_surface() {
    assert_eq!(quantize(-1.5), Some(-1.0));
    assert_eq!(quantize(-1.6), Some(-2.0));
    assert_eq!(quantize(1.5), Some(2.0));
    assert_eq!(quantize(i32::MAX as f64 + 1.0), None);
    assert_eq!(quantize(f64::INFINITY), None);
    let displays = vec![
        display(1, -10.0, 20.0, Unit::WindowsPixel),
        display(2, 11.0, 20.0, Unit::WindowsPixel),
    ];
    assert_eq!(canonical_position(XY { x: 9.6, y: 10.0 }, &displays), None);
    assert_eq!(canonical_position(XY { x: 10.6, y: 10.0 }, &displays), None);
    assert_eq!(
        canonical_position(XY { x: -1.5, y: 10.0 }, &displays)
            .unwrap()
            .1
            .x,
        -1.0
    );
    assert_eq!(canonical_position(XY { x: 10.0, y: 10.0 }, &displays), None);
}
#[test]
fn coordinates_handle_negative_origins_gap_before_rounding_and_distinctness_after_rounding() {
    let displays = vec![
        display(1, -100.0, 100.0, Unit::WindowsPixel),
        display(2, 100.0, 100.0, Unit::WindowsPixel),
    ];
    let mapping = Mapping {
        origin: XY { x: -100.0, y: 0.0 },
        width: 100.0,
        height: 100.0,
    };
    let resolved = resolve_sample(mapping, 2.25, 0.5, &displays).unwrap();
    assert_eq!(resolved, (id(2), XY { x: 125.0, y: 50.0 }));
    assert_eq!(resolve_sample(mapping, 1.25, 0.5, &displays), None);
    let points = vec![
        Point {
            point_id: id(1),
            display_id: id(2),
            x: 0.1,
            y: 10.1,
        },
        Point {
            point_id: id(2),
            display_id: id(2),
            x: 0.4,
            y: 10.4,
        },
    ];
    assert!(validate_route(&points, 1_000, &displays)
        .0
        .errors
        .contains(&ValidationError::DistinctPositions));
}
#[test]
fn mac_coordinates_preserve_fractions_and_normalize_negative_zero() {
    let displays = vec![display(1, 0.0, 100.0, Unit::MacosPoint)];
    let (_, point) = canonical_position(XY { x: -0.0, y: 1.25 }, &displays).unwrap();
    assert_eq!(point.x.to_bits(), 0f64.to_bits());
    assert_eq!(point.y, 1.25);
}
#[test]
fn mirror_and_overlap_resolution_uses_lowest_canonical_token() {
    let mut mirror = display(2, 0.0, 100.0, Unit::MacosPoint);
    mirror.mirror_of = Some(id(1));
    let displays = vec![
        display(3, 0.0, 100.0, Unit::MacosPoint),
        mirror,
        display(1, 0.0, 100.0, Unit::MacosPoint),
    ];
    assert_eq!(
        canonical_position(XY { x: 10.0, y: 10.0 }, &displays)
            .unwrap()
            .0,
        id(1)
    );
}
#[test]
fn accepted_update_replay_is_idempotent_before_revision_validation() {
    let mut f = Fixture::new();
    let action = f.input(true, Action::AddPoint { u: 0.4, v: 0.4 });
    let receipt = f
        .controller
        .update(&f.control, action.clone(), f.now)
        .unwrap()
        .receipt;
    f.act(false, Action::AddPoint { u: 0.3, v: 0.3 });
    let count = f.controller.setup.as_ref().unwrap().points.len();
    assert_eq!(
        f.controller
            .update(&f.control, action.clone(), f.now)
            .unwrap()
            .receipt,
        receipt
    );
    assert_eq!(f.controller.setup.as_ref().unwrap().points.len(), count);
    let mut conflicting = action;
    conflicting.action = Action::AddPoint { u: 0.1, v: 0.1 };
    assert_eq!(
        f.controller
            .update(&f.control, conflicting, f.now)
            .unwrap_err()
            .code,
        ErrorCode::StaleAction
    );
}
#[test]
fn review_consent_never_moves_to_a_draft_that_won_a_queued_race() {
    let mut f = Fixture::new();
    f.require_review();
    let gesture = f.input(true, Action::ConfirmReview {});
    f.act(false, Action::AddPoint { u: 0.4, v: 0.4 });
    assert_eq!(
        f.controller
            .update(&f.control, gesture.clone(), f.now)
            .unwrap_err()
            .code,
        ErrorCode::StaleSnapshot
    );
    assert!(f
        .controller
        .setup
        .as_ref()
        .unwrap()
        .review
        .confirmed_for
        .is_none());
    let start = f.start_input(1);
    assert_eq!(
        f.controller
            .start(&f.control, start, f.now, false)
            .unwrap_err()
            .code,
        ErrorCode::ReviewRequired
    );
    let mut fresh = f.input(true, Action::ConfirmReview {});
    fresh.action_seq = id(gesture.action_seq.get() + 1);
    f.controller.update(&f.control, fresh, f.now).unwrap();
    assert!(f
        .controller
        .setup
        .as_ref()
        .unwrap()
        .review
        .confirmed_for
        .is_some());
    f.start(2);
}
#[test]
fn lost_accepted_review_reply_cannot_restore_consent_cleared_by_a_peer() {
    let mut f = Fixture::new();
    f.require_review();
    let accepted = f.input(true, Action::ConfirmReview {});
    f.controller
        .update(&f.control, accepted.clone(), f.now)
        .unwrap();
    f.act(false, Action::AddPoint { u: 0.1, v: 0.3 });
    f.controller
        .update(&f.control, accepted.clone(), f.now)
        .unwrap();
    assert!(f
        .controller
        .setup
        .as_ref()
        .unwrap()
        .review
        .confirmed_for
        .is_none());
    let mut rebase = accepted;
    rebase.route_revision = f.controller.setup.as_ref().unwrap().route_revision;
    assert_eq!(
        f.controller
            .update(&f.control, rebase, f.now)
            .unwrap_err()
            .code,
        ErrorCode::StaleAction
    );
}
#[test]
fn all_sources_must_ack_before_native_arming() {
    let mut f = Fixture::new();
    let run = f.start(1);
    assert!(f.ack(true).effects.is_empty());
    assert!(!f.controller.arming(run));
    let result = f.ack(false);
    assert!(matches!(&result.effects[..],[Effect::Arm(id)] if *id==run));
    assert!(f.controller.arming(run));
    let plan = f.controller.route_plan(run).unwrap();
    assert_eq!(
        plan.points,
        vec![XY { x: 20.0, y: 20.0 }, XY { x: 280.0, y: 80.0 }]
    );
}
#[test]
fn final_peer_edit_drains_but_aborts_the_unseen_start() {
    let mut f = Fixture::new();
    let run = f.start(1);
    f.ack(true);
    let point = f.controller.setup.as_ref().unwrap().points[1].point_id;
    f.act(
        false,
        Action::MovePoint {
            point_id: point,
            u: 0.5,
            v: 0.6,
        },
    );
    let result = f.ack(false);
    assert!(
        matches!(&result.effects[..],[Effect::StartFailed(id,error)] if *id==run&&error.code==ErrorCode::StaleSnapshot)
    );
    assert!(!f.controller.arming(run));
    assert_eq!(f.controller.setup.as_ref().unwrap().points[1].x, 50.0);
    let repeated = f.start_input(1);
    assert_eq!(
        f.controller
            .start(&f.control, repeated, f.now, false)
            .unwrap_err()
            .code,
        ErrorCode::StaleAction
    );
    f.start(2);
}
#[test]
fn acknowledgment_at_exact_deadline_cannot_arm_even_if_timeout_task_was_delayed() {
    let mut f = Fixture::new();
    f.start(1);
    f.ack(true);
    f.now += Duration::from_millis(PREPARATION_MS);
    let prep = f
        .controller
        .setup
        .as_ref()
        .unwrap()
        .preparation
        .as_ref()
        .unwrap()
        .preparation_id;
    let drained = f
        .controller
        .snapshot(&f.peer)
        .caller
        .unwrap()
        .last_accepted_action_seq;
    let input = f.input(
        false,
        Action::AckStartPreparation {
            preparation_id: prep,
            outcome: PreparationOutcome::Ready,
            drained_through: drained,
        },
    );
    assert_eq!(
        f.controller.update(&f.peer, input, f.now).unwrap_err().code,
        ErrorCode::EditPreparationTimedOut
    );
    let effects = f.controller.expire_preparation(f.now).unwrap();
    assert!(
        matches!(&effects[..],[Effect::StartFailed(_,failure)] if failure.code==ErrorCode::EditPreparationTimedOut)
    );
    assert_eq!(
        f.controller.setup.as_ref().unwrap().overlays[1].state,
        EditState::Unresponsive
    );
}
#[test]
fn duplicate_ack_has_one_effect_and_new_sequence_after_ack_invalidates_start() {
    let mut f = Fixture::new();
    f.start(1);
    let prep = f
        .controller
        .setup
        .as_ref()
        .unwrap()
        .preparation
        .as_ref()
        .unwrap()
        .preparation_id;
    let drained = f
        .controller
        .snapshot(&f.control)
        .caller
        .unwrap()
        .last_accepted_action_seq;
    let ack = f.input(
        true,
        Action::AckStartPreparation {
            preparation_id: prep,
            outcome: PreparationOutcome::Ready,
            drained_through: drained,
        },
    );
    f.controller.update(&f.control, ack.clone(), f.now).unwrap();
    f.controller.update(&f.control, ack, f.now).unwrap();
    assert_eq!(
        f.controller
            .setup
            .as_ref()
            .unwrap()
            .preparation
            .as_ref()
            .unwrap()
            .ready_overlay_ids
            .len(),
        1
    );
    let edit = f.input(true, Action::SetPause { pause_ms: 2_000 });
    assert_eq!(
        f.controller
            .update(&f.control, edit, f.now)
            .unwrap_err()
            .code,
        ErrorCode::Busy
    );
    assert!(!f
        .controller
        .reject_late_edit(&f.control)
        .unwrap()
        .is_empty());
    assert!(f.controller.status.run_id.is_none());
}
#[test]
fn metadata_can_drain_after_a_sibling_route_revision_change() {
    let mut f = Fixture::new();
    let stale = f.input(
        true,
        Action::ReportEditState {
            state: ReportedEditState::Ready,
            point_ids: vec![],
            reason: None,
        },
    );
    f.act(false, Action::AddPoint { u: 0.3, v: 0.3 });
    f.controller.update(&f.control, stale, f.now).unwrap();
}
#[test]
fn noncontrol_role_and_old_setup_cannot_mutate_current_draft() {
    let mut f = Fixture::new();
    let input = f.input(false, Action::SetPause { pause_ms: 2_000 });
    assert_eq!(
        f.controller.update(&f.peer, input, f.now).unwrap_err().code,
        ErrorCode::Forbidden
    );
    let old = f.input(true, Action::AddPoint { u: 0.1, v: 0.1 });
    let old_label = f.control.clone();
    f.controller.begin_close().unwrap();
    f.controller.finish_close().unwrap();
    f.controller.open().unwrap();
    f.register();
    assert_ne!(old_label, f.control);
    assert_eq!(
        f.controller
            .update(&old_label, old, f.now)
            .unwrap_err()
            .code,
        ErrorCode::StaleSetup
    );
    assert!(f
        .controller
        .error("main", Failure::new(ErrorCode::StaleSetup, "old"))
        .snapshot
        .is_none());
}
#[test]
fn cancel_and_reconnection_cannot_clear_display_review_requirement() {
    let mut f = Fixture::new();
    f.require_review();
    f.controller.begin_close().unwrap();
    f.controller.finish_close().unwrap();
    f.controller.layout_changed(vec![]).unwrap();
    let revision = f.controller.layout_revision;
    f.controller
        .layout_changed(vec![display(1, 0.0, 100.0, Unit::MacosPoint)])
        .unwrap();
    assert!(f.controller.layout_revision > revision);
    assert!(f.controller.review_required);
    f.controller.open().unwrap();
    assert!(f.controller.setup.as_ref().unwrap().review.required);
}

fn plan() -> RoutePlan {
    RoutePlan {
        points: vec![
            XY { x: 0.0, y: 10.0 },
            XY { x: 80.0, y: 10.0 },
            XY { x: 20.0, y: 20.0 },
        ],
        displays: vec![display(1, 0.0, 100.0, Unit::MacosPoint)],
        pause_ms: 1_000,
        layout_revision: id(1),
    }
}
#[test]
fn endpoint_delivery_starts_a_full_pause_from_actual_call_completion() {
    let base = Instant::now();
    let mut engine = Engine::new(plan(), XY { x: 90.0, y: 10.0 }, base).unwrap();
    for ms in [0, 100, 200, 300, 400, 490] {
        if let Decision::Emit(_) = engine.poll(base + Duration::from_millis(ms)) {
            engine.complete(base + Duration::from_millis(ms), true);
        }
    }
    assert_eq!(
        engine.poll(base + Duration::from_millis(510)),
        Decision::Emit(XY { x: 0.0, y: 10.0 })
    );
    engine.complete(base + Duration::from_millis(520), true);
    for ms in [600, 700, 800, 900, 1000, 1100, 1200, 1300, 1400, 1500] {
        assert!(matches!(
            engine.poll(base + Duration::from_millis(ms)),
            Decision::Wait(_)
        ));
    }
    assert!(matches!(
        engine.poll(base + Duration::from_millis(1519)),
        Decision::Wait(_)
    ));
    assert!(matches!(
        engine.poll(base + Duration::from_millis(1520)),
        Decision::Emit(_)
    ));
}
#[test]
fn easing_is_smoothstep_and_pending_or_rate_limited_calls_never_burst() {
    let base = Instant::now();
    let mut engine = Engine::new(plan(), XY { x: 80.0, y: 10.0 }, base).unwrap();
    assert_eq!(engine.poll(base), Decision::Emit(XY { x: 80.0, y: 10.0 }));
    assert_eq!(engine.poll(base), Decision::InFlight);
    engine.complete(base, true);
    assert!(matches!(
        engine.poll(base + Duration::from_millis(1)),
        Decision::Wait(_)
    ));
    for ms in [100, 200] {
        let _ = engine.poll(base + Duration::from_millis(ms));
        engine.complete(base + Duration::from_millis(ms), true);
    }
    assert_eq!(
        engine.poll(base + Duration::from_millis(250)),
        Decision::Emit(XY { x: 40.0, y: 10.0 })
    );
}
#[test]
fn a_stalled_wake_cancels_before_an_overdue_endpoint() {
    let base = Instant::now();
    let mut engine = Engine::new(plan(), XY { x: 80.0, y: 10.0 }, base).unwrap();
    assert!(matches!(engine.poll(base), Decision::Emit(_)));
    engine.complete(base, true);
    assert_eq!(
        engine.poll(base + Duration::from_millis(500)),
        Decision::Stopped(StopReason::ExecutionStalled)
    );
}
#[test]
fn intermediate_gaps_are_omitted_but_endpoint_arrival_is_retained() {
    let base = Instant::now();
    let p = RoutePlan {
        points: vec![XY { x: 20.0, y: 10.0 }, XY { x: 280.0, y: 10.0 }],
        displays: vec![
            display(1, 0.0, 100.0, Unit::WindowsPixel),
            display(2, 200.0, 100.0, Unit::WindowsPixel),
        ],
        pause_ms: 1_000,
        layout_revision: id(1),
    };
    let mut engine = Engine::new(p, XY { x: 280.0, y: 10.0 }, base).unwrap();
    for ms in [0, 100, 200] {
        if let Decision::Emit(_) = engine.poll(base + Duration::from_millis(ms)) {
            engine.complete(base + Duration::from_millis(ms), true);
        }
    }
    assert!(matches!(
        engine.poll(base + Duration::from_millis(250)),
        Decision::Wait(_)
    ));
    for ms in [350, 450] {
        if let Decision::Emit(_) = engine.poll(base + Duration::from_millis(ms)) {
            engine.complete(base + Duration::from_millis(ms), true);
        }
    }
    assert_eq!(
        engine.poll(base + Duration::from_millis(510)),
        Decision::Emit(XY { x: 20.0, y: 10.0 })
    );
}
#[test]
fn observer_and_availability_authorization_expire_inclusively_and_old_generations_do_not_refresh() {
    let now = Instant::now();
    let control = RunControl::new(id(1));
    control.publish_observer(id(1), now);
    control.publish_availability(id(1), now);
    assert!(control.healthy(now + Duration::from_millis(99)));
    control.publish_observer(id(2), now + Duration::from_millis(100));
    assert_eq!(
        control.eligibility(now + Duration::from_millis(100)),
        Err(StopReason::ObserverLost)
    );
    control.publish_observer(id(1), now + Duration::from_millis(50));
    assert_eq!(
        control.eligibility(now + Duration::from_millis(100)),
        Err(StopReason::AvailabilityExpired)
    );
}
#[test]
fn first_stop_reason_wins_but_a_later_display_change_still_requires_review() {
    let control = RunControl::new(id(1));
    control.cancel(StopReason::Escape);
    control.cancel(StopReason::DisplayChanged);
    assert_eq!(control.reason(), Some(StopReason::Escape));
    assert!(control.saw_display_change());
    assert!(!control.is_acknowledged());
}
#[test]
fn timer_math_accepts_zero_and_maximum_u64_milliseconds_without_future_instant_overflow() {
    let now = Instant::now();
    let zero = Deadline {
        revision: 1,
        started_at: now,
        duration: Duration::ZERO,
    };
    assert!(zero.expired(now));
    let huge = Deadline {
        revision: 2,
        started_at: now,
        duration: Duration::from_millis(u64::MAX),
    };
    assert_eq!(
        huge.remaining(now + Duration::from_secs(1)),
        Duration::from_millis(u64::MAX) - Duration::from_secs(1)
    );
}
struct FakeClock {
    base: Instant,
    ticks: Mutex<Duration>,
    automatic: bool,
}
impl Clock for FakeClock {
    fn now(&self) -> Instant {
        self.base + *self.ticks.lock().unwrap()
    }
    fn wait(&self, duration: Duration) {
        if self.automatic {
            *self.ticks.lock().unwrap() += duration;
        } else {
            std::thread::park();
        }
    }
}
struct FakeMovement {
    calls: Arc<AtomicUsize>,
    entered: Option<mpsc::Sender<()>>,
    release: Option<mpsc::Receiver<()>>,
}
impl Movement for FakeMovement {
    fn prepare(&mut self, _: &[XY]) -> Result<XY, Failure> {
        Ok(XY { x: 90.0, y: 10.0 })
    }
    fn position(
        &mut self,
        _: XY,
        before: &dyn Fn() -> Result<(), StopReason>,
    ) -> Result<(), StopReason> {
        before()?;
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(entered) = &self.entered {
            entered.send(()).unwrap();
        }
        if let Some(release) = &self.release {
            release.recv().unwrap();
        }
        Ok(())
    }
}
#[test]
fn a_pause_still_wakes_at_exact_lease_expiry_without_native_movement() {
    let now = Instant::now();
    let clock = Arc::new(FakeClock {
        base: now,
        ticks: Mutex::new(Duration::ZERO),
        automatic: true,
    });
    let control = RunControl::new(id(1));
    control.publish_observer(id(1), now);
    control.publish_availability(id(1), now);
    let gate = EmissionGate::new(id(1));
    {
        let mut state = gate.state.lock().unwrap();
        state.awake_active = true;
        state.armed = true;
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let movement = Box::new(FakeMovement {
        calls: calls.clone(),
        entered: None,
        release: None,
    });
    assert_eq!(
        run_worker_with_clock(
            plan(),
            XY { x: 0.0, y: 10.0 },
            movement,
            control.clone(),
            gate,
            || {},
            clock.clone()
        ),
        StopReason::ObserverLost
    );
    assert_eq!(*clock.ticks.lock().unwrap(), Duration::from_millis(100));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(control.is_acknowledged());
}
#[test]
fn cancellation_and_awake_transition_never_acknowledge_an_inflight_native_call_early() {
    let now = Instant::now();
    let clock = Arc::new(FakeClock {
        base: now,
        ticks: Mutex::new(Duration::ZERO),
        automatic: false,
    });
    let control = RunControl::new(id(1));
    control.publish_observer(id(1), now);
    control.publish_availability(id(1), now);
    let gate = EmissionGate::new(id(1));
    {
        let mut state = gate.state.lock().unwrap();
        state.awake_active = true;
        state.armed = true;
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let (send, entered) = mpsc::channel();
    let (release, recv) = mpsc::channel();
    let movement = Box::new(FakeMovement {
        calls: calls.clone(),
        entered: Some(send),
        release: Some(recv),
    });
    let signal = control.clone();
    let worker_gate = gate.clone();
    let worker = std::thread::spawn(move || {
        run_worker_with_clock(
            plan(),
            XY { x: 90.0, y: 10.0 },
            movement,
            signal,
            worker_gate,
            || {},
            clock,
        )
    });
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(gate.defer_transition());
    control.cancel(StopReason::Escape);
    assert!(!control.is_acknowledged());
    assert!(gate.state.lock().unwrap().emitting);
    release.send(()).unwrap();
    assert_eq!(worker.join().unwrap(), StopReason::Escape);
    assert!(control.is_acknowledged());
    assert!(!gate.state.lock().unwrap().emitting);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn ordered_cross_screen_loop_pauses_before_leaving_each_delivered_endpoint() {
    let now = Instant::now();
    let route = RoutePlan {
        points: vec![
            XY { x: 20.0, y: 20.0 },
            XY { x: 220.0, y: 30.0 },
            XY { x: 70.0, y: 40.0 },
        ],
        displays: vec![
            display(1, 0.0, 100.0, Unit::MacosPoint),
            display(2, 200.0, 100.0, Unit::MacosPoint),
        ],
        pause_ms: 1000,
        layout_revision: id(1),
    };
    let mut engine = Engine::new(route.clone(), route.points[0], now).unwrap();
    let mut endpoints = vec![];
    for tick in 1..=500 {
        let at = now + Duration::from_millis(tick * 20);
        match engine.poll(at) {
            Decision::Emit(p) => {
                if route.points.contains(&p) {
                    endpoints.push((tick * 20, p));
                }
                engine.complete(at, true);
            }
            Decision::Stopped(reason) => panic!("Unexpected stop: {reason:?}"),
            _ => {}
        }
        if endpoints.len() == 4 {
            break;
        }
    }
    assert_eq!(
        endpoints.iter().map(|(_, p)| *p).collect::<Vec<_>>(),
        vec![
            route.points[0],
            route.points[1],
            route.points[1],
            route.points[2]
        ]
    );
    // The zero-progress sample repeats the last endpoint at departure, after a full 1000ms pause.
    assert!(endpoints[0].0 >= 1000);
    assert!(endpoints[2].0 - endpoints[1].0 >= 1000);
    let mut last = route.points[0];
    let mut delivered = vec![];
    let mut engine = Engine::new(route.clone(), last, now).unwrap();
    for tick in 1..=400 {
        let at = now + Duration::from_millis(tick * 20);
        if let Decision::Emit(p) = engine.poll(at) {
            if route.points.contains(&p) && p != last {
                delivered.push(p);
                last = p;
            }
            engine.complete(at, true);
        }
        if delivered.len() == 4 {
            break;
        }
    }
    assert_eq!(
        delivered,
        vec![
            route.points[1],
            route.points[2],
            route.points[0],
            route.points[1]
        ]
    );
}

#[test]
fn final_native_precheck_samples_time_after_a_slow_os_query() {
    struct Delayed {
        clock: Arc<FakeClock>,
        calls: Arc<AtomicUsize>,
    }
    impl Movement for Delayed {
        fn prepare(&mut self, _: &[XY]) -> Result<XY, Failure> {
            unreachable!()
        }
        fn position(
            &mut self,
            _: XY,
            before: &dyn Fn() -> Result<(), StopReason>,
        ) -> Result<(), StopReason> {
            *self.clock.ticks.lock().unwrap() = Duration::from_millis(100);
            before()?;
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    let now = Instant::now();
    let clock = Arc::new(FakeClock {
        base: now,
        ticks: Mutex::new(Duration::ZERO),
        automatic: true,
    });
    let control = RunControl::new(id(1));
    control.publish_observer(id(1), now);
    control.publish_availability(id(1), now);
    let gate = EmissionGate::new(id(1));
    {
        let mut g = gate.state.lock().unwrap();
        g.awake_active = true;
        g.armed = true;
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let movement = Box::new(Delayed {
        clock: clock.clone(),
        calls: calls.clone(),
    });
    assert_eq!(
        run_worker_with_clock(
            plan(),
            XY { x: 90.0, y: 10.0 },
            movement,
            control.clone(),
            gate,
            || {},
            clock
        ),
        StopReason::ObserverLost
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(control.is_acknowledged());
}

#[test]
fn awake_activation_failure_or_expired_timer_cannot_commit_a_prepared_draft() {
    for (active, remaining, code) in [
        (false, None, ErrorCode::AwakeActivationFailed),
        (true, Some(Counter(0)), ErrorCode::TimerExpired),
    ] {
        let mut f = Fixture::new();
        let run = f.start(1);
        f.ack(true);
        f.ack(false);
        let points = f.controller.setup.as_ref().unwrap().points.clone();
        f.controller
            .publish_awake(Awake {
                active,
                timer_remaining_ms: remaining,
            })
            .unwrap();
        assert_eq!(f.controller.commit(run).unwrap_err().code, code);
        assert_eq!(f.controller.setup.as_ref().unwrap().points, points);
        assert!(f.controller.arming(run));
        f.controller
            .start_failed(run, Failure::new(code, "injected failure"))
            .unwrap();
        assert_eq!(f.controller.status.run_id, None);
        assert_eq!(f.controller.awake.active, active);
        assert_eq!(f.controller.setup.as_ref().unwrap().points, points);
    }
}

#[test]
fn successful_commit_retains_route_for_next_setup_and_old_sources_cannot_cancel_it() {
    let mut f = Fixture::new();
    let run = f.start(1);
    f.ack(true);
    f.ack(false);
    let old_setup = f.controller.setup_id().unwrap();
    let points = f.controller.setup.as_ref().unwrap().points.clone();
    assert!(matches!(f.controller.commit(run).unwrap(),Receipt::Start{run_id,..}if run_id==run));
    assert!(f.controller.setup.is_none());
    assert_eq!(f.controller.status.phase, Phase::Running);
    assert!(!f.controller.has_source(&f.control));
    f.controller
        .request_stop(run, StopReason::ManualMovement)
        .unwrap();
    assert_eq!(f.controller.status.phase, Phase::Stopping);
    assert_eq!(f.controller.status.run_id, Some(run));
    f.controller
        .stopped(run, StopReason::ManualMovement, None)
        .unwrap();
    assert!(f.controller.awake.active);
    let new_setup = f.controller.open().unwrap();
    assert_ne!(new_setup, old_setup);
    assert_eq!(f.controller.setup.as_ref().unwrap().points, points);
    assert_eq!(
        f.controller
            .validate_setup(&f.control, old_setup)
            .unwrap_err()
            .code,
        ErrorCode::StaleSetup
    );
}

#[test]
fn late_old_run_stops_and_heartbeats_never_touch_a_new_generation() {
    let mut f = Fixture::new();
    let first = f.start(1);
    f.ack(true);
    f.ack(false);
    f.controller.commit(first).unwrap();
    f.controller
        .stopped(first, StopReason::Escape, None)
        .unwrap();
    f.controller.open().unwrap();
    f.register();
    let second = f.start(2);
    assert_ne!(second, first);
    assert_eq!(
        f.controller
            .request_stop(first, StopReason::Escape)
            .unwrap_err()
            .code,
        ErrorCode::StaleRun
    );
    assert_eq!(f.controller.status.run_id, Some(second));
    let control = RunControl::new(second);
    control.publish_observer(first, f.now);
    control.publish_availability(first, f.now);
    assert!(!control.healthy(f.now));
    control.publish_observer(second, f.now);
    control.publish_availability(second, f.now);
    assert!(control.healthy(f.now));
}

#[test]
fn native_input_routing_changes_from_setup_edits_to_immediate_takeover_atomically() {
    use super::platform::{Event, Input, InputRouter};
    let events = Arc::new(Mutex::new(vec![]));
    let recorded = events.clone();
    let router = InputRouter::new(id(1), move |event| recorded.lock().unwrap().push(event));
    let control = RunControl::new(id(1));
    assert!(router.reserve_run(control.clone()));
    router.input(Input::Motion);
    router.input(Input::Button);
    assert!(control.reason().is_none());
    assert!(router.arm(id(1)));
    router.committed(id(1));
    router.input(Input::Motion);
    assert_eq!(control.reason(), Some(StopReason::ManualMovement));
    router.input(Input::Escape);
    assert!(events.lock().unwrap().is_empty());
    router.clear_run(id(1));
    router.set_setup(id(2));
    router.input(Input::Escape);
    assert!(
        matches!(events.lock().unwrap().as_slice(),[Event::EscapeSetup(setup)] if *setup==id(2))
    );
    let next = RunControl::new(id(2));
    assert!(!router.reserve_run(next.clone()));
    assert_eq!(next.reason(), Some(StopReason::Escape));
    router.disable();
    router.input(Input::Escape);
    assert_eq!(events.lock().unwrap().len(), 1);
}

#[test]
fn observer_heartbeat_is_bound_to_the_current_reserved_run() {
    use super::platform::InputRouter;
    let router = InputRouter::new(id(1), |_| {});
    let first = RunControl::new(id(1));
    let second = RunControl::new(id(2));
    let now = Instant::now();
    router.reserve_run(first.clone());
    router.health(now);
    first.publish_availability(id(1), now);
    assert!(first.healthy(now));
    router.clear_run(id(1));
    router.reserve_run(second.clone());
    second.publish_availability(id(2), now);
    assert!(!second.healthy(now));
    router.health(now);
    assert!(second.healthy(now));
    assert_eq!(
        first.eligibility(now + Duration::from_millis(100)),
        Err(StopReason::ObserverLost)
    );
}

#[test]
fn off_waits_for_native_ack_but_failure_does_not_restart_movement_or_falsify_awake() {
    let now = Instant::now();
    let control = RunControl::new(id(1));
    control.publish_observer(id(1), now);
    control.publish_availability(id(1), now);
    let gate = EmissionGate::new(id(1));
    {
        let mut state = gate.state.lock().unwrap();
        state.awake_active = true;
        state.armed = true;
    }
    let emission = gate.reserve(&control, id(1), now).unwrap().unwrap();
    assert!(gate.defer_transition());
    control.cancel(StopReason::AwakeOff);
    assert!(!gate.transition_ready(Some(&control)));
    drop(emission);
    assert!(!gate.transition_ready(Some(&control)));
    control.acknowledge();
    assert!(gate.transition_ready(Some(&control)));
    // A fallible inhibitor-release port leaves actual Awake state active. Cancellation is permanent.
    let release: Result<(), &str> = Err("injected inhibitor error");
    if release.is_ok() {
        gate.state.lock().unwrap().awake_active = false;
    }
    assert!(gate.state.lock().unwrap().awake_active);
    assert!(matches!(
        gate.reserve(&control, id(1), now),
        Err(StopReason::AwakeOff)
    ));
}

#[test]
fn control_panel_always_belongs_to_a_canonical_surface_with_primary_fallback() {
    let first = display(1, 0.0, 100.0, Unit::MacosPoint);
    let mut mirror = display(2, 0.0, 100.0, Unit::MacosPoint);
    mirror.mirror_of = Some(id(1));
    let other = display(3, 200.0, 100.0, Unit::MacosPoint);
    let displays = vec![mirror, other, first];
    assert_eq!(
        control_display(&displays, Some(XY { x: 10.0, y: 10.0 }), Some(id(2))),
        Some(id(1))
    );
    assert_eq!(control_display(&displays, None, Some(id(2))), Some(id(1)));
    assert_eq!(control_display(&displays, None, Some(id(3))), Some(id(3)));
    assert_eq!(
        control_display(&displays, Some(XY { x: 250.0, y: 10.0 }), Some(id(1))),
        Some(id(3))
    );
    assert_eq!(control_display(&[], None, None), None);
}

#[test]
fn getters_do_not_emit_state_events_or_create_a_refetch_feedback_loop() {
    let (tx, _rx) = mpsc::channel();
    let runtime = super::PatrolRuntime::new(tx);
    let mut core = runtime.core.lock().unwrap();
    assert!(!core.publication_changed());
    core.controller
        .layout_changed(vec![display(1, 0.0, 100.0, Unit::MacosPoint)])
        .unwrap();
    core.controller.open().unwrap();
    let source = core
        .controller
        .register(
            id(1),
            Mapping {
                origin: XY { x: 0.0, y: 0.0 },
                width: 100.0,
                height: 100.0,
            },
            true,
        )
        .unwrap();
    assert!(core.publication_changed());
    for _ in 0..20 {
        core.controller.get(&source.label).unwrap();
        assert!(!core.publication_changed());
    }
    core.controller
        .publish_awake(Awake {
            active: true,
            timer_remaining_ms: None,
        })
        .unwrap();
    assert!(core.publication_changed());
    assert!(!core.publication_changed());
    core.controller
        .publish_awake(Awake {
            active: true,
            timer_remaining_ms: None,
        })
        .unwrap();
    assert!(!core.publication_changed());
}

#[test]
fn windows_registration_null_probe_accepts_documented_insufficient_buffer_result() {
    use super::platform::{read_raw_input_registrations, registration_read, RegistrationRead};
    assert!(matches!(
        registration_read(u32::MAX, 2, true, true),
        Some(RegistrationRead::Required(2))
    ));
    assert!(matches!(
        registration_read(u32::MAX, 3, false, true),
        Some(RegistrationRead::Required(3))
    ));
    assert!(registration_read(u32::MAX, 2, true, false).is_none());
    assert!(matches!(
        registration_read(2, 2, false, false),
        Some(RegistrationRead::Filled(2))
    ));
    let mut calls = 0;
    let registrations = read_raw_input_registrations::<u8>(|buffer| {
        calls += 1;
        if let Some(buffer) = buffer {
            assert_eq!(buffer.len(), 2);
            buffer.copy_from_slice(&[2, 6]);
            Ok(registration_read(2, 2, false, false).unwrap())
        } else {
            Ok(registration_read(u32::MAX, 2, true, true).unwrap())
        }
    })
    .unwrap();
    assert_eq!(registrations, vec![2, 6]);
    assert_eq!(calls, 2);
}

#[test]
fn windows_empty_registration_probe_does_not_make_a_zero_length_buffer_call() {
    use super::platform::{read_raw_input_registrations, registration_read};
    for copied in [0, u32::MAX] {
        let registrations = read_raw_input_registrations::<u8>(|buffer| {
            assert!(buffer.is_none());
            Ok(registration_read(copied, 0, true, copied == u32::MAX).unwrap())
        })
        .unwrap();
        assert!(registrations.is_empty());
    }
}

#[test]
fn windows_registration_query_retries_a_size_race_and_uses_only_written_entries() {
    use super::platform::{read_raw_input_registrations, RegistrationRead};
    let mut calls = 0;
    let registrations = read_raw_input_registrations::<u8>(|buffer| {
        calls += 1;
        match calls {
            1 => {
                assert!(buffer.is_none());
                Ok(RegistrationRead::Required(1))
            }
            2 => {
                assert_eq!(buffer.unwrap().len(), 1);
                Ok(RegistrationRead::Required(3))
            }
            3 => {
                let buffer = buffer.unwrap();
                assert_eq!(buffer.len(), 3);
                buffer[0] = 2;
                buffer[1] = 6;
                Ok(RegistrationRead::Filled(2))
            }
            _ => panic!("Unexpected query"),
        }
    })
    .unwrap();
    assert_eq!(registrations, vec![2, 6]);
    assert_eq!(calls, 3);
}

#[test]
fn windows_registration_query_preserves_real_errors_instead_of_reporting_ownership() {
    use super::platform::{read_raw_input_registrations, RegistrationRead};
    let error = Failure::new(
        ErrorCode::NativeUnavailable,
        "Cannot query Awake's Raw Input registrations (Windows error 5)",
    );
    assert_eq!(
        read_raw_input_registrations::<u8>(|_| Err(error.clone())).unwrap_err(),
        error
    );
    let actual = read_raw_input_registrations::<u8>(|buffer| {
        if buffer.is_none() {
            Ok(RegistrationRead::Required(2))
        } else {
            Err(error.clone())
        }
    })
    .unwrap_err();
    assert_eq!(actual, error);
}

#[test]
fn windows_registration_enumeration_is_bounded_and_rejects_invalid_counts() {
    use super::platform::{read_raw_input_registrations, RegistrationRead};
    let mut calls = 0;
    assert!(read_raw_input_registrations::<u8>(|_| {
        calls += 1;
        Ok(RegistrationRead::Required(4097))
    })
    .is_err());
    assert_eq!(calls, 1);
    calls = 0;
    assert!(read_raw_input_registrations::<u8>(|_| {
        calls += 1;
        Ok(RegistrationRead::Required(calls))
    })
    .is_err());
    assert_eq!(calls, 4);
    assert!(
        read_raw_input_registrations::<u8>(|buffer| if buffer.is_none() {
            Ok(RegistrationRead::Required(1))
        } else {
            Ok(RegistrationRead::Filled(2))
        })
        .is_err()
    );
}
