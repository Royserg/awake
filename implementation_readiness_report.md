# Cinematic eye startup: implementation readiness review

- **Target:** [Approved design](./docs/designs/cinematic-eye-startup.md), lines 540–875 (`Detailed Implementation Plan`).
- **Date:** 2026-10-26.
- **Mode:** EGM Stage 9, implementation readiness only.
- **Target SHA-256:** `ac13fcb1a28c7e4e19c01fbf02cf07a49f810f54eea8accae922dbfbe5c82413`.
- **Initial result:** Two Major findings and four Nits; no Blockers. All six were addressed in the design on 2026-10-27; see the [resolution report](./goldfish_review_report.md).
- **Changes during initial review:** None. Subsequent document repairs are recorded separately in the resolution report. The initial review did not include a re-audit.
- **Latest review:** The [2026-10-27 re-audit](./implementation_readiness_reaudit_report.md) confirmed closure of all six original findings and identified RA-01. The user subsequently requested its portability repair, recorded in the [resolution report](./goldfish_review_report.md). The statements below describe the initial review.
- **Portability confirmation:** The [latest readiness report](./implementation_readiness_portability_report.md) independently confirms the Windows/Linux repair with zero findings. Runtime acceptance remains pending.

Three fresh-context reviewers independently read the target and its referenced
existing files. They reviewed implementation readiness, codebase compatibility,
and clarity. The first two used the bundled
[implementation-readiness standard](/Users/jqubi/.codex/plugins/cache/egm/egm/0.1.0/resources/document_standards/implementation-readiness-standard.md);
the style reviewer used its persona. This was a document and source review;
no runtime tests, builds, or visual acceptance checks were performed.

## Major findings

### IR-01 — Define equal-revision request behavior

- **Reviewer lens:** Style/contract precision.
- **Affected scope:** Design lines 711–716, native command table.
- **Discrepancy:** `start_intro` and `show_compact` reject lower/older revisions,
  but the contract does not say what an equal revision means. Retrying a request
  could return its existing token, create another window/token, or fail.
- **Consequence:** Implementations can disagree about duplicate delivery,
  deadline renewal, and idempotency even while following the written rules.
- **Recommendation:** Specify that an identical request at the current revision
  returns the existing run/token without recreating its presentation or renewing
  its deadline. Reject equal-revision requests whose payload differs. Define
  the response when that revision has already completed or been cancelled so a
  retry cannot resurrect it. Apply the equivalent rule to preview revisions.
- **Verification:** Add duplicate-identical, duplicate-conflicting, and
  duplicate-after-completion/cancellation cases to native policy tests. Check
  that the original deadline and run/token remain unchanged.

### IR-02 — Specify a nonactivating compact-window show on macOS

- **Reviewer lens:** Codebase details.
- **Affected scope:** Design line 563 and lines 715–738.
- **Discrepancy:** Initial `focus: false` does not control subsequent calls to
  `show()` on the installed macOS window implementation. The plan requires a
  compact handoff without focus stealing but does not specify the mechanism.
- **Consequence:** An implementer following the current `window.show()` pattern
  can request key-window status for the compact update window. A late update
  found while Awake's timer is focused can move keyboard focus away from it.
- **Recommendation:** Specify the serialized native show sequence: temporarily
  make `main` nonfocusable, show it, and restore its previous focusability,
  including restoration after errors. Keep these operations ordered on the
  native main thread. Verify the behavior on the actual desktop before treating
  the mitigation as accepted.
- **Verification:** Add a native acceptance case with the timer focused when a
  late update appears, and another with a different application focused. The
  compact window must appear without taking keyboard focus and remain usable
  when deliberately selected afterward.

The locked dependency source supports this finding and the proposed mitigation:

- [Runtime Show handler](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-runtime-wry-2.12.3/src/lib.rs:3380)
  maps `Show` to `set_visible(true)`.
- [macOS visibility implementation](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/macos/window.rs:643)
  calls `make_key_and_order_front_sync`; the
  [native helper](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/macos/util/async.rs:213)
  invokes `makeKeyAndOrderFront`.
- [Initial construction](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/macos/window.rs:605)
  separately consults the initial focused flag.
- [Key-window eligibility](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/macos/window.rs:408)
  reads the focusable flag; the
  [focusability setter](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tao-0.37.2/src/platform_impl/macos/window.rs:663)
  changes that flag. Tauri exposes
  [set_focusable](/Users/jqubi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.12.5/src/webview/webview_window.rs:2391).

The primary agent independently re-read these installed source excerpts. This
confirms the source-level discrepancy; it does not replace a desktop focus test.

## Nits

### IR-03 — Mark stable implementation checkpoints

- **Reviewer lens:** Implementation readiness.
- **Affected scope:** Design lines 544–585.
- **Discrepancy:** The ordered file list does not identify the intermediate
  buildable/testable phase boundaries required by the readiness standard.
- **Recommendation:** Group the existing work into shared renderer, simulated
  browser replay, native/production integration, and acceptance/documentation.
  State the checks at each boundary. Keep the production entry point working
  until its integration phase; do not add architectural scope.

### IR-04 — State cell-size units at the tuning boundary

- **Reviewer lens:** Style.
- **Affected scope:** Design line 604.
- **Discrepancy:** `introCellPx` and the compact 15-pixel cell do not explicitly
  distinguish CSS pixels from physical monitor pixels.
