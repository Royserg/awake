# Goldfish Annotated Review Report

**Status:** resolved

**Subsequent re-audit:** All six original findings were independently confirmed
closed. The new Major issue RA-01 from the
[re-audit report](./implementation_readiness_reaudit_report.md) is also addressed
in the separate portability resolution below. The latest changes have not had
desktop validation. A [fresh portability readiness review](./implementation_readiness_portability_report.md)
has now independently confirmed both repairs with zero findings.

**Date:** 2026-10-27

**Target:** [Cinematic eye startup design](./docs/designs/cinematic-eye-startup.md),
`Detailed Implementation Plan`: original lines 540–875, first repair lines
542–942, current lines 542–1016. The six IR entries retain their first-repair
line references; RA-01 uses current references.

**Inputs:** [Implementation readiness report](./implementation_readiness_report.md)
and [portability re-audit](./implementation_readiness_reaudit_report.md).

**Authorization:** The user requested resolution of the findings and updates to
the design, then explicitly requested separate Windows and Linux handling for
RA-01. All seven findings are classified `PROPOSE FIX` and applied. None are
contested, skipped, or left unaddressed. The original six closures received one
independent re-audit. The subsequent portability edit records document
resolution only; implementation, runtime validation, and visual acceptance
remain pending.

## Prioritized findings and resolutions

### IR-01 — Equal-revision request behavior [Style] — Major

- **Original scope:** Lines 711–716, native command table.
- **Discrepancy:** Lower revisions were rejected, but equal revisions had no
  defined meaning.
- **Rationale:** An identical retry could allocate another run, reset a deadline,
  or resurrect an ended presentation, undermining the stale-callback rules.
- **AI suggestion:** `PROPOSE FIX`.
- **Justification:** This is a missing protocol decision, not a change to the
  approved presentation architecture.
- **Applied edit:** The native protocol retains the highest accepted revision,
  command, validated payload, token, status, and failure error within each
  namespace. Lower revisions return `stale_revision`. Equal revisions with
  different requests return `revision_conflict`. Identical duplicates return
  the recorded token and current `preparing`, `active`, `ended`, or `failed`
  status without native work or deadline renewal. Completed/cancelled/failed
  presentations cannot be resurrected; new attempts require a higher revision.
  Preview follows the same rule without sharing production ownership.
- **Updated scope:** Native protocol, lines 726–762; race assertions, line 885.
- **Verification specified:** Identical/conflicting duplicates and duplicates
  after completion/cancellation/failure; token, status, deadline, and lease
  preservation in production and preview namespaces.

### IR-02 — Nonactivating compact show on macOS [Codebase] — Major

- **Original scope:** Line 563 and lines 715–738.
- **Discrepancy:** Initial `focus: false` does not control later macOS `show()`
  calls, which request key-window status in the locked implementation.
- **Rationale:** A late update could move focus away from Awake's timer or
  another application, contrary to the approved behavior.
- **AI suggestion:** `PROPOSE FIX`.
- **Justification:** The original review cites the exact locked Tauri/tao
  implementation. A supported native show sequence resolves the mechanism
  while retaining the compact window's intentional interactivity.
- **Applied edit:** For hidden-to-visible compact transitions, a main-thread
  helper saves prior focusability, disables focusability, shows, and restores
  the prior value. It does not call `set_focus`. Already-visible content changes
  skip another show. Failure to disable prevents showing; show errors still
  attempt restoration; restoration/show failure records the actual state,
  hides newly shown UI, and leaves update work running. Compact preview uses
  its own equivalent helper; cinematic views remain nonfocusable.
- **Updated scope:** Native lifetime, lines 780–797; focus assertions, line 892;
  native acceptance, lines 912–914.
- **Verification specified:** Disable/show/restore ordering and error paths,
  no redundant show, timer-focused and other-app-focused late updates, and
  deliberate interaction with the compact UI afterward. Desktop validation
  remains pending.

### IR-03 — Stable implementation checkpoints [Readiness] — Nit

- **Original scope:** Lines 544–585.
- **Discrepancy:** The ordered file list did not mark intermediate stable,
  buildable, testable phases.
- **Rationale:** The readiness standard requires incremental verification;
  the implementer should not have to choose integration boundaries.
- **AI suggestion:** `PROPOSE FIX`.
- **Justification:** Grouping the existing tasks is sufficient. No new file,
  dependency, feature, or architectural component is needed.
- **Applied edit:** Task numbers identify scope; four phases define execution
  order: shared artwork/motion, simulated browser replay, native/production
  integration, and acceptance/documentation. Each phase lists its tasks and
  exact checks. The production startup route and initial visibility change
  together only after native integration is ready. Early tests run only files
  already created; all four test files exist before package scripts are added.
- **Updated scope:** Task-list introduction and phase table, lines 546–606.
- **Verification specified:** Existing startup stays usable in Phases 1–2;
  browser preview remains simulated; Phase 3 runs frontend/native checks;
  Phase 4 records actual platform and visual acceptance evidence.

### IR-04 — Cell-size units [Style] — Nit

- **Original scope:** Line 604.
- **Discrepancy:** Tuning called these pixels without naming the coordinate
  system.
- **Rationale:** Artwork coordinates must not be confused with native monitor
  bounds on scaled displays.
