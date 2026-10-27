//! Request policy and retained run ownership; no native UI operations.
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tauri::WebviewWindow;

const PREPARE: Duration = Duration::from_secs(2);
const PREVIEW_LEASE: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Scope {
    Production,
    Preview,
}
impl Scope {
    pub(super) fn owner(self) -> &'static str {
        match self {
            Self::Production => "main",
            Self::Preview => "startup-preview",
        }
    }
    pub(super) fn surface(self) -> &'static str {
        match self {
            Self::Production => "startup-intro",
            Self::Preview => "startup-preview-surface",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tuning {
    opening_ms: f64,
    hold_ms: f64,
    exit_ms: f64,
    backdrop_enter_ms: f64,
    wave_start_ms: f64,
    intro_cell_px: f64,
    wave_opacity: f64,
    wave_enabled: bool,
    backdrop_center_alpha: f64,
    backdrop_edge_alpha: f64,
    idle_cycle_ms: f64,
    idle_amplitude_rows: f64,
}
impl Default for Tuning {
    fn default() -> Self {
        Self {
            opening_ms: 850.,
            hold_ms: 600.,
            exit_ms: 300.,
            backdrop_enter_ms: 120.,
            wave_start_ms: 200.,
            intro_cell_px: 15.,
            wave_opacity: 0.25,
            wave_enabled: true,
            backdrop_center_alpha: 0.55,
            backdrop_edge_alpha: 0.30,
            idle_cycle_ms: 2400.,
            idle_amplitude_rows: 0.5,
        }
    }
}
impl Tuning {
    pub(super) fn validate(&self) -> Result<(), String> {
        let between = |x: f64, a: f64, b: f64| x.is_finite() && x >= a && x <= b;
        if !between(self.opening_ms, 0.001, 2000.)
            || !between(self.hold_ms, 0., 2000.)
            || !between(self.exit_ms, 0.001, 2000.)
            || !between(self.opening_ms + self.hold_ms + self.exit_ms, 1500., 2000.)
            || !between(self.backdrop_enter_ms, 0., self.opening_ms)
            || !between(self.wave_start_ms, 0., self.opening_ms - 100.)
            || !between(self.intro_cell_px, 8., 24.)
            || self.intro_cell_px.fract() != 0.
            || !between(self.wave_opacity, 0., 0.4)
            || !between(self.backdrop_center_alpha, 0., 0.7)
            || !between(self.backdrop_edge_alpha, 0., 0.7)
            || !between(self.idle_cycle_ms, 1800., 4000.)
            || !between(self.idle_amplitude_rows, 0., 1.)
        {
            return Err("Invalid startup tuning".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Cinematic,
    CompactIntro,
    CompactWait,
    CompactError,
}

#[derive(Clone, PartialEq)]
pub(super) struct Request {
    pub(super) command: &'static str,
    pub(super) kind: Kind,
    pub(super) tuning: Tuning,
    pub(super) reduced_motion: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Status {
    Preparing,
    Active,
    Ended,
    Failed,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub(super) run_id: String,
    pub(super) compact_token: Option<String>,
    pub(super) revision: u64,
    pub(super) status: Status,
    pub(super) error: Option<String>,
}

pub(super) struct Record {
    pub(super) request: Request,
    pub(super) receipt: Receipt,
    pub(super) window: Option<WebviewWindow>,
    pub(super) awaiting_removal: bool,
    pub(super) deadline: Option<Instant>,
    pub(super) shown_at: Option<Instant>,
    pub(super) lease: Instant,
    #[cfg(debug_assertions)]
    pub(super) last_sample: Option<Instant>,
}
#[derive(Default)]
pub(super) struct Slot {
    pub(super) revision: u64,
    pub(super) record: Option<Record>,
}
impl Slot {
    fn duplicate(&self, revision: u64, request: &Request) -> Result<Option<Receipt>, String> {
        if revision < self.revision {
            return Err("stale_revision".into());
        }
        if revision == self.revision {
            return match &self.record {
                Some(record)
                    if record.receipt.revision == revision && &record.request == request =>
                {
                    Ok(Some(record.receipt.clone()))
                }
                _ => Err("revision_conflict".into()),
            };
        }
        Ok(None)
    }
}
pub(super) struct Retired {
    pub(super) window: WebviewWindow,
    pub(super) token: String,
}
#[derive(Default)]
pub(super) struct Registry {
    pub(super) production: Slot,
    pub(super) preview: Slot,
    pub(super) counter: u64,
    pub(super) retired: Vec<Retired>,
    pub(super) stopped: bool,
    pub(super) monitor: Option<tauri::Monitor>,
    pub(super) compact_positioned: bool,
}
impl Registry {
    pub(super) fn slot(&self, scope: Scope) -> &Slot {
        match scope {
            Scope::Production => &self.production,
            Scope::Preview => &self.preview,
        }
    }
    pub(super) fn slot_mut(&mut self, scope: Scope) -> &mut Slot {
        match scope {
            Scope::Production => &mut self.production,
            Scope::Preview => &mut self.preview,
        }
    }
    pub(super) fn begin(
        &mut self,
        scope: Scope,
        revision: u64,
        request: Request,
    ) -> Result<(Receipt, Option<Record>, bool), String> {
        request.tuning.validate()?;
        if let Some(receipt) = self.slot(scope).duplicate(revision, &request)? {
            return Ok((receipt, None, false));
        }
        self.counter += 1;
        let token = self.counter.to_string();
        let receipt = Receipt {
            run_id: token.clone(),
            compact_token: (scope == Scope::Production && request.kind != Kind::Cinematic)
                .then_some(token),
            revision,
            status: Status::Preparing,
            error: None,
        };
        let now = Instant::now();
        let slot = self.slot_mut(scope);
        slot.revision = revision;
        let previous = slot.record.replace(Record {
            request,
            receipt: receipt.clone(),
            window: None,
            awaiting_removal: false,
            deadline: Some(now + PREPARE),
            shown_at: None,
            lease: now + PREVIEW_LEASE,
            #[cfg(debug_assertions)]
            last_sample: None,
        });
        Ok((receipt, previous, true))
    }
    pub(super) fn matches(&self, scope: Scope, token: &str) -> bool {
        self.slot(scope).record.as_ref().is_some_and(|r| {
            r.receipt.run_id == token
                && matches!(r.receipt.status, Status::Preparing | Status::Active)
        })
    }
    pub(super) fn can_prepare_replacement(&self, scope: Scope, token: &str) -> bool {
        self.slot(scope).record.as_ref().is_some_and(|record| {
            record.receipt.run_id == token
                && record.receipt.status == Status::Preparing
                && record.awaiting_removal
                && record.window.is_none()
                && record
                    .deadline
                    .is_some_and(|deadline| deadline > Instant::now())
        })
    }
    pub(super) fn end(
        &mut self,
        scope: Scope,
        token: Option<&str>,
        barrier: Option<u64>,
        cinematic_only: Option<bool>,
    ) -> Option<(Receipt, Option<WebviewWindow>)> {
        let slot = self.slot_mut(scope);
        if barrier.is_some_and(|revision| revision < slot.revision) {
            return None;
        }
        if token.is_some_and(|token| {
            slot.record
                .as_ref()
                .is_none_or(|r| token != r.receipt.run_id)
        }) {
            return None;
        }
        if let Some(revision) = barrier {
            slot.revision = revision;
        }
        let record = slot.record.as_mut()?;
        if cinematic_only
            .is_some_and(|cinematic| cinematic != (record.request.kind == Kind::Cinematic))
        {
            return None;
        }
        if record.receipt.status == Status::Ended && record.window.is_none() {
            return None;
        }
        record.receipt.status = Status::Ended;
        record.deadline = None;
        Some((record.receipt.clone(), record.window.take()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Request {
        Request {
            command: "start_intro",
            kind: Kind::Cinematic,
            tuning: Tuning::default(),
            reduced_motion: false,
        }
    }
    #[test]
    fn identical_requests_keep_token_and_deadline() {
        let mut state = Registry::default();
        let first = state.begin(Scope::Production, 1, request()).unwrap().0;
        let deadline = state.production.record.as_ref().unwrap().deadline;
        let again = state.begin(Scope::Production, 1, request()).unwrap();
        assert!(!again.2);
        assert_eq!(first.run_id, again.0.run_id);
        assert_eq!(deadline, state.production.record.as_ref().unwrap().deadline);
        state.production.record.as_mut().unwrap().receipt.status = Status::Ended;
        assert_eq!(
            state
                .begin(Scope::Production, 1, request())
                .unwrap()
                .0
                .status,
            Status::Ended
        );
    }
    #[test]
    fn conflicts_barriers_and_namespaces_are_independent() {
        let mut state = Registry::default();
        state.begin(Scope::Production, 5, request()).unwrap();
        let mut changed = request();
        changed.kind = Kind::CompactWait;
        assert!(state.begin(Scope::Production, 5, changed).is_err());
        assert!(state.begin(Scope::Production, 4, request()).is_err());
        assert!(state.begin(Scope::Preview, 1, request()).is_ok());
        state.production.revision = 6;
        assert!(state.begin(Scope::Production, 5, request()).is_err());
        assert!(state.begin(Scope::Production, 6, request()).is_err());
        assert!(state.begin(Scope::Production, 7, request()).is_ok());
    }
    #[test]
    fn cancelling_before_creation_blocks_late_requests() {
        let mut state = Registry::default();
        assert!(state
            .end(Scope::Production, None, Some(2), Some(true))
            .is_none());
        assert!(state.begin(Scope::Production, 1, request()).is_err());
        assert!(state.begin(Scope::Production, 3, request()).is_ok());
    }
    #[test]
    fn old_token_or_barrier_cannot_cancel_new_wait() {
        let mut state = Registry::default();
        let old = state.begin(Scope::Production, 1, request()).unwrap().0;
        let mut waiting = request();
        waiting.kind = Kind::CompactWait;
        let current = state.begin(Scope::Production, 3, waiting).unwrap().0;
        assert!(state
            .end(Scope::Production, Some(&old.run_id), Some(9), None)
            .is_none());
        assert_eq!(state.production.revision, 3);
        assert!(state.end(Scope::Production, None, Some(2), None).is_none());
        assert!(state.matches(Scope::Production, &current.run_id));
        assert!(state
            .end(Scope::Production, Some(&current.run_id), None, Some(true))
            .is_none());
        assert!(state
            .end(Scope::Production, Some(&current.run_id), None, Some(false))
            .is_some());
        assert!(!state.matches(Scope::Production, &current.run_id));
        assert!(state
            .end(Scope::Production, Some(&current.run_id), None, None)
            .is_none());
    }
    #[test]
    fn failed_duplicate_preserves_original_error_without_retry() {
        let mut state = Registry::default();
        state.begin(Scope::Production, 1, request()).unwrap();
        let record = state.production.record.as_mut().unwrap();
        record.receipt.status = Status::Failed;
        record.receipt.error = Some("unsupported transparency".into());
        record.deadline = None;
        let duplicate = state.begin(Scope::Production, 1, request()).unwrap();
        assert!(!duplicate.2);
        assert_eq!(duplicate.0.status, Status::Failed);
        assert_eq!(
            duplicate.0.error.as_deref(),
            Some("unsupported transparency")
        );
        assert!(state.production.record.as_ref().unwrap().deadline.is_none());
    }
    #[test]
    fn deferred_replacement_cannot_resurrect_cancelled_expired_or_active_runs() {
        let mut state = Registry::default();
        let first = state.begin(Scope::Preview, 1, request()).unwrap().0;
        state.preview.record.as_mut().unwrap().awaiting_removal = true;
        assert!(state.can_prepare_replacement(Scope::Preview, &first.run_id));
        let deadline = state.preview.record.as_ref().unwrap().deadline;
        state.begin(Scope::Preview, 1, request()).unwrap();
        assert_eq!(state.preview.record.as_ref().unwrap().deadline, deadline);
        state.preview.record.as_mut().unwrap().receipt.status = Status::Active;
        assert!(!state.can_prepare_replacement(Scope::Preview, &first.run_id));
        state.preview.record.as_mut().unwrap().receipt.status = Status::Preparing;
        state.preview.record.as_mut().unwrap().deadline =
            Some(Instant::now() - Duration::from_millis(1));
        assert!(!state.can_prepare_replacement(Scope::Preview, &first.run_id));
        let next = state.begin(Scope::Preview, 2, request()).unwrap().0;
        state.preview.record.as_mut().unwrap().awaiting_removal = true;
        assert!(!state.can_prepare_replacement(Scope::Preview, &first.run_id));
        assert!(state.can_prepare_replacement(Scope::Preview, &next.run_id));
        state.end(Scope::Preview, None, Some(3), None);
        assert!(!state.can_prepare_replacement(Scope::Preview, &next.run_id));
    }
    #[test]
    fn tuning_rejects_nonfinite_and_invalid_duration() {
        assert!(Tuning::default().validate().is_ok());
        assert!(Tuning {
            wave_opacity: f64::NAN,
            ..Tuning::default()
        }
        .validate()
        .is_err());
        assert!(Tuning {
            hold_ms: 5000.,
            ..Tuning::default()
        }
        .validate()
        .is_err());
    }
}
