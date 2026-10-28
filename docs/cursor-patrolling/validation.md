# Cursor patrolling: native validation

**Implementation complete; live native acceptance pending.** Updated 2026-10-27. The user reported a manual check as “seems fine” and explicitly authorized full integration with final desktop testing deferred until the real application is ready. The macOS and Windows adapters, coordinated overlays, native worker, Awake/timer/exit integration and automated checks are implemented. No Windows desktop is available. The general manual report does not establish any particular timing, physical-input or multi-monitor case below.

This record implements the evidence boundary in the [approved design](../designs/cursor-patrolling.md#execution-boundary-and-phases). A textual review, successful compile, hidden geometry match or missing test environment is not a native behavioral pass. Every required case below is Passed, Failed or Not run; the scope of a Passed preparation check is stated explicitly.

## Environment and provenance

| Field | Actual observation |
| --- | --- |
| Host | Mac15,6; arm64; macOS 27.2, build26B123 |
| Read-only inventory | 2026-10-27T18:19:15Z, real desktop, outside the filesystem/process sandbox |
| Native display bounds | One built-in Retina display, session display ID 1; Core Graphics 1512×982 points at (0,0); no mirroring, rotation 0° |
| Measured overlay content | NSScreen, NSWindow and content rectangle each1512×982 at(0,0); Tauri inner physical size3024×1964; scale2.0 |
| Mouse / trackpad inventory | MX Master 3S; Apple internal keyboard/trackpad. Presence is not proof that either supplied a tested event. |
| Other keyboard devices | MagicKuboard; Karabiner DriverKit VirtualHIDKeyboard 1.8.0. Synthetic/virtual input must not be substituted for physical takeover evidence. |
| Permissions | CGPreflightListenEventAccess=false; AXIsProcessTrusted=false. These are the initial values. A later explicit listening-only request returned listenAccess=false; user confirmation/OS approval is pending. No Accessibility request was initiated. Accessibility trust is recorded, not presumed necessary for every chosen API. |
| Current session | CGSession query succeeded; on-console=true; login-done=true; secure input=false. This is only the current unlocked state, not a lock/secure-input transition test. |
| Foreground during inventory | com.brave.Browser 156.1.98.60; no window content or key stream captured |
| Timing workload | No acceptance workload declared or measured; power mode, load profiles and memory-pressure observations remain to be recorded before timing trials. |
| Windows | Unavailable, confirmed by the user. The actual Windows adapter and patrol coordinator/IPC/overlay module have since been cross-compiled and linted against real Tauri/Windows dependencies in isolated harnesses; the complete Windows app and desktop behavior remain unrun. |

The sandboxed first inventory returned zero displays and no session dictionary. Re-running the same read-only probe with actual desktop access found the display and active session above. The sandbox result is an environment-access limitation, not a monitor/session failure.

Temporary prototype workspace: [README and runnable commands](/private/tmp/awake-cursor-patrolling-phase0-q1_sf8vu/README.md). Its code and generated artifacts are outside the repository. [Source and binary hashes](/private/tmp/awake-cursor-patrolling-phase0-q1_sf8vu/source-manifest.json) bind the evidence to the actual fixtures. [Source/evidence archive](/private/tmp/awake-cursor-patrolling-phase0-q1_sf8vu/phase0-sources.zip) excludes the generated Cargo target tree. Temporary storage may be cleared; retain the archive outside the source tree for later runs.

Raw records: [desktop inventory](/private/tmp/awake-cursor-patrolling-phase0-q1_sf8vu/preflight-native.json), [permission refusal](/private/tmp/awake-cursor-patrolling-phase0-q1_sf8vu/permission-denied.json), [hidden Tauri geometry](/private/tmp/awake-cursor-patrolling-phase0-q1_sf8vu/overlay-hidden.json).

## Preparation and narrow native observations

| ID | Case | Status | Actual evidence and limit |
| --- | --- | --- | --- |
| PRE-1 | Compile read-only/native macOS API probes | Passed | Both Objective-C fixtures compile with Xcode clang, ARC, -Wall -Wextra -Werror and the public AppKit/ApplicationServices/Carbon/IOKit frameworks as applicable. This does not compile the future Rust adapter. |
| PRE-2 | Enumerate actual desktop and current session | Passed | One display pairs with NSScreen; active console/login and current secure-input state were queried. No transition was induced. |
| PRE-3 | Refuse observation when listening access is absent | Passed | The initial recorded native-probe --observe build exits 77 with permissionRequired. Reported listener creations 0, permission requests 0, cursor-position calls 0. This is the temporary runner's guard, not the future application's permission/recovery behavior. |
| PRE-4 | Compile temporary Tauri fixture with pinned macOS bindings | Passed | cargo build --offline --locked succeeds for Tauri 2.12.5, Tao 0.37.2, Wry 0.57.1, `objc2 = 0.6.5`, `block2 = 0.6.2` and the selected objc2 framework crates = 0.3.2/features. Its isolated Cargo.lock is retained; the production manifests/lockfile are unchanged. Successful fixture dependency compilation is not compilation of a complete production adapter. |
| PRE-5 | Native content placement on the one connected screen | Passed | The hidden borderless transparent Tauri window's frame and content exactly match NSScreen. The process exits successfully with no visible overlays, permission requests or cursor calls. This establishes only hidden placement on this configuration. |
| PRE-6 | Fixture source checks | Passed | cargo fmt --all -- --check passes; the embedded capture fixture script passes Node syntax checking. These are preparation checks, not interaction evidence. |

The native movement runner uses CGWarpMouseCursorPosition and a passive CGSession event tap on its own run loop. Session/secure-input queries stay on the main thread. It has 100 ms health authorization, interruption/acknowledgment, a bounded 20-point two-position experiment and no cursor restoration. It first requires an observed Esc, then a separate explicit Return and three-second countdown. No movement trial was started. Input source identity, observer validity/loss, callbacks, lease stopping, and cleanup remain unverified runtime paths. The small API experiment is not the complete adapter: it does not implement the 250 ms display-fingerprint backstop, full cross-screen route policy, final setup protocol or Awake integration.

The Tauri fixture separately provides optional input-taking overlays with real WebKit pointer capture and an immutable released sample delayed 200 ms before submission. Its hidden mode was executed. The user later ran the visible capture fixture; a completed result contains 12 captured release records on the one screen, while an earlier completed run contains none. It contains no cursor-positioning code. It is an observation fixture, not the production multi-window route editor or Start barrier.

## Listening-access follow-up

After the user confirmed they can perform the macOS steps now, the runner's explicit `--request-listening-access` action was executed. [Request result](/private/tmp/awake-cursor-patrolling-phase0-q1_sf8vu/access-request.json): listenAccess=false at return, Accessibility trust=false, cursor-position calls0. The request did not create a listener or start a movement trial. The OS may require user approval or restarting its identified host; no approval is inferred from the request. Physical tests remain Not run until real access and events are observed.

The initial negative observation predates the later phase-counter/reporting additions. Its exact [source](/private/tmp/awake-cursor-patrolling-phase0-q1_sf8vu/evidence-history/native-probe-initial.m) and [manifest](/private/tmp/awake-cursor-patrolling-phase0-q1_sf8vu/evidence-history/initial-source-manifest.json) are retained so that the earlier result is not attributed to a rebuilt executable.

## Required macOS Phase 0 cases

| ID | Case | Status | Observation / prerequisite |
| --- | --- | --- | --- |
| MAC-1 | Cursor positioning does not generate the manual-input stop | Not run | The initial probe had no listening access; no instrumented positioning result has been recorded. Require physical-input-qualified, otherwise quiet trials. |
| MAC-2 | Global Esc during travel, another app focused | Not run | Require listening approval and real Esc delivery; permission bits or a tap handle alone are insufficient. |
| MAC-3 | Global Esc during a stationary pause | Not run | Same prerequisite; qualify separately from travel. |
| MAC-4 | Mouse takeover during travel and pauses | Not run | MX Master 3S present; physical trials not performed. |
| MAC-5 | Trackpad takeover during travel and pauses | Not run | Internal trackpad present; physical trials not performed. |
| MAC-6 | No deferred movement after completed stop | Not run | Require actual trial, native acknowledgment record and observation of the pointer after stopping. |
| MAC-7 | Visible all-monitor overlays, first click, focus and no click-through | Not run | User-operated one-screen fixture returned 12 release records. No case-specific coverage/focus/click-through observation or additional display was recorded; validate the production app. |
| MAC-8 | Overlay coverage over a fullscreen application | Not run | Requires visible fixture and actual fullscreen workspace. |
| MAC-9 | Captured cross-monitor release; later movement toward Start cannot replace it | Not run | Only one monitor connected. Use fixture's200ms delayed submission with an additional display. |
| MAC-10 | Mixed scale, negative origins, screen gaps and mirrored displays | Not run | Additional physical display configurations unavailable in this run. |
| MAC-11 | Display disconnect/reconfiguration stops travel and pauses | Not run | Requires permission-enabled trial and physical display changes. Callback registration/source code is not evidence. |
| MAC-12 | Lock, session deactivation and system/screens sleep stop movement | Not run | Current unlocked-state query only. Do not infer transition behavior from public keys. |
| MAC-13 | Secure-input activation / loss of usable keyboard stream stops movement | Not run | Current secure-input=false only. Must exercise the selected public detector with an actual protected-input scenario. |
| MAC-14 | Denied, revoked and recovered listening/access permissions | Not run | Runner's initial unavailable guard checked in PRE-3; live loss, recovery and explicit restart have not been exercised. |
| MAC-15 | Disabled/stopped observer, expired availability/observer lease and main-thread stall | Not run | Requires actual listener and runtime fault trials; do not count code inspection or the timing constants as measurements. |
| MAC-16 | Resource teardown; no callbacks or movement after acknowledged completion | Not run | Hidden fixture process exited; permission-enabled input/worker cleanup and failures remain unexercised. |
| MAC-17 | 50 ms takeover and 150 ms health-loss completed-stop targets | Not run | No timing trials. Before the 20 travel + 20 pause acceptance set, declare workload, power mode, applications/versions, CPU/memory observations and measurement method. Health-loss authorization expires inclusively at 100 ms; retain separate entry/cancellation/ack times. |

## Required Windows Phase 0 cases

| ID | Case | Status | Observation / prerequisite |
| --- | --- | --- | --- |
| WIN-1 | Actual Windows adapter/dependency compilation | Passed (compile only) | Installed Rust x86_64-pc-windows-gnu std; Cargo check and Clippy -D warnings compile the actual model/engine/platform source with windows=0.62.2 and the declared feature set. The temporary harness stubs only the Tauri WebviewWindow type/typed hwnd accessor; it executes no Windows API. A second harness also compiles/lints the complete actual patrol module (coordinator, IPC, overlays, backend) against real Tauri 2.12.5, stubbing only Awake/tray effects. The complete Windows application still needs its CI/desktop build. |
| WIN-2 | Tao DeviceEventFilter::Always handoff and owned Raw Input sink | Failed (user report; fix awaiting retest) | On 2026-10-28 the user could not start patrol on their work Windows machine: "Another component owns mouse or keyboard Raw Input". A confirmed enumeration/error-reporting bug was corrected locally; successful runtime handoff and startup still require a rebuilt-app retest. |
| WIN-3 | SetCursorPos input separation; global Esc and physical mouse/trackpad takeover during travel/pauses | Not run | No Windows desktop. Include precision-touchpad zero-device-handle cases. |
| WIN-4 | Stop acknowledgment, in-flight emission, no deferred movement and resource ownership cleanup | Not run | No Windows desktop. |
| WIN-5 | All-monitor overlays and captured release on mixed scales/negative origins/gaps/mirrors | Not run | No Windows desktop or monitor configuration. |
| WIN-6 | Display/session/power/lock/input-desktop transitions and observer/availability loss | Not run | No Windows desktop; do not infer from declarations or a macOS result. |
| WIN-7 | Existing confinement before Start and introduced in travel/pauses | Not run | Inspect without modifying another application's ClipCursor rectangle. |
| WIN-8 | Success-but-clamped positioning, readback mismatch, truthful failure and explicit recovery | Not run | Must demonstrate no corrective move, restoration, permission assumption or automatic restart. |
| WIN-9 | Existing window input/focus behavior after the Tao registration handoff | Not run | Includes ordinary Awake/timer regression behavior. |
| WIN-10 | 50 ms takeover / 150 ms health-loss acceptance under a declared workload | Not run | Retain20travel+20pause trials and independent health-expiry/ack timestamps per design. |

## Decision and next actions

No required native behavior has been marked Failed, because those trials were not run. This is not a Phase 0 pass. There is no evidence yet requiring a different adapter/capture design; a failed required native trial must return that decision for design revision rather than silently changing APIs or weakening stopping.

1. Test the built application through its tray menu. Use explicit permission recovery if macOS requests it; verify Esc and physical takeover during both travel and pause with another app focused.
2. Connect additional displays and test visible coverage, captured cross-monitor release, mixed scales, gaps, mirrors and layout changes. The one-screen fixture is not a substitute.
3. Run the Windows application on an actual desktop before marking Raw Input, confinement, focus or stop timings Passed.
4. Keep release/native acceptance pending the recorded desktop results. Full integration proceeded under the user's explicit sequencing override below.

The approved design and all earlier review/resolution artifacts are unchanged. No production checks were run because application sources/configuration were not modified. Probe build/syntax checks and actual desktop observations above are the validation performed in this phase.

## Implementation sequence authorized by the user

The user reported a manual check as “seems fine” and explicitly requested the entire implementation with logic/tests prioritized, deferring final desktop testing until the actual app is ready. This supersedes the earlier requirement to finish Phase 0 before integration; it does not turn the unrun cases above into Passed. Native production integration may now proceed under that instruction. Release/native acceptance still requires the eventual desktop results. The scope/device/focused app of the general manual report was not specified, so no individual native case was inferred from it.

A visible temporary launcher was built and opened after the command-line permission request was not visible to the user. It provides explicit listening-access, observation, trial preparation/start and capture-fixture controls. Computer Use was not approved for the launcher; no automated UI control or permission grant was performed. The user can operate it directly. Further implementation work does not require operating that probe.


## Production implementation verification

These are automated/source/build outcomes, not physical desktop acceptance. Checks ran on the Mac described above with Rust 1.97.1, Node v25.7.0 and the existing pinned package graph. The temporary Windows adapter harness is `/private/tmp/awake-patrol-windows-check`; its Cargo features match the application's Windows section. `/private/tmp/awake-patrol-windows-integration` additionally checks the full patrol module against real Tauri, with only Awake/tray effects stubbed. A Clang module cache under `/private/tmp/awake-patrol-clang-cache` avoids this agent's read-only home-cache sandbox; no build-script/configuration change was required.

| Check | Outcome | Scope |
| --- | --- | --- |
| `pnpm check` | Passed | Svelte/TypeScript and existing startup test type checks; zero errors/warnings. |
| `pnpm test:patrol` | Passed, 14 tests | Strict wire schema/u64 values; stale versions; frozen confirmation and identical lost-reply replay; final release; slow preview coalescing; concurrent target edit; uncertain move result; drain/ACK ordering; layout/setup teardown; failure ownership/removal. |
| `pnpm test:startup` | Passed, 32 tests | Existing startup/update/replay regressions. |
| `pnpm build` | Passed | Static frontend, including `/patrol`. |
| `cargo test --locked --lib` | Passed, 45 tests | 35 patrol tests plus existing startup policy and three real macOS sleep-inhibitor tests. New patrol tests use fake time/input/position; no pointer calls or native windows. |
| `cargo fmt --check` | Passed | Production Rust formatting. |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed | Complete macOS application, including test targets. Existing dependency `block 0.1.6` emits Cargo's future-incompatibility notice. |
| `cargo check --locked --release --lib` | Passed | Release handler list and native integration compile. |
| Windows adapter and patrol integration cross-check/Clippy | Passed | Actual Windows backend and complete patrol module/IPC/overlays compile against real Tauri and exact Windows features. Only the harness Awake/tray port is stubbed. No desktop execution or complete Windows app build claimed. |
| Linux unsupported adapter cross-check | Passed | Actual model/engine/platform source for x86_64-unknown-linux-gnu in the isolated harness; no listener. Full Linux GUI build remains the existing CI matrix's job. |
| Local macOS debug app bundle | Passed | `pnpm tauri build --debug --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'`. The per-invocation override avoids requesting a release signing key; repository bundle configuration is unchanged. |

App for final user testing: [`awake.app`](/Users/jqubi/Desktop/code/awake/src-tauri/target/debug/bundle/macos/awake.app). Quit an older Awake instance before opening this build. Right-click its tray icon, choose Cursor Patrolling, add points, choose a pause and Start. No production movement or OS permission prompt was initiated by the automated tests/build.

The final integration audit also checks two scheduling hazards: Tauri can execute `run_on_main_thread` inline, so callbacks enter through an off-main dispatcher and tray refresh runs after releasing the coordinator mutex. State notifications publish only when the setup/version changes; getters cannot create a refetch/notification loop. Windows WebView2 construction runs off the synchronous UI handler, then placement/mapping/visibility return to the main actor; cancellation/rebuild waits for its completion. The latter has a deterministic regression test.

## Native acceptance details to record during the final app test

- At least two screens, ordered loop/return to first, 500 ms smooth travel and the full selected pause after arrival. Include negative origins, different scaling, gaps and mirroring where available.
- Esc, mouse and trackpad takeover from another focused app during travel and pause; no later movement and Awake stays on. Measure native observation-to-ack separately from UI refresh before claiming the 50 ms/150 ms targets.
- Awake Off and timer expiry, including a timer set/canceled/replaced while moving; stopped movement even if inhibitor release fails. Starting while Off enables Awake; an existing timer survives Start.
- Disconnect/reconfigure while moving and paused; review is required, invalid points are retained for removal/re-placement, and no automatic restart occurs. Generic Windows display broadcasts conservatively retire uncertain display tokens; unchanged physical points may require re-placement after that broadcast.
- Permission deny/revoke/recover, protected keyboard input, lock/sleep/session changes, observer loss, slow/hung native calls and explicit recovery. Windows confinement before/during patrol, successful-but-clamped readback, zero-device-handle/absolute Raw Input, and normal timer/main-window input after the Tao handoff.
- Cancel/reopen, repeated Start/Stop, overlay reload/close, cleanup errors and quit/relaunch. Process-plugin restart uses the normal two-second cleanup grace; a reserved installer-owned Tauri restart cannot be deferred and immediately invalidates movement before process termination.

The approved design and all review/resolution reports remain unchanged. These implementation notes record conservative native handling and framework integration constraints; live failures must be fixed before release, without weakening stop or review semantics.

Final local bundle executable SHA-256: `475aec330b328ee3429b12f4cc307345ea3c5f601a9a3e5c6420a153aa61f4aa`; bundle ID `com.awake.app`, version `0.3.5`. The executable was built after the final production source edit.

## User application smoke test — 2026-10-28

The user tested the built macOS application and confirmed that the cursor moves between the configured points, then reported that the functionality seems to be working and authorized committing/pushing to dev. This records a successful basic live movement smoke test. It does not establish the unmeasured timing, permission-loss, multi-monitor or Windows cases above. Teams presence behavior has not been verified.

## Windows startup investigation — 2026-10-28

The user reported that patrol could not start on a work Windows machine. The setup showed “Enable input access / Retry” and “Another component owns mouse or keyboard Raw Input.” OS build, device layout and actual registration owner were not captured; the agent still has no Windows desktop access. This establishes a failed startup smoke test, not a measured input or company-policy diagnosis.

Source inspection found a definite bug in the Windows registration query. `GetRegisteredRawInputDevices(NULL, &count, ...)` uses `ERROR_INSUFFICIENT_BUFFER` to report the required buffer size. The adapter treated that documented size result as a failure and used the same ownership error for any enumeration failure. It could also submit an empty buffer for zero registrations. Microsoft documents both [enumeration return values](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getregisteredrawinputdevices) and [registration scope within the calling process](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerrawinputdevices). This message does not establish a denied OS mouse permission or interference from a separate application.

The correction accepts the documented size-probe response, returns an empty snapshot directly for zero registrations, retries bounded size races, validates copied counts and retains actual Windows error codes for other failures. Startup, health and cleanup use the same query. Actual conflicting registrations still block startup; ownership is never silently stolen. The UI says “Enable input access” only for permissionRequired and “Retry input monitoring” for other native availability failures.

Verification: 50 Rust library tests pass (including five new registration-query regression tests), 14 patrol frontend tests pass, Svelte/TypeScript checks pass, and Clippy passes for the Mac application and the actual full patrol module against real Windows/Tauri dependencies. These checks do not run the Windows API on a desktop. A rebuilt Windows application must be retested before changing WIN-2 to Passed.
