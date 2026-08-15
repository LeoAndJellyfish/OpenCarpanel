# F1 24 MVP Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a local Rust Host that receives F1 24 UDP telemetry and renders a configurable, low-latency dashboard on a phone or iPad over the LAN.

**Architecture:** A Rust modular monolith owns UDP ingestion, game adaptation, game-neutral state, versioned configuration, pairing, and HTTP/WebSocket delivery. A TypeScript + Preact client renders only subscribed fields; continuous snapshots are lossy latest-state messages while discrete events are ordered and briefly replayable.

**Tech Stack:** Stable Rust（MSRV 1.91.1）, Tokio, Axum, Serde, Schemars, Tracing, TypeScript, Preact, Vite, Vitest, Playwright, CSS/WAAPI, npm workspaces.

---

## Working rules

- Read the accepted [architecture design](2026-08-11-opensimdash-architecture-design.md) and all ADRs before Task 1.
- Use the EA F1 24 document linked from [`docs/protocols/f1-24.md`](../protocols/f1-24.md) as the only authority for packet sizes and offsets. Record its revision before transcribing layouts.
- Keep commits limited to one task. Do not combine an adapter change with UI work.
- Write tests before implementation for packet parsing, state transitions, protocol compatibility and migrations.
- Production Rust must not use `unsafe`, `unwrap`, `expect`, input-triggered `panic`, or unbounded channels.
- Benchmark release builds on real devices; debug-mode timings are diagnostic only.

## Task 1: Establish repeatable quality gates

**Files:**

- Create: `.github/workflows/ci.yml`
- Create: `web/dashboard/src/.gitkeep` only if the frontend scaffold is intentionally deferred
- Modify: `README.md`

**Step 1: Run the current baseline**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check:web
```

Expected: all commands exit 0. There are no application tests yet.

**Step 2: Add Windows/macOS CI**

Create `.github/workflows/ci.yml` with two jobs:

```yaml
name: ci

on:
  pull_request:
  push:
    branches: [main]

