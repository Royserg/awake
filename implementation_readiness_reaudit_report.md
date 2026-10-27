# Cinematic eye startup: implementation readiness re-audit

- **Date:** 2026-10-27.
- **Target:** [Updated design](./docs/designs/cinematic-eye-startup.md), lines 542–942, `Detailed Implementation Plan`.
- **Target SHA-256:** `53d96458c93b2475d947378b7399f1ef02039c432bcf14419dc4a525d1e3b509`.
- **Scope:** EGM Stage 9 re-audit, checking the six prior findings and material regressions.
- **Result:** All six original findings are closed at the document level. One new Major portability issue remains; the plan does not yet pass the combined readiness review.
- **Counts:** 0 Blockers, 1 Major, 0 Nits.
- **Changes applied:** None to the design or application source. No runtime acceptance checks were run.
- **Subsequent resolution:** The user requested a portability repair after this review. RA-01 is addressed in the [resolution record](./goldfish_review_report.md) and independently confirmed in the [portability readiness report](./implementation_readiness_portability_report.md) with zero findings. The findings below describe this report's reviewed hash. Desktop validation remains pending.

Three independent reviewers ran in fresh contexts: implementation readiness,
codebase details, and style. The readiness and style reviewers reported no
actionable findings. The codebase reviewer identified the platform-specific
behavior below. The finding is retained because it is supported by the locked
dependency source, rather than decided by a majority of reviewer conclusions.

## Closure of original findings

| Finding | Status | Evidence in the updated design |
| --- | --- | --- |
| IR-01: equal-revision behavior | Closed | Lines 726–762 define identical/conflicting duplicates, lower revisions, retained terminal records, independent namespaces, and no resurrection/deadline renewal. Line 885 adds race assertions. |
| IR-02: macOS compact show | Closed for its original macOS scope | Lines 780–796 specify the synchronous macOS mechanism and error paths; lines 892 and 912–914 require policy and desktop focus checks. Applying this mechanism across platforms creates the separate finding below. |
| IR-03: implementation checkpoints | Closed | Lines 596–606 define four stable phases and checks; production remains intact through Phases 1–2, with route and visibility changes coordinated in Phase 3. |
| IR-04: cell-size units | Closed | Line 619 distinguishes CSS artwork pixels from physical monitor bounds. |
| IR-05: completion subject | Closed | Lines 708–713 explicitly identify visual completion and separate update-resource lifetime. |
| IR-06: contextual retry actions | Closed | Lines 844–854 distinguish Try again, Restart, and installation-failure Hide; line 890 requires matching preview assertions. |

All three reviewers independently confirmed these closures. Closure here means
that the plan supplies the missing decisions, not that those decisions have
been implemented or passed desktop tests.

## New finding

### RA-01 — Use platform-specific nonactivating compact shows — Major

- **Reviewer lens:** Codebase details.
- **Affected scope:** Design lines 780–796 and the universal test assertion at line 892.
- **Discrepancy:** The immediate `set_focusable(false)` → `show()` → restore
  sequence is prescribed for production and compact preview windows without a
  platform restriction. It addresses macOS's locked implementation, but the
  Windows and Linux implementations behave differently.
- **Consequence:** The specified helper does not establish the required
  nonactivating presentation on Windows/Linux. It can request activation on
  Windows and remove the protective focus hint before mapping on Linux.
  Observed focus changes remain untested and depend on desktop policy.

### Windows source evidence

The locked tao 0.37.2 implementation of
[set_focusable](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/windows/window.rs:153)
changes window flags. Nonfocusability sets
[WS_EX_NOACTIVATE](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/windows/window_state.rs:293).
Restoring focusability removes that style; the same flag update then calls
[ShowWindow for a visible window](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/windows/window_state.rs:457)
with `SW_SHOW`. Its construction-only suppression marker was already
[removed](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/windows/window.rs:1415).
Microsoft documents `SW_SHOW` as an activating show operation.
[ShowWindow reference](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow).

### Linux source evidence

