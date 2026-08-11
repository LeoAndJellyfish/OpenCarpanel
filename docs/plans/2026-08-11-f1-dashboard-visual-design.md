# F1 24 Dashboard Visual and Motion Design

**Status:** accepted implementation direction for the first usable version  
**Date:** 2026-08-11  
**Scope:** phone/iPad driving view, connection and stale states, three core widgets

## Intent

The dashboard is a driving instrument, not a decorative second screen. A driver must read gear, rev state and speed in peripheral vision while the device is mounted at different distances. The interface therefore prioritizes silhouette, contrast, stable geometry and immediate state changes. It must feel purpose-built for motorsport without copying Formula 1 game branding, team liveries, trademarks or broadcast graphics.

The chosen direction is **Trackside Signal System**: matte graphite surfaces, warm-white numerals, a narrow fluorescent yellow-green rev band and restrained red for terminal warnings. The memorable element is a single horizontal shift-light horizon that compresses toward the centre as RPM rises, visually framing the gear rather than competing with it.

Two alternatives were considered:

- **Broadcast telemetry wall:** very information-dense and excellent for spectators, but slower to scan while driving and poorly suited to a phone in portrait orientation.
- **Neo-analog cockpit:** expressive arcs, glow and physical dial metaphors, but it spends more GPU budget on decoration and makes user-defined layouts visually inconsistent.

Trackside Signal System best matches the accepted requirements: simple, intuitive, high-frame-rate and themeable.

## Composition

The default landscape layout has one strong visual axis. The shift-light horizon occupies the upper edge. Gear is the largest central shape, speed sits lower-left, and connection/DRS state sits lower-right. Empty space around the gear is intentional; it makes the value legible through vibration and at arm's length. Portrait mode stacks the same hierarchy instead of shrinking the landscape layout.

The three MVP widgets are independent render targets:

1. **Gear** — immediate DOM text update; neutral and reverse use explicit letterforms. No bounce, roll or delayed gear-change animation.
2. **Tachometer** — static SVG geometry with segment opacity and transform updates. The redline zone is always spatially stable.
3. **Speed** — tabular numeric text, interpolated only between valid samples, with a smaller unit label owned by the widget.

The visual system uses CSS custom properties for surface, ink, muted ink, rev-ready, redline and warning colors. Fonts use locally available condensed instrument faces (`Bahnschrift`, `Avenir Next Condensed`, `DIN Alternate`) to avoid a network dependency. Fallbacks preserve layout metrics.

## Motion and data flow

WebSocket handlers only validate messages and write the newest target state. They never trigger a Preact render for each packet. A single `requestAnimationFrame` scheduler samples a monotonic clock, computes display values and invokes only bindings subscribed to changed fields.

Continuous values such as speed and RPM interpolate for at most one expected telemetry interval. After that interval they clamp to the newest value; the UI never extrapolates beyond received data. Discrete values—gear, DRS, brake state, flags and stale state—change immediately. A session-id change clears interpolation history.

Motion is limited to compositor-friendly `transform` and `opacity` on pre-existing layers. There is no per-frame DOM creation, layout measurement, filter, blur or animated shadow. The connection-to-dashboard reveal is a short coordinated sequence; recurring telemetry movement remains physically direct. With `prefers-reduced-motion: reduce`, entrance motion is removed and continuous value interpolation can be disabled without hiding state changes.

The page pauses its rAF loop when hidden. On visibility restoration it discards old interpolation history and waits for the latest snapshot before resuming. A stale source dims live accents and displays a textual `DATA STALE` rail without zeroing or inventing values.

## Failure states and accessibility

Pairing-required, reconnecting, incompatible-protocol and stale-data states each have distinct text. Color is supplementary: status always includes a word or symbol. Connection errors remain visible after the WebSocket close event instead of being overwritten by a generic disconnected state.

Core text targets WCAG AA contrast on the graphite background. Large values use tabular numerals, dynamic labels have stable accessible names, and decorative SVG segments are hidden from assistive technology. Touch targets in later edit mode are at least 44 CSS pixels; drive mode itself has no accidental touch gestures or scrolling.

## Performance contract

- One rAF scheduler per page, with no per-widget animation loops.
- No full Preact tree re-render at telemetry frequency.
- Snapshot storage is latest-only; intermediate frames are replaceable.
- 60 FPS baseline: frame p95 below 16.7 ms and JavaScript p95 below 3 ms.
- 120 FPS is best-effort on high-refresh devices and must not change data correctness.
- Browser production JavaScript stays budgeted and measured after each widget addition.

## Verification

Pure tests use a fake monotonic clock for interpolation limits, session resets and immediate discrete fields. Subscription tests prove that an RPM-only update does not notify gear or unrelated widgets. Widget tests verify formatting and missing-data states without relying on animation timing. A browser smoke test covers pairing, snapshot receipt, background/foreground recovery and reconnect. Real-device traces are recorded separately for a baseline phone and iPad before decorative motion is expanded.