jobs:
  rust:
    strategy:
      matrix:
        os: [windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          components: rustfmt, clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace

  web:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: actions/setup-node@v4
        with:
          node-version: 24
          cache: npm
      - run: npm ci
      - run: npm run check:web
      - run: npm run test:web
      - run: npm run build:web
```

Do not enable `npm ci` until `package-lock.json` exists in Task 11. The first CI commit contains only the Rust job; land the web job together with Task 11.

**Step 3: Re-run local gates**

Expected: Rust gates pass and the workflow YAML is syntactically valid.

**Step 4: Commit**

```powershell
git add .github README.md
git commit -m "ci: establish workspace quality gates"
```

## Task 2: Define the canonical telemetry model

**Files:**

- Modify: `crates/telemetry-core/Cargo.toml`
- Modify: `crates/telemetry-core/src/lib.rs`
- Create: `crates/telemetry-core/src/model.rs`
- Create: `crates/telemetry-core/tests/model_contract.rs`

**Step 1: Add serialization/schema dependencies**

Add `serde` with derive and `schemars` to `telemetry-core`. Pin compatible versions in `Cargo.lock`.

**Step 2: Write the failing model contract test**

```rust
use opensimdash_telemetry_core::{Gear, TelemetrySnapshot};

#[test]
fn absent_game_data_stays_absent() {
    let snapshot = TelemetrySnapshot::default();

    assert_eq!(snapshot.vehicle.speed_mps, None);
    assert_eq!(snapshot.vehicle.gear, Gear::Unknown);
    assert!(snapshot.extensions.is_empty());
}

#[test]
fn normalized_inputs_reject_out_of_range_values() {
    assert!(opensimdash_telemetry_core::Normalized::new(0.75).is_ok());
    assert!(opensimdash_telemetry_core::Normalized::new(1.01).is_err());
}
```

Run:

```powershell
cargo test -p opensimdash-telemetry-core --test model_contract
```

Expected: FAIL because the public model does not exist.

**Step 3: Implement the minimum typed model**

Create explicit `Meta`, `VehicleState`, `LapState`, `SessionState`, `TyreState`, `TelemetrySnapshot`, `TelemetryEvent`, `Gear`, and `Normalized` types. Requirements:

- Optional game data uses `Option<T>`.
- `Normalized` has a checked constructor for inclusive `0.0..=1.0`.
- `Gear` distinguishes reverse, neutral, forward number, and unknown.
- All public types derive `Debug`, `Clone`, `PartialEq`, Serde and JSON Schema traits where valid.
- `extensions` is keyed by adapter id and contains JSON values only at the model boundary.

**Step 4: Run focused and workspace tests**

```powershell
cargo test -p opensimdash-telemetry-core --test model_contract
cargo clippy -p opensimdash-telemetry-core --all-targets -- -D warnings
```

Expected: PASS.

**Step 5: Commit**

```powershell
git add Cargo.lock crates/telemetry-core
git commit -m "feat(core): define canonical telemetry model"
```

## Task 3: Define the adapter contract

**Files:**

- Modify: `crates/adapter-api/Cargo.toml`
- Modify: `crates/adapter-api/src/lib.rs`
- Create: `crates/adapter-api/src/adapter.rs`
- Create: `crates/adapter-api/tests/adapter_contract.rs`

**Step 1: Add the path dependency**

`adapter-api` depends on `telemetry-core`; the reverse dependency is forbidden.

**Step 2: Write a failing fake-adapter test**

The test constructs a fake adapter, decodes one byte slice into a reusable `AdapterOutput`, and verifies:

```rust
assert_eq!(adapter.descriptor().id.as_str(), "test-adapter");
assert_eq!(output.updates.len(), 1);
assert!(output.events.is_empty());
```

Also assert that calling `output.clear()` retains allocated capacity.

**Step 3: Implement the contract**

```rust
pub trait GameAdapter: Send {
    fn descriptor(&self) -> &AdapterDescriptor;

    fn decode(
        &mut self,
        datagram: &[u8],
        received_at: MonotonicInstant,
        output: &mut AdapterOutput,
    ) -> Result<(), AdapterError>;
}
```

Use typed `AdapterId`, `AdapterDescriptor`, `CapabilitySet`, `TelemetryUpdate`, `AdapterOutput`, and non-exhaustive `AdapterError`. Do not expose an HTTP or async runtime type in this crate.

**Step 4: Verify dependency direction**

```powershell
cargo tree -p opensimdash-adapter-api
cargo test -p opensimdash-adapter-api
```

Expected: only adapter-api points to telemetry-core; tests pass.

**Step 5: Commit**

```powershell
git add Cargo.lock crates/adapter-api
git commit -m "feat(adapter): define game adapter contract"
```

## Task 4: Define the versioned wire protocol and schemas

**Files:**

- Modify: `crates/protocol/Cargo.toml`
- Modify: `crates/protocol/src/lib.rs`
- Create: `crates/protocol/src/message.rs`
- Create: `crates/protocol/src/schema.rs`
- Create: `crates/protocol/tests/wire_contract.rs`
- Create: `schemas/protocol/.gitkeep` or generated schema files

**Step 1: Write failing serialization tests**

Verify stable external tags and protocol version:

```rust
let encoded = serde_json::to_value(ServerMessage::Snapshot(snapshot_message))?;
assert_eq!(encoded["v"], 1);
assert_eq!(encoded["type"], "snapshot");
assert!(encoded.get("data").is_some());
```

Add tests for `hello`, `capabilities`, `event`, `event_ack`, `resync_required`, `stale`, and structured error messages. Unknown message types must return a protocol error, never panic.

**Step 2: Implement protocol envelopes**

Use an externally tagged representation equivalent to:

```rust
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerPayload {
    Snapshot(SnapshotMessage),
    Event(EventMessage),
    Capabilities(CapabilitiesMessage),
    ResyncRequired(ResyncRequiredMessage),
    Error(ErrorMessage),
}

pub struct ServerMessage {
    pub v: u16,
    #[serde(flatten)]
    pub payload: ServerPayload,
}
```

**Step 3: Generate and check JSON Schema**

Add a small schema export binary or integration test that writes deterministic files under `schemas/protocol/v1/` and `schemas/telemetry/v1/`. The verification command must fail if committed output differs from newly generated output.

**Step 4: Run contract tests**

```powershell
cargo test -p opensimdash-protocol
cargo run -p opensimdash-protocol --bin export-schema
git diff --exit-code -- schemas
```

Expected: PASS and no schema drift.

**Step 5: Commit**

```powershell
git add Cargo.lock crates/protocol schemas
git commit -m "feat(protocol): define versioned wire contracts"
```

## Task 5: Implement the F1 24 packet header decoder

**Files:**

- Modify: `crates/adapter-f1-24/Cargo.toml`
- Modify: `crates/adapter-f1-24/src/lib.rs`
- Create: `crates/adapter-f1-24/src/cursor.rs`
- Create: `crates/adapter-f1-24/src/header.rs`
- Create: `crates/adapter-f1-24/src/error.rs`
- Create: `crates/adapter-f1-24/tests/header_decoder.rs`
- Create: `tests/fixtures/f1-24/header-v2024.metadata.json`

**Step 1: Record the authoritative spec revision**

Update [`docs/protocols/f1-24.md`](../protocols/f1-24.md) with the exact attachment revision and retrieval date used. Confirm the header byte count and every field offset before coding.

**Step 2: Write failing header tests**

Build bytes field-by-field in the test rather than transmuting a Rust struct. Cover:

- A valid F1 24 header returns format, packet id, session UID, frame identifiers and player index.
- Every truncation length returns `UnexpectedEnd`.
- A non-F1-24 format returns `UnsupportedPacketFormat`.
- Trailing payload bytes remain available to the packet-specific decoder.

Run:

```powershell
cargo test -p opensimdash-adapter-f1-24 --test header_decoder
```

Expected: FAIL because `PacketHeader::decode` does not exist.

**Step 3: Implement a checked little-endian cursor**

The cursor exposes checked `read_u8`, `read_u16_le`, `read_u32_le`, `read_u64_le`, and `read_f32_le`. Every method checks remaining length before advancing. Do not use pointer casts, packed structs or `unsafe`.

**Step 4: Implement and verify the header**

Use constants derived from the recorded official spec. Return typed errors containing expected/actual values but never include the entire datagram in normal logs.

```powershell
cargo test -p opensimdash-adapter-f1-24 --test header_decoder
cargo clippy -p opensimdash-adapter-f1-24 --all-targets -- -D warnings
```

Expected: PASS.

**Step 5: Commit**

```powershell
git add Cargo.lock crates/adapter-f1-24 docs/protocols tests/fixtures/f1-24
git commit -m "feat(f1-24): decode and validate packet headers"
```

## Task 6: Decode player telemetry and reduce it into state

**Files:**

- Create: `crates/adapter-f1-24/src/packets/mod.rs`
- Create: `crates/adapter-f1-24/src/packets/car_telemetry.rs`
- Create: `crates/adapter-f1-24/src/mapping.rs`
- Create: `crates/adapter-f1-24/tests/car_telemetry.rs`
- Create: `crates/telemetry-core/src/reducer.rs`
- Create: `crates/telemetry-core/tests/reducer.rs`
- Add: minimal synthetic fixtures and metadata under `tests/fixtures/f1-24/`

**Step 1: Write a failing car-telemetry test**

Construct a complete packet using offsets verified against the official document. Set a non-zero player index and distinct values for speed, throttle, brake, gear, RPM and DRS. Assert the adapter selects the player entry, converts speed to m/s, validates normalized inputs and emits only the expected update fields.

**Step 2: Test malformed and edge inputs**

Add table tests for truncated car arrays, invalid player index, NaN float input, out-of-range throttle/brake and unknown gear enum. Specify whether each case drops a field or rejects the packet; do not silently clamp corrupt network input.

**Step 3: Implement only the required F1 24 packet**

Decode the packet header, skip or read each officially defined field in order, and map only verified player fields. Do not implement other packet ids in the same commit.

**Step 4: Write a failing reducer test**

```rust
let mut reducer = TelemetryReducer::default();
reducer.apply(update_with_speed(50.0));
reducer.apply(update_with_rpm(11_000));
let snapshot = reducer.snapshot();
assert_eq!(snapshot.vehicle.speed_mps, Some(50.0));
assert_eq!(snapshot.vehicle.rpm, Some(11_000));
```

Add session-change and out-of-order frame tests. A new session clears stale game state; an older frame cannot overwrite a newer field.

**Step 5: Implement reducer and run tests**

```powershell
cargo test -p opensimdash-adapter-f1-24
cargo test -p opensimdash-telemetry-core
```

Expected: PASS.

**Step 6: Commit**

```powershell
git add crates/adapter-f1-24 crates/telemetry-core tests/fixtures/f1-24
git commit -m "feat(f1-24): map player telemetry into canonical state"
```

Repeat Task 6 as separate commits for session, lap, status, tyres/damage, participants and event packet groups. Each packet group requires its own normal/truncated/version tests.

## Task 7: Add deterministic UDP recording and replay

**Files:**

- Modify: `Cargo.toml`
- Create: `tools/telemetry-replay/Cargo.toml`
- Create: `tools/telemetry-replay/src/main.rs`
- Create: `tools/telemetry-replay/src/format.rs`
- Create: `tools/telemetry-replay/tests/replay.rs`

**Step 1: Define a safe capture format**

Use a local file header containing format version, adapter id and creation metadata, followed by records of monotonic delta microseconds and datagram bytes. Enforce maximum datagram and file sizes. Do not serialize Rust memory layouts.

**Step 2: Write a failing timing test**

Given three in-memory records at `0`, `16_667` and `33_334` microseconds, a fake clock/sink must receive them in order at the expected virtual times. Add `--speed 0` for immediate deterministic test playback.

**Step 3: Implement `record` and `replay` subcommands**

Recording requires an explicit output path under the ignored `captures/` directory by default and displays a privacy warning. Replay targets a configurable loopback UDP address and supports speed multipliers.

**Step 4: Verify**

```powershell
cargo test -p opensimdash-telemetry-replay
cargo run -p opensimdash-telemetry-replay -- --help
```

Expected: tests pass; help lists `record` and `replay` without opening a socket.

**Step 5: Commit**

```powershell
git add Cargo.toml Cargo.lock tools/telemetry-replay
git commit -m "feat(tools): add deterministic telemetry replay"
```

## Task 8: Implement versioned, atomic configuration storage

**Files:**

- Modify: `crates/config/Cargo.toml`
- Modify: `crates/config/src/lib.rs`
- Create: `crates/config/src/model.rs`
- Create: `crates/config/src/repository.rs`
- Create: `crates/config/src/migration.rs`
- Create: `crates/config/tests/recovery.rs`

**Step 1: Write failing persistence tests**

Using a temporary directory, verify:

- Saving then loading preserves `schemaVersion` and `revision`.
- A stale revision returns `Conflict { current_revision }`.
- An invalid JSON primary file is quarantined and last-known-good is loaded.
- A failed validation does not modify the existing file.
- A v0 fixture migrates deterministically to v1.

**Step 2: Define settings/layout/theme models**

Use typed component ids, instance ids, grid coordinates, breakpoint names and JSON-schema-validated settings. Limit document size, widget count, string length and nesting depth at the import boundary.

**Step 3: Implement atomic writes**

Write to a unique temporary file in the destination directory, flush it, sync when supported, then atomically replace the target. Keep a bounded backup set. Put OS-specific directory discovery behind a small function so tests always inject a temporary root.

**Step 4: Verify**

```powershell
cargo test -p opensimdash-config
cargo clippy -p opensimdash-config --all-targets -- -D warnings
```

Expected: PASS.

**Step 5: Commit**

```powershell
git add Cargo.lock crates/config schemas/layout
git commit -m "feat(config): persist versioned layouts atomically"
```

## Task 9: Build the minimal Host and health endpoint

**Files:**

- Modify: `apps/host/Cargo.toml`
- Replace: `apps/host/src/main.rs`
- Create: `apps/host/src/app.rs`
- Create: `apps/host/src/telemetry.rs`
- Create: `apps/host/src/http.rs`
- Create: `apps/host/src/shutdown.rs`
- Create: `apps/host/tests/health.rs`

**Step 1: Write a failing health test**

Start the app on `127.0.0.1:0`, request `/api/v1/health`, and assert:

```json
{"status":"ok","protocolVersion":1,"adapter":"f1-24"}
```

The test must shut down without leaving a task or occupied port.

**Step 2: Implement lifecycle and dependency wiring**

Add Tokio, Axum, tracing and typed application configuration. Inject listener sockets into tests. Use cancellation tokens or an equivalent explicit shutdown signal; no detached tasks.

**Step 3: Add UDP ingestion**

Bind the configured F1 port, reuse a bounded receive buffer, call the adapter and reducer, and update counters. Packet decode errors are rate-limited diagnostics; bind failure is a startup error with actionable context.

**Step 4: Verify health and shutdown**

```powershell
cargo test -p opensimdash-host --test health
cargo run -p opensimdash-host
```

Expected: health test passes; Ctrl+C exits cleanly without a backtrace.

**Step 5: Commit**

```powershell
git add Cargo.lock apps/host
git commit -m "feat(host): add supervised UDP and HTTP runtime"
```

## Task 10: Add pairing and dual-lane WebSocket delivery

**Files:**

- Create: `apps/host/src/pairing.rs`
- Create: `apps/host/src/websocket.rs`
- Create: `apps/host/src/events.rs`
- Create: `apps/host/tests/websocket_flow.rs`
- Modify: `apps/host/src/http.rs`

**Step 1: Write failing end-to-end tests**

Tests must prove:

1. An invalid/expired pairing token is rejected.
2. A valid one-time token creates a device session and cannot be reused.
3. Publishing snapshots faster than the client reads yields only the newest sequence.
4. Events reconnect from `lastEventSeq` in order.
5. Falling behind the ring buffer returns `resync_required`.
6. Oversized messages and unsupported protocol versions close with a typed reason.

**Step 2: Implement pairing tokens**

Generate at least 128 random bits, keep only a verifier/hashed representation where practical, expire quickly, and remove on successful use. Accept the token in the first protocol message, not in the WebSocket URL query.

**Step 3: Implement bounded channels**

Use a latest-value primitive for snapshots and a fixed-capacity ring for events. Never use an unbounded MPSC channel. Pre-serialize identical snapshot projections once per publish tick.

**Step 4: Verify**

```powershell
cargo test -p opensimdash-host --test websocket_flow
```

Expected: PASS, including the intentionally slow client case.

**Step 5: Commit**

```powershell
git add Cargo.lock apps/host
git commit -m "feat(host): stream paired telemetry over websocket"
```

## Task 11: Scaffold and contract-test the Web workspace

**Files:**

- Modify: `package.json`
- Create: `package-lock.json`
- Replace: `web/dashboard/package.json`
- Create: `web/dashboard/vite.config.ts`
- Create: `web/dashboard/tsconfig.json`
- Create: `web/dashboard/index.html`
- Create: `web/dashboard/src/main.tsx`
- Replace: `web/widget-sdk/package.json`
- Create: `web/widget-sdk/tsconfig.json`
- Create: `web/widget-sdk/src/index.ts`
- Create: `web/widget-sdk/src/generated/`

**Step 1: Install the smallest verified toolchain**

Add Preact, Vite, TypeScript and Vitest. Before committing, verify selected versions support the pinned Node version and each other. Do not add a UI framework or animation library.

**Step 2: Generate TypeScript protocol types**

Generate from committed JSON Schema. Add a CI check that regenerates to a temporary directory and fails on diff. Hand-written wire message duplicates are forbidden.

**Step 3: Write a failing protocol fixture test**

Parse Rust-produced hello/snapshot/event fixtures in TypeScript and assert discriminated unions, optional fields and unknown-version rejection.

**Step 4: Implement a minimal connection screen**

The app reads `#pair`, removes it from visible history after consuming it, opens the WebSocket, performs hello/pairing and renders connection state. No dashboard styling in this task.

**Step 5: Verify**

```powershell
npm ci
npm run check:web
npm run test:web
npm run build:web
```

Expected: PASS; production output is created under `web/dashboard/dist/` and remains ignored.

**Step 6: Enable the web CI job and commit**

```powershell
git add package.json package-lock.json web .github/workflows/ci.yml
git commit -m "feat(web): scaffold typed dashboard client"
```

## Task 12: Build the high-frame-rate dashboard renderer

**Files:**

- Create: `web/dashboard/src/telemetry/store.ts`
- Create: `web/dashboard/src/telemetry/render-loop.ts`
- Create: `web/dashboard/src/telemetry/interpolate.ts`
- Create: `web/dashboard/src/telemetry/*.test.ts`
- Create: `web/widget-sdk/src/manifest.ts`
- Create: `web/widget-sdk/src/registry.ts`
- Create: `web/dashboard/src/widgets/gear/`
- Create: `web/dashboard/src/widgets/tachometer/`
- Create: `web/dashboard/src/widgets/speed/`
- Create: `web/dashboard/src/styles/motion.css`

**Step 1: Test interpolation rules without a browser**

Use a fake monotonic clock. Assert continuous values interpolate for no more than one expected sample interval, clamping to the newest value afterward. Assert gear, flags, brake state and stale state update immediately. A new session clears interpolation history.

**Step 2: Test field subscriptions**

Mount three fake widgets with disjoint dependencies. Updating RPM must notify the tachometer subscriber but not gear or unrelated widgets. This is the guard against full-tree 60 Hz rendering.

**Step 3: Implement one rAF loop**

Network messages update targets only. One scheduler reads current targets, computes display values and invokes registered render bindings. Stop on `document.hidden`; on resume, request/consume a fresh snapshot before interpolating again.

**Step 4: Implement the three core widgets**

- Gear: DOM text, immediate update, no decorative gear-change motion.
- Speed: DOM text with unit conversion in the widget.
- Tachometer: SVG/static geometry with transform/opacity updates; no per-frame layout reads.

Include `prefers-reduced-motion`, explicit easing tokens and no `transition: all`.

**Step 5: Measure before adding more widgets**

Capture a 10-minute trace on one baseline phone and one iPad. Record device/browser details under `tests/performance/results/` in a small Markdown result, not a raw user profile.

Expected: 60 FPS frame time p95 below 16.7 ms, JS p95 below 3 ms, dropped frames below 1%. Fix failures before adding decorative effects.

**Step 6: Commit**

```powershell
git add web tests/performance
git commit -m "feat(web): add telemetry render loop and core widgets"
```

## Task 13: Add layout editing and safe persistence

**Files:**

- Create: `web/widget-sdk/src/layout.ts`
- Create: `web/dashboard/src/editor/`
- Create: `web/dashboard/src/routes/drive.tsx`
- Create: `web/dashboard/src/routes/edit.tsx`
- Create: `web/dashboard/src/api/layouts.ts`
- Create: `web/dashboard/src/editor/*.test.ts`
- Create: `apps/host/src/layout_api.rs`
- Create: `apps/host/tests/layout_api.rs`

**Step 1: Test the pure grid engine**

Given pointer deltas and a breakpoint grid, assert deterministic move, resize, bounds and collision results. Undo/redo must restore exact JSON documents. Keep geometry pure and independent from DOM APIs.

**Step 2: Test lazy loading**

The production driving entry chunk must not import the editor or its gesture code. Add a build-stats assertion or bundle inspection script.

**Step 3: Implement edit mode**

Support add/remove, drag, resize, snap, duplicate, settings, breakpoint switch, undo/redo and live preview. Normal driving mode locks the grid and has no editor controls or scrolling.

**Step 4: Implement optimistic persistence**

Host endpoints validate schema and session, enforce size/widget limits, and require revision. The UI shows an explicit resolution screen for HTTP 409; it never silently overwrites another device.

**Step 5: Verify import security and recovery**

Test unknown widget types, excessive nesting, oversized files, stale schema, arbitrary script-like strings, corrupt primary layout and last-known-good restoration.

**Step 6: Commit**

```powershell
git add web apps/host crates/config schemas/layout
git commit -m "feat(editor): add safe responsive layout editing"
```

## Task 14: Productize, diagnose and enforce budgets

**Files:**

- Create: `apps/host/src/diagnostics.rs`
- Create: `apps/host/src/tray.rs`
- Create: `apps/host/src/update.rs` only after signing design is reviewed
- Create: `apps/host/tests/diagnostics.rs`
- Create: `tests/performance/host_soak.ps1`
- Create: `tests/performance/browser_trace/`
- Create: `docs/release-checklist.md`
- Modify: release workflow files

**Step 1: Add local metrics and sanitized diagnostics**

Expose packet rate/errors, last-packet age, snapshot overwrites, event resyncs, WebSocket RTT, clients, config recovery, Host RSS and version. Export must redact tokens, raw UDP, IP addresses and player names by default.

**Step 2: Add tray and QR onboarding**

The tray must expose start/stop adapter, open dashboard, show QR, diagnostics and quit. Keep the backend usable headlessly for tests. Verify macOS main-thread requirements before selecting the tray crate version.

**Step 3: Run soak and memory tests**

Replay fixed 60 Hz telemetry to four clients for two hours in a release build. Sample RSS and counters every minute. Fail if the process crashes, channels grow without bound, or steady-state RSS trend exceeds the documented tolerance.

**Step 4: Run end-to-end latency tests**

Instrument Host UDP receipt and browser rAF completion using the clock-offset handshake. Record p50/p95/p99 and packet age for 10 minutes. Acceptance is p95 below 100 ms; investigate any sustained p99 spike rather than hiding it in an average.

**Step 5: Package and test clean machines**

Build signed Windows and macOS candidates. Test install, firewall onboarding, first pairing, upgrade with existing v1 config, failed update rollback and uninstall on clean VMs/devices.

**Step 6: Run the full release gate**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm ci
npm run check:web
npm run test:web
npm run build:web
```

Expected: all pass, plus documented real-device and soak results.

**Step 7: Commit**

```powershell
git add apps tests docs .github
git commit -m "feat: productize the F1 24 MVP"
```

## MVP completion checklist

- F1 24 real telemetry produces correct player vehicle, lap, session, tyre and event data.
- Phone/iPad can pair locally, reconnect, and distinguish disconnected/stale/unsupported states.
- Snapshot backpressure never accumulates old telemetry; events either replay or explicitly resync.
- Core dashboard is stable at 60 FPS on reference devices and satisfies the measured latency budget.
- Layout editing, import/export, version migration and corrupt-file recovery are tested.
- Host runs two hours without crash or unbounded RSS growth.
- Windows/macOS artifacts, firewall onboarding, update rollback and local-only network behavior are verified.
- Architecture documentation, ADRs, protocol revision and performance results match the released implementation.
