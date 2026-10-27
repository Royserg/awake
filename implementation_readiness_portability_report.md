# Cinematic eye startup: portability readiness confirmation

- **Date:** 2026-10-27.
- **Target:** [Design](./docs/designs/cinematic-eye-startup.md), lines 542–1016, `Detailed Implementation Plan`.
- **Target SHA-256:** `ee3cd71a7ca28905bc5b707fd713dc3e8101a4c5f33dc6b5ab72588bbee7af33`.
- **Result:** PASS at the implementation-plan and source-compatibility level.
- **Findings:** 0 Blockers, 0 Majors, 0 Nits across all three reviewers.
- **RA-01:** Closed for both Windows and Linux.
- **Changes applied:** None to the design or application source.

The user explicitly requested this fresh review after the portability repair.
Three reviewers independently audited the current plan: implementation
readiness, codebase details, and style. Each returned a pass with no actionable
finding or material regression. Earlier reports remain historical records of
their respective design versions; this report records the latest result.

## Portability closure

| Area | Result | Evidence in the reviewed design |
| --- | --- | --- |
| Windows show behavior | Closed | Lines 804–832 specify the UI-thread-scoped activation guard around synchronous Tauri show, preserving focusability and tao visibility state. HWND/thread validation, callback filtering, forwarding, nesting rejection, cleanup, and removal-failure ownership are explicit. |
| Linux queued mapping | Closed | Lines 834–845 keep mapping focus disabled and input-focus acceptance enabled before and through mapping and later shows. The premature focusability restoration is removed. |
| Dependency and file scope | Complete | Tasks 35–36 and lines 608–614 specify target-specific GTK/Windows dependencies, their matching locked versions, Cargo lockfile handling, and conditional native imports. |
| Verification and acceptance | Complete as a plan | Lines 948–951 define platform-specific assertions. Lines 971–979 require focus, deliberate interaction, hide/re-show, Windows guard cleanup, visibility/DPI consistency, and Linux session checks. |

The original six repairs remain present. The platform helpers preserve the
presentation revision/cancellation rules, compact preview isolation, updater
ownership, and unchanged sleep/timer behavior. No additional design choice
requiring remediation was identified.

## Independent source checks

**Windows:** Reviewers followed the locked Tauri 2.12.5 accessor, runtime Wry
2.12.3 dispatcher, and tao 0.37.2 thread executor/visibility path. They confirmed
that a show already running on the owning UI thread reaches the native
operation synchronously and updates tao's stored visibility flags. That
supports the guard's prescribed lifetime without bypassing runtime state.
The callback and thread scope agree with the official
[CBT callback](https://learn.microsoft.com/en-us/windows/win32/winmsg/cbtproc),
[hook installation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw),
and [cleanup](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unhookwindowshookex)
contracts.

**Linux:** Reviewers followed Tauri's main-thread GTK accessor, tao's directly
applied focus-acceptance setter, and its queued visibility operation ending in
GTK `show_all()`. The persistent policies in the repair agree with GTK's
distinct [mapping-focus](https://docs.gtk.org/gtk3/method.Window.set_focus_on_map.html)
and [input-focus acceptance](https://docs.gtk.org/gtk3/method.Window.set_accept_focus.html)
contracts. They no longer rely on an immediate restoration preceding a queued
map. Window-manager behavior is still an explicit acceptance test.

The scoped lockfile records resolve GTK 0.18.2 and Windows bindings 0.62.2.
Installed Tauri/tao manifests use compatible GTK 0.18 and Windows 0.62
requirements. The planned feature groups cover the Windows API namespaces
used in the design; target conditions separate the native branches.

## Read completion and verification limits

All three reviewers were spawned with `fork_turns: "none"`, completed the
audit, and supplied read-completion manifests:

- `/root/eye_portability_readiness`: PASS; no findings.
- `/root/eye_portability_codebase`: PASS; no findings.
- `/root/eye_portability_style`: PASS; no findings.

Each read its persona, Codex integration guidance, the entire target, and all
applicable context in `/tmp/awake-eye-portability-review.u382ln5d/reading-list.md`.
Readiness and codebase reviewers also read the implementation-readiness
standard. No separate style standard was mapped.

The reading list contains 18 existing files: the root README, concept brief,
resolution report, two earlier readiness reports, package manifest, native
manifest and scoped lockfile records, both native READMEs, native runtime and
configuration, frontend README/document shell, layout, startup route, route
README, and eye renderer. All repository paths resolve from the Awake root.
All nine primary documentation URLs linked by the design were opened and
relevant sections inspected. Additional installed-source reads covered Tauri,
runtime Wry, tao Windows/Linux/macOS paths, schema, and configuration as needed.

Read tools were `exec_command` with `cat`, `nl`, bounded `sed`, `rg`, and scoped
lockfile extractors, plus `web__run` open/find. Truncated output was recovered
with smaller reads. An unavailable Python `tomllib` was replaced with scoped
record extraction. The required target, persona, standard, and context reads
completed; no missing required read was counted as a clean result.

**Supplementary limit:** Exact GTK 0.18.2 and Windows 0.62.2 Rust binding source
archives were absent locally, and docs.rs/archive fetch attempts were
unavailable. The codebase reviewer obtained official Windows Rust 0.62.2 API
documentation for the specified functions/types. The available GTK Rust stable
mirror identifies itself as 0.18.1, so it was not claimed as an exact 0.18.2
source read. Lockfile records, installed Tauri accessor types, compatible crate
requirements, and official native API contracts supplied the applicable
evidence. No binding incompatibility was demonstrated. Native compilation is
still required during implementation.

The primary agent verified the semantic scope and target fingerprint before
and after the review. Reviewers did not modify files, run builds/tests, spawn
additional agents, or retrieve conversation history.

## Meaning of this pass

The design is ready to implement without another identified design repair.
This review does not establish successful Windows/Linux focus behavior on a
running desktop. Native compilation and tests, focus/transparency behavior,
long-hidden updates, packaged updater delivery, and visual approval through
replay remain pending exactly as specified in the design. No implementation
or runtime result is claimed here.