The locked tao implementation
[queues visibility](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/linux/window.rs:573)
through a GLib channel, while
[set_focusable](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/linux/window.rs:593)
changes GTK `accept_focus` immediately. The channel receiver later invokes
[show_all](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/linux/event_loop.rs:299).
The immediate restore therefore happens before the queued native show. GTK
provides distinct hints for accepting input focus and requesting focus when a
window is mapped.
[GTK accept-focus](https://docs.gtk.org/gtk3/method.Window.set_accept_focus.html),
[GTK focus-on-map](https://docs.gtk.org/gtk3/method.Window.set_focus_on_map.html).

Wry's
[main-thread dispatch](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-runtime-wry-2.12.3/src/lib.rs:263)
handles the Tauri operations synchronously into tao; it does not wait for
Linux's subsequently queued visibility operation. The primary agent
independently checked the critical Windows/Linux/Wry excerpts and official
documentation supporting this finding.

### Recommended correction and verification

Keep the macOS sequence. Specify separate Windows/Linux mechanisms that show
without activation while retaining deliberate keyboard/mouse interaction:

- Windows must avoid tao's activating focusability-restoration path for an
  already-visible window.
- Linux must suppress focus at the actual queued mapping operation, rather
  than restore the hint immediately after queuing it.
- Update the implementation file/dependency inventory if native access needs
  additional files or target-specific dependencies.
- Replace the universal disable/show/restore test assertion with
  platform-specific assertions. Keep the timer-focused and other-app-focused
  desktop acceptance cases on each release platform, followed by deliberate
  compact-window interaction.

This is a correction to the prescribed mechanism, not a request to expand the
feature. No platform implementation was chosen or changed during this review.

## Read completion and review evidence

Reviewers `/root/eye_readiness_reaudit`, `/root/eye_codebase_reaudit`, and
`/root/eye_style_reaudit` were spawned with `fork_turns: "none"` and all returned
final reports and read-completion manifests. Each read its persona, the Codex
integration guidance, the whole target, and all 16 existing files in
`/tmp/awake-eye-reaudit.hk4j3dsk/reading-list.md`. The readiness and codebase
reviewers additionally read the implementation-readiness standard; no separate
style standard was mapped.

The common existing-file context was:

- `README.md`, `concepts_brief.md`, `goldfish_review_report.md`,
  `implementation_readiness_report.md`, and `package.json`;
- `src-tauri/Cargo.toml`, `src-tauri/README.md`, `src-tauri/src/README.md`,
  `src-tauri/src/lib.rs`, and `src-tauri/tauri.conf.json`;
- `src/README.md`, `src/app.html`, `src/routes/+layout.ts`,
  `src/routes/+page.svelte`, `src/routes/README.md`, and
  `src/routes/eye-splashscreen.ts`.

All common context paths resolve from the Awake repository root. Relevant
installed schemas/dependency sources, TypeScript configuration, capabilities,
and lockfile entries were also inspected. All reviewers opened the relevant
sections of the four primary URLs referenced by the design: Tauri API
changelog, updater reference, Node native TypeScript guidance, and Tauri
capabilities. The codebase reviewer additionally inspected official GTK and
Microsoft documentation for RA-01.

Read tools were `exec_command` with `cat`, `nl`, bounded `sed`, and `rg`, plus
`web__run` open/find/click. Initial output truncation was recovered with smaller
reads. A GTK GitLab HTTP 406 response was worked around by reading the same
upstream source through GNOME's official GitHub mirror. No required context
read remained unavailable. Proposed files were not classified as missing
implementation.

No reviewer modified files, ran builds/tests, spawned additional agents, or
retrieved conversation history. The primary agent verified that the reviewed
design hash remained unchanged before saving this report.

## Acceptance work remains separate

Native transparency/focus behavior, long-hidden downloads, packaged update
delivery, Windows installer handoff, and visual approval in replay still need
implementation-time validation. This re-audit does not claim those tests passed.
This completed the requested single re-audit with RA-01 open at that time.
Its subsequent document resolution is linked above.
