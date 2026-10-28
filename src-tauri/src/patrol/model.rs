//! Wire contracts and platform-independent route geometry. No native side effects.
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::BTreeSet, fmt, num::NonZeroU64};

pub const DEFAULT_PAUSE_MS: u64 = 10_000;
pub const MIN_POINTS: usize = 2;
pub const MAX_POINTS: usize = 256;
pub const TRAVEL_MS: u64 = 500;
pub const HEALTH_MS: u64 = 50;
pub const LEASE_MS: u64 = 100;
pub const DISPLAY_CHECK_MS: u64 = 250;
pub const PREPARATION_MS: u64 = 2_000;
pub const SAMPLE_NS: u64 = 16_666_667;
pub const STALL_MS: u64 = 150;

fn decimal(value: &str) -> Result<u64, &'static str> {
    if value.is_empty()
        || value.len() > 20
        || !value.bytes().all(|c| c.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err("Expected a canonical u64 decimal string");
    }
    value.parse().map_err(|_| "Decimal counter overflow")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(NonZeroU64);
impl Id {
    pub fn new(value: u64) -> Result<Self, &'static str> {
        NonZeroU64::new(value).map(Self).ok_or("ID must be nonzero")
    }
    pub fn get(self) -> u64 {
        self.0.get()
    }
}
impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Serialize for Id {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::new(decimal(&text).map_err(de::Error::custom)?).map_err(de::Error::custom)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counter(pub u64);
impl Serialize for Counter {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for Counter {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        decimal(&String::deserialize(deserializer)?)
            .map(Self)
            .map_err(de::Error::custom)
    }
}
#[derive(Default)]
pub struct Ids(u64);
impl Ids {
    pub fn next_id(&mut self) -> Result<Id, ErrorCode> {
        self.0 = self.0.checked_add(1).ok_or(ErrorCode::InternalError)?;
        Id::new(self.0).map_err(|_| ErrorCode::InternalError)
    }
}
pub fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}
fn finite<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let value = f64::deserialize(deserializer)?;
    if value.is_finite() {
        Ok(if value == 0.0 { 0.0 } else { value })
    } else {
        Err(de::Error::custom("Non-finite coordinate"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XY {
    #[serde(deserialize_with = "finite")]
    pub x: f64,
    #[serde(deserialize_with = "finite")]
    pub y: f64,
}
impl XY {
    pub fn valid(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    #[serde(deserialize_with = "finite")]
    pub x: f64,
    #[serde(deserialize_with = "finite")]
    pub y: f64,
    #[serde(deserialize_with = "finite")]
    pub width: f64,
    #[serde(deserialize_with = "finite")]
    pub height: f64,
}
impl Rect {
    pub fn valid(self) -> bool {
        [
            self.x,
            self.y,
            self.width,
            self.height,
            self.x + self.width,
            self.y + self.height,
        ]
        .iter()
        .all(|v| v.is_finite())
            && self.width > 0.0
            && self.height > 0.0
    }
    pub fn contains(self, p: XY) -> bool {
        self.valid()
            && p.valid()
            && p.x >= self.x
            && p.y >= self.y
            && p.x < self.x + self.width
            && p.y < self.y + self.height
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub origin: XY,
    pub width: f64,
    pub height: f64,
}
impl Mapping {
    pub fn rect(self) -> Rect {
        Rect {
            x: self.origin.x,
            y: self.origin.y,
            width: self.width,
            height: self.height,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Unit {
    MacosPoint,
    WindowsPixel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Stopped,
    Editing,
    Starting,
    Running,
    Stopping,
    ReviewRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartStage {
    PreparingEdits,
    Arming,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    Escape,
    ManualMovement,
    MouseButton,
    TrayStop,
    AwakeOff,
    TimerExpired,
    DisplayChanged,
    SessionInactive,
    SecureInput,
    ObserverLost,
    AvailabilityExpired,
    PermissionLost,
    CursorConstrained,
    NativeFailure,
    ExecutionStalled,
    AppExit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    UnsupportedPlatform,
    NativeUnavailable,
    PermissionRequired,
    InvalidPayload,
    InvalidRoute,
    Forbidden,
    StaleSetup,
    StaleSnapshot,
    StaleAction,
    StaleRun,
    StalePreparation,
    ReviewRequired,
    AlreadyRunning,
    Busy,
    StartCanceled,
    EditIncomplete,
    EditPreparationTimedOut,
    TimerExpired,
    AwakeActivationFailed,
    AwakeDeactivationFailed,
    OverlayFailed,
    CursorConstrained,
    StopPending,
    InternalError,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub code: ErrorCode,
    pub message: String,
}
impl Failure {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewPair {
    pub route_revision: Id,
    pub layout_revision: Id,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Display {
    pub display_id: Id,
    pub name: String,
    pub native_bounds: Rect,
    pub unit: Unit,
    pub scale: XY,
    pub rotation_degrees: f64,
    #[serde(deserialize_with = "required_nullable")]
    pub mirror_of: Option<Id>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Point {
    pub point_id: Id,
    pub display_id: Id,
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InvalidPointReason {
    DisplayMissing,
    OutsideDisplay,
    InvalidCoordinate,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvalidPoint {
    pub point_id: Id,
    pub reason: InvalidPointReason,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationError {
    PointCount,
    DistinctPositions,
    InvalidPoints,
    Pause,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteValidation {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EditReason {
    InvalidDrop,
    CaptureLost,
    MutationFailed,
    NotResponding,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EditState {
    Initializing,
    Ready,
    Editing,
    Incomplete,
    Unresponsive,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReportedEditState {
    Ready,
    Editing,
    Incomplete,
}
impl From<ReportedEditState> for EditState {
    fn from(value: ReportedEditState) -> Self {
        match value {
            ReportedEditState::Ready => Self::Ready,
            ReportedEditState::Editing => Self::Editing,
            ReportedEditState::Incomplete => Self::Incomplete,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverlayEdit {
    pub overlay_id: Id,
    pub display_id: Id,
    pub state: EditState,
    pub point_ids: Vec<Id>,
    #[serde(deserialize_with = "required_nullable")]
    pub reason: Option<EditReason>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preparation {
    pub preparation_id: Id,
    pub requester_overlay_id: Id,
    pub request_id: Id,
    pub route_revision: Id,
    pub layout_revision: Id,
    pub pending_overlay_ids: Vec<Id>,
    pub ready_overlay_ids: Vec<Id>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Review {
    pub required: bool,
    #[serde(deserialize_with = "required_nullable")]
    pub confirmed_for: Option<ReviewPair>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Setup {
    pub setup_id: Id,
    pub route_revision: Id,
    pub layout_revision: Id,
    pub points: Vec<Point>,
    pub pause_ms: u64,
    pub displays: Vec<Display>,
    pub invalid_points: Vec<InvalidPoint>,
    pub route_validation: RouteValidation,
    pub review: Review,
    pub overlays: Vec<OverlayEdit>,
    #[serde(deserialize_with = "required_nullable")]
    pub preparation: Option<Preparation>,
}
impl Setup {
    pub fn pair(&self) -> ReviewPair {
        ReviewPair {
            route_revision: self.route_revision,
            layout_revision: self.layout_revision,
        }
    }
    pub fn revalidate(&mut self) {
        let (validation, invalid) = validate_route(&self.points, self.pause_ms, &self.displays);
        self.route_validation = validation;
        self.invalid_points = invalid;
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Status {
    pub phase: Phase,
    #[serde(deserialize_with = "required_nullable")]
    pub start_stage: Option<StartStage>,
    #[serde(deserialize_with = "required_nullable")]
    pub run_id: Option<Id>,
    #[serde(deserialize_with = "required_nullable")]
    pub stop_reason: Option<StopReason>,
    #[serde(deserialize_with = "required_nullable")]
    pub last_failure: Option<Failure>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Awake {
    pub active: bool,
    #[serde(deserialize_with = "required_nullable")]
    pub timer_remaining_ms: Option<Counter>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AvailabilityState {
    Checking,
    Ready,
    UnsupportedPlatform,
    PermissionRequired,
    NativeUnavailable,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Availability {
    pub state: AvailabilityState,
    #[serde(deserialize_with = "required_nullable")]
    pub message: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Caller {
    pub overlay_id: Id,
    pub display_id: Id,
    pub is_control_panel: bool,
    pub mapping: Mapping,
    pub last_accepted_action_seq: Counter,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub state_version: Id,
    pub status: Status,
    pub awake: Awake,
    pub availability: Availability,
    #[serde(deserialize_with = "required_nullable")]
    pub setup: Option<Setup>,
    #[serde(deserialize_with = "required_nullable")]
    pub caller: Option<Caller>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Receipt {
    Update {
        setup_id: Id,
        overlay_id: Id,
        action_seq: Id,
    },
    Start {
        setup_id: Id,
        overlay_id: Id,
        request_id: Id,
        run_id: Id,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub snapshot: Snapshot,
    #[serde(deserialize_with = "required_nullable")]
    pub receipt: Option<Receipt>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatrolError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(deserialize_with = "required_nullable")]
    pub snapshot: Option<Box<Snapshot>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreparationOutcome {
    Ready,
    Incomplete,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Action {
    AddPoint {
        #[serde(deserialize_with = "finite")]
        u: f64,
        #[serde(deserialize_with = "finite")]
        v: f64,
    },
    MovePoint {
        point_id: Id,
        #[serde(deserialize_with = "finite")]
        u: f64,
        #[serde(deserialize_with = "finite")]
        v: f64,
    },
    RemovePoint {
        point_id: Id,
    },
    SetPause {
        pause_ms: u64,
    },
    ConfirmReview {},
    ReportEditState {
        state: ReportedEditState,
        point_ids: Vec<Id>,
        #[serde(deserialize_with = "required_nullable")]
        reason: Option<EditReason>,
    },
    AckStartPreparation {
        preparation_id: Id,
        outcome: PreparationOutcome,
        drained_through: Counter,
    },
}
impl Action {
    pub fn metadata(&self) -> bool {
        matches!(
            self,
            Self::ReportEditState { .. } | Self::AckStartPreparation { .. }
        )
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateInput {
    pub setup_id: Id,
    pub route_revision: Id,
    pub layout_revision: Id,
    pub action_seq: Id,
    pub action: Action,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartInput {
    pub setup_id: Id,
    pub route_revision: Id,
    pub layout_revision: Id,
    pub request_id: Id,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupInput {
    pub setup_id: Id,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopInput {
    pub run_id: Id,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyInput {}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateChanged {
    pub setup_id: Id,
    pub state_version: Id,
}

pub fn pause_valid(value: u64) -> bool {
    (1_000..=3_600_000).contains(&value) && value.is_multiple_of(1_000)
}
fn normalize(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}
pub fn quantize(value: f64) -> Option<f64> {
    let result = (value + 0.5).floor();
    (result.is_finite() && result >= i32::MIN as f64 && result <= i32::MAX as f64)
        .then_some(normalize(result))
}
pub fn canonical_position(raw: XY, displays: &[Display]) -> Option<(Id, XY)> {
    let display = displays
        .iter()
        .filter(|d| d.mirror_of.is_none() && d.native_bounds.contains(raw))
        .min_by_key(|d| d.display_id)?;
    let position = match display.unit {
        Unit::MacosPoint => XY {
            x: normalize(raw.x),
            y: normalize(raw.y),
        },
        Unit::WindowsPixel => XY {
            x: quantize(raw.x)?,
            y: quantize(raw.y)?,
        },
    };
    display
        .native_bounds
        .contains(position)
        .then_some((display.display_id, position))
}
pub fn resolve_sample(mapping: Mapping, u: f64, v: f64, displays: &[Display]) -> Option<(Id, XY)> {
    if !mapping.rect().valid() || !u.is_finite() || !v.is_finite() {
        return None;
    }
    canonical_position(
        XY {
            x: mapping.origin.x + u * mapping.width,
            y: mapping.origin.y + v * mapping.height,
        },
        displays,
    )
}
pub fn resolve_point(point: &Point, displays: &[Display]) -> Result<XY, InvalidPointReason> {
    let display = displays
        .iter()
        .find(|d| d.display_id == point.display_id && d.mirror_of.is_none())
        .ok_or(InvalidPointReason::DisplayMissing)?;
    if !point.x.is_finite() || !point.y.is_finite() {
        return Err(InvalidPointReason::InvalidCoordinate);
    }
    let raw = XY {
        x: display.native_bounds.x + point.x,
        y: display.native_bounds.y + point.y,
    };
    if !display.native_bounds.contains(raw) {
        return Err(InvalidPointReason::OutsideDisplay);
    }
    let position = match display.unit {
        Unit::MacosPoint => XY {
            x: normalize(raw.x),
            y: normalize(raw.y),
        },
        Unit::WindowsPixel => XY {
            x: quantize(raw.x).ok_or(InvalidPointReason::InvalidCoordinate)?,
            y: quantize(raw.y).ok_or(InvalidPointReason::InvalidCoordinate)?,
        },
    };
    if !display.native_bounds.contains(position) {
        return Err(InvalidPointReason::OutsideDisplay);
    }
    Ok(position)
}
pub fn validate_route(
    points: &[Point],
    pause: u64,
    displays: &[Display],
) -> (RouteValidation, Vec<InvalidPoint>) {
    let mut errors = vec![];
    let mut invalid = vec![];
    let mut distinct = BTreeSet::new();
    if !(MIN_POINTS..=MAX_POINTS).contains(&points.len()) {
        errors.push(ValidationError::PointCount);
    }
    for point in points {
        match resolve_point(point, displays) {
            Ok(p) => {
                distinct.insert((normalize(p.x).to_bits(), normalize(p.y).to_bits()));
            }
            Err(reason) => invalid.push(InvalidPoint {
                point_id: point.point_id,
                reason,
            }),
        }
    }
    if distinct.len() < 2 {
        errors.push(ValidationError::DistinctPositions);
    }
    if !invalid.is_empty() {
        errors.push(ValidationError::InvalidPoints);
    }
    if !pause_valid(pause) {
        errors.push(ValidationError::Pause);
    }
    (
        RouteValidation {
            valid: errors.is_empty(),
            errors,
        },
        invalid,
    )
}

/// Choose one canonical control surface, including when the OS primary is a mirrored member.
pub fn control_display(
    displays: &[Display],
    cursor: Option<XY>,
    primary: Option<Id>,
) -> Option<Id> {
    let canonical_primary = primary
        .and_then(|id| displays.iter().find(|d| d.display_id == id))
        .map(|d| d.mirror_of.unwrap_or(d.display_id));
    displays
        .iter()
        .filter(|d| d.mirror_of.is_none() && cursor.is_some_and(|p| d.native_bounds.contains(p)))
        .map(|d| d.display_id)
        .min()
        .or_else(|| {
            canonical_primary.filter(|id| {
                displays
                    .iter()
                    .any(|d| d.display_id == *id && d.mirror_of.is_none())
            })
        })
        .or_else(|| {
            displays
                .iter()
                .filter(|d| d.mirror_of.is_none())
                .map(|d| d.display_id)
                .min()
        })
}