- **Recommendation:** State that both use CSS pixels. Retain physical pixels
  for native monitor bounds and logical/CSS pixels for artwork.

### IR-05 — Name visual completion explicitly

- **Reviewer lens:** Style.
- **Affected scope:** Design lines 693–698.
- **Discrepancy:** “Hide/Stop/completion” follows update-completion rules, leaving
  the subject of “completion” unclear.
- **Recommendation:** Use “Hide, preview Stop, and visual completion dispose
  only presentation work.” Keep update-resource release rules separate.

### IR-06 — Distinguish preview retry actions

- **Reviewer lens:** Style.
- **Affected scope:** Design lines 786–791, read with 656 and 686–689.
- **Discrepancy:** The preview lists a generic Retry control while the
  controller distinguishes download retry from restart retry.
- **Recommendation:** Give the preview contextual actions matching production:
  Try again creates a fresh check/download attempt after download failure;
  Restart retries relaunch after restart failure; installation failure offers
  Hide without an automatic retry.

## Traceability

| Raw reviewer finding | Consolidated finding | Severity |
| --- | --- | --- |
| Style 1: equal-revision meaning | IR-01 | Major |
| Codebase 1: macOS compact show focus | IR-02 | Major |
| Implementation readiness 1: checkpoints | IR-03 | Nit |
| Style 2: cell-size units | IR-04 | Nit |
| Style 3: completion subject | IR-05 | Nit |
| Style 4: retry action naming | IR-06 | Nit |

The implementation-readiness reviewer reported no Major or Blocker findings.
The codebase reviewer reported one Major; the style reviewer reported one
Major and three Nits. The report retains all six findings without duplication.

## Compatibility checks and limits

The reviewers found the separate updater download/install operations,
existing updater permissions, Windows installer handoff, transparency support,
and documented background-throttling limitation consistent with the inspected
installed dependencies. They also checked the linked primary documentation:
[Tauri updater](https://tauri.app/reference/javascript/updater/),
[Tauri capabilities](https://v2.tauri.app/security/capabilities/),
[API changelog](https://github.com/tauri-apps/tauri/blob/dev/packages/api/CHANGELOG.md),
and [Node native TypeScript](https://nodejs.org/learn/typescript/run-natively).

Dynamic window creation was investigated but was not retained as a finding:
async command handling with deferred main-thread dispatch can satisfy the
existing serialization requirement. The document does not mandate the
synchronous callback pattern associated with the Windows deadlock warning.

Platform transparency, focus behavior, hidden downloads longer than five
minutes, packaged update delivery, and visual quality remain unverified runtime
acceptance work. These were already explicit in the design and are not new
review findings. Proposed files were not treated as missing implementation.

## Review completion evidence

All three reviewers were spawned with `fork_turns: "none"` and returned final
reports. Each reported reading its persona, the Codex integration guidance,
the whole target, and all 14 existing files in the supplied reading list.
The implementation/codebase reviewers also read their mapped standard. The
style persona had no separately mapped standard.

The shared reading list was
`/tmp/awake-eye-readiness.n4ni8ugn/reading-list.md`. It identified:

- `README.md`, `concepts_brief.md`, and `package.json`;
- `src-tauri/Cargo.toml`, `src-tauri/README.md`, `src-tauri/src/README.md`,
  `src-tauri/src/lib.rs`, and `src-tauri/tauri.conf.json`;
- `src/README.md`, `src/app.html`, `src/routes/+layout.ts`,
  `src/routes/+page.svelte`, `src/routes/README.md`, and
  `src/routes/eye-splashscreen.ts`;
- the four linked primary documentation pages above.

All file paths in that list resolve from the Awake repository root. Reviewers
reported successful `cat`, `nl`, and bounded `sed` reads, recovering initial
output truncation through smaller reads, and `web__run` open/find calls for the
primary documentation. No required context source remained unavailable.

Additional compatibility inspection covered `tsconfig.json`, `vite.config.js`,
the existing capability files, timer documentation, installed updater JS/Rust
sources, Tauri schema/window APIs, and the locked runtime/windowing sources.
The codebase reviewer also read `build.rs` and the release workflow. The
implementation reviewer supplied command chunk evidence for its reads:
`0c0ceb`, `148939`, `c52cd9`, `d60008`, `2fb9ce`, `5ae53b`, `cb7ecb`, `3607c4`,
`a55135`, `cd5bcf`, `cf7563`, `50f0e6`, `1477e0`, `984635`, `e05698`, `0a16c8`,
`35e48f`, `cb260e`, and `a7ec7d`.

An unavailable docs.rs page was replaced with the installed exact-version
Tauri source. An initial lookup for runtime version 2.12.0 was corrected to the
locked 2.12.3 source. Neither attempt left a required read incomplete.

Reviewer identities: `/root/eye_readiness_audit`, `/root/eye_codebase_audit`,
and `/root/eye_style_audit`. Their read-completion manifests and findings were
returned through the collaboration tool. The primary agent verified the target
boundary, hash, and critical macOS source evidence before consolidating this
report. These findings describe the original reviewed version. Subsequent fixes
are recorded in the resolution report. The later re-audit is linked above.
