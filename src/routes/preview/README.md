# Startup replay preview

`/preview` mounts the shared startup visual and a simulated launch controller.
Run `pnpm preview:intro`; it opens the route at the fixed Vite port 1420.

Replay starts a new simulated launch. Pause/Resume freeze and resume virtual
time. The timeline scrubs only the eye presentation, including a completed
opening; update state is not rewound or advanced by seeking. Stop cancels
preview work. Tuning is validated in memory and can be copied as JSON.

Scenarios cover ordinary/slow checks, early/late discovery, long download,
installation, and three failure phases. Download complete and Install complete
advance deferred simulated operations. Try again starts a fresh simulated
check/download; Restart only retries relaunch. Installation failure offers Hide.
Windows handoff ends the old simulated view; Simulate restart starts a new
ordinary launch. These controls perform no real update, process, sleep-state, or timer operations.

The labelled background is a browser-only desktop simulation. Current stage
uses 1 CSS pixel per stage pixel; fixed HD/4K/ultrawide viewports scale to fit and
show that scale. Expand stage uses browser fullscreen, exited with Escape.
In a debug desktop build, open **Intro Preview** from the tray, then enable
**Native desktop replay**. Native runs use separate surfaces, exact caller/token
checks, and a 30-second lease. Pause, seek, tuning, status labels, and Stop are
forwarded through the preview namespace. The passive surface reports time at
most ten times per second; it never forwards commands to production.

The route owns one animation frame while work is active, a ResizeObserver, and
a keyboard listener (R replay, Space pause). It disposes them on teardown.
No update plugins or activation/timer commands are imported.