- **AI suggestion:** `PROPOSE FIX`.
- **Justification:** CSS pixels were already the intended artwork units.
- **Applied edit:** `introCellPx` defaults to 15 CSS pixels, accepts integer
  8–24 CSS pixels, and compact cells use 15 CSS pixels. Native monitor bounds
  remain physical pixels.
- **Updated scope:** Tuning table, line 619.

### IR-05 — Visual completion subject [Style] — Nit

- **Original scope:** Lines 693–698.
- **Discrepancy:** “Hide/Stop/completion” did not identify which work completed.
- **Rationale:** Its placement after update-completion rules could be misread
  as update completion rather than visual completion.
- **AI suggestion:** `PROPOSE FIX`.
- **Justification:** This is a wording clarification of existing ownership.
- **Applied edit:** “Hide, preview Stop, and visual completion dispose only
  presentation work.” Existing update-resource release rules remain separate.
- **Updated scope:** Controller cleanup, lines 708–713.

### IR-06 — Contextual preview retry actions [Style] — Nit

- **Original scope:** Lines 786–791, with controller rules at 656 and 686–689.
- **Discrepancy:** Generic Retry did not distinguish download and restart
  failure actions.
- **Rationale:** Preview must exercise the same permitted recovery operations
  as production and must not imply that a partial installation is retried.
- **AI suggestion:** `PROPOSE FIX`.
- **Justification:** The architecture already defines Try again for download
  failure, Restart for relaunch failure, and Hide for installation failure.
- **Applied edit:** Try again calls `retryDownload` for a new check/download
  attempt; Restart calls `retryRestart` for relaunch only; installation failure
  offers Hide without retry. Tests explicitly check these distinctions.
- **Updated scope:** Replay controls, lines 844–855; preview assertions, line 890.

## Portability resolution after re-audit

### RA-01 — Platform-specific compact showing [Codebase] — Major

- **Reviewed scope:** Lines 780–796 and 892 of the re-audited version.
- **Discrepancy:** The macOS disable/show/restore sequence was prescribed on all
  three platforms. Windows restoration requests an activating show; Linux
  restoration happens before its queued native map.
- **Rationale:** One platform's setter/show ordering cannot establish the
  cross-platform nonactivating behavior required by the design.
- **AI suggestion:** `PROPOSE FIX`.
- **Justification:** The exact locked sources and official API references in
  the re-audit support the finding. Separate branches resolve it without
  changing the approved cinematic/update flow or forking native dependencies.
- **Applied edit:**
  - **macOS:** Retain the verified synchronous disable/show/restore branch.
  - **Windows:** Keep normal focusability and Tauri visibility bookkeeping.
    Surround synchronous Tauri show with a target-specific activation guard
    scoped to Awake's UI thread. The design specifies callback filtering,
    installation/removal, nesting and failure handling, and cleanup ownership.
    It does not use the activating focusability-restoration path or bypass
    tao's visibility state.
  - **Linux:** Set GTK mapping focus false and input-focus acceptance true
    before showing. Keep those policies through mapping and later shows; no
    immediate restoration or extra map/draw callbacks are needed.
  - **Inventory:** Tasks 35–36 add the planned target-specific GTK/Windows
    direct dependencies and Cargo lockfile refresh. The versions match the
    existing lockfile: GTK 0.18.2 and Windows bindings 0.62.2. Frontend lockfile,
    Tauri/tao versions, icons, and timer behavior retain their existing scope.
  - **Verification:** Replace the universal order assertion with shared and
    platform-specific policy/error-path tests. Require native compilation/tests
    on each release OS, Linux X11/Wayland focus checks, Windows guard cleanup
    and visibility/DPI consistency, and deliberate interaction after show.
- **Current scope:** File tasks 35–36 at lines 589–590; Phase 3 and target
  dependency rules at lines 608–614; platform helpers at lines 788–852;
  focus assertions at lines 948–951; native acceptance at lines 971–979.
- **Evidence checked during resolution:** Tauri 2.12.5 `hwnd()`/`gtk_window()`,
  Wry 2.12.3 main-thread dispatch, tao 0.37.2 Windows thread executor and
  platform visibility/focus paths, the existing Cargo lockfile, and official
  [Windows callback](https://learn.microsoft.com/en-us/windows/win32/winmsg/cbtproc),
  [thread-hook installation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw),
  [hook cleanup](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unhookwindowshookex),
  and [GTK mapping-focus](https://docs.gtk.org/gtk3/method.Window.set_focus_on_map.html)
  documentation. The pinned locked sources establish the synchronous Windows
  scope assumption. Windows/Linux desktop behavior is still untested.
- **Status:** Addressed in the design at the user's request, then independently
  confirmed by all three reviewers in the portability readiness report.
  Implementation and desktop acceptance remain pending.

## Synchronization and remaining acceptance work

The working [draft](./design_draft.md) is synchronized with the canonical design,
with its root-relative links adjusted. The initial readiness report remains a
historical review of the original hash and now links to this resolution record.
The implementation file inventory now contains 36 tasks, including the native
manifest and lockfile changes required by platform interop.

Markdown structure, relative link targets, task inventory, and all seven repairs
were checked after editing. No production source or unrelated documentation
was changed. The requested re-audit of the first repair and the explicitly
requested portability follow-up both completed. The latest review returned
zero findings at the design and source-compatibility level.

**Accepted fixes:** 7. **Contested:** 0. **Unaddressed findings:** 0.

Implementation, platform focus/transparency tests, long-hidden update tests,
packaged updater checks, and visual approval in the working replay remain
pending as specified in the design.
