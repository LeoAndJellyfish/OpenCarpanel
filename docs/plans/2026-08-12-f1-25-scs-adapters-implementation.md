# F1 25、ETS2 与 ATS Adapters Implementation Plan

**Goal:** Add production-shaped F1 25, Euro Truck Simulator 2, and American Truck Simulator telemetry inputs without regressing F1 24.

**Architecture:** A built-in registry owns one adapter/reducer pipeline per game and selects a sticky active source from a single bounded UDP ingress. F1 24/25 share a validated parser; ETS2/ATS receive a versioned loopback datagram produced by a minimal SCS SDK plugin included in release packages.

**Tech Stack:** Rust 1.91.1, Tokio UDP, Cargo workspace, C++17/CMake, SCS Telemetry SDK 1.14, Node.js 22 packaging, Preact Dashboard.

---

### Task 1: Consolidate the F1 parser and add F1 25

**Files:**
- Create: `crates/adapter-f1/Cargo.toml`
- Create: `crates/adapter-f1/src/*.rs`
- Create: `crates/adapter-f1/tests/f1_24.rs`
- Create: `crates/adapter-f1/tests/f1_25.rs`
- Modify: `Cargo.toml`
- Modify: `apps/host/Cargo.toml`
- Remove after migration: `crates/adapter-f1-24/`

**Step 1: Write format-isolation tests**

Build one 1352-byte packet for format 2024 and one for 2025. Assert `F1_24Adapter` accepts only 2024, `F1_25Adapter` accepts only 2025, and both map speed, gear, RPM, rev lights, throttle, brake and DRS identically.

**Step 2: Run tests to verify the new crate is absent**

Run: `cargo test -p opencarpanel-adapter-f1`

Expected: FAIL because the package does not exist.

**Step 3: Move shared cursor/header/packet/mapping code**

Parameterize header decoding with an immutable protocol descriptor containing adapter ID, display name, packet format and game year. Keep exact header length, packet ID/version, car count and datagram length checks. Do not accept a set or range of formats in one concrete adapter.

**Step 4: Implement the two concrete adapters**

Expose `F1_24Adapter`, `F1_25Adapter`, constants for both adapter IDs/formats, and test-only packet constants needed by Host integration tests. Each descriptor reports its own official protocol revision.

**Step 5: Run focused tests and Clippy**

Run:

```powershell
cargo test -p opencarpanel-adapter-f1
cargo clippy -p opencarpanel-adapter-f1 --all-targets -- -D warnings
```

Expected: all positive, truncation, version, length, enum and cross-format tests pass.

**Step 6: Migrate dependencies and remove the old crate**

Update workspace and Host imports, then remove `adapter-f1-24` only after all references resolve.

**Step 7: Commit**

```powershell
git add Cargo.toml Cargo.lock crates/adapter-f1 apps/host/Cargo.toml apps/host/src apps/host/tests
git add -u crates/adapter-f1-24
git commit -m "feat(f1): add shared F1 24 and F1 25 adapters"
```

### Task 2: Define and decode the SCS bridge protocol

**Files:**
- Create: `crates/adapter-scs/Cargo.toml`
- Create: `crates/adapter-scs/src/lib.rs`
- Create: `crates/adapter-scs/src/protocol.rs`
- Create: `crates/adapter-scs/src/adapter.rs`
- Create: `crates/adapter-scs/tests/protocol.rs`
- Create: `crates/adapter-scs/tests/mapping.rs`
- Modify: `Cargo.toml`

**Step 1: Write wire-contract tests**

Define an exact v1 fixture with magic `OCP\0`, version 1, ETS2/ATS game byte, zero flags/reserved bytes, session nonce, u32 frame sequence, and finite little-endian vehicle fields. Test every truncation point plus wrong magic, version, game, flags, reserved fields, NaN/Infinity and out-of-range pedals.

**Step 2: Run tests to verify failure**

Run: `cargo test -p opencarpanel-adapter-scs`

Expected: FAIL because the package does not exist.

**Step 3: Implement the bounded decoder**

Use an index-based safe cursor; do not cast input bytes to a Rust/C struct. Reject any length other than the exact v1 length. Convert RPM values only after checking finite/non-negative/u16 range. Map signed speed to magnitude for the speedometer and use displayed gear to preserve reverse/neutral.

**Step 4: Implement ETS2 and ATS adapters**

Share one parser and mapper. `Ets2Adapter` rejects ATS game IDs as unsupported protocol and vice versa. Capabilities include speed, gear, RPM, RPM max, throttle and brake; DRS is explicitly unavailable and no rev-light field is fabricated.

**Step 5: Verify adapters**

Run:

```powershell
cargo test -p opencarpanel-adapter-scs
cargo clippy -p opencarpanel-adapter-scs --all-targets -- -D warnings
```

Expected: all contract and mapping tests pass without unsafe code.

**Step 6: Commit**

```powershell
git add Cargo.toml Cargo.lock crates/adapter-scs
git commit -m "feat(scs): decode ETS2 and ATS bridge telemetry"
```

### Task 3: Add built-in adapter registry and sticky source selection

**Files:**
- Create: `apps/host/src/adapters.rs`
- Modify: `apps/host/src/telemetry.rs`
- Modify: `apps/host/src/app.rs`
- Modify: `apps/host/src/lib.rs`
- Modify: `apps/host/Cargo.toml`
- Create: `apps/host/tests/multi_game_ingestion.rs`
- Modify: `apps/host/tests/udp_ingestion.rs`
- Modify: `apps/host/tests/end_to_end_latency.rs`

**Step 1: Write registry behavior tests**

Cover F1 24, F1 25, ETS2 and ATS fixtures through a real Tokio UDP socket. Assert the published snapshot has the correct `meta.gameId`, one malformed datagram increments errors once, and a protocol mismatch that a later adapter accepts does not increment errors.

**Step 2: Write sticky-selection tests with paused Tokio time**

Send F1 24, then ATS before the active timeout and assert F1 remains published. Advance past the timeout, send ATS again, and assert ATS becomes active with a clean ATS snapshot. In fixed F1 25 mode, assert all other protocols are rejected/ignored without switching.

**Step 3: Run focused tests to verify failure**

Run: `cargo test -p opencarpanel-host --test multi_game_ingestion`

Expected: FAIL because registry/configuration does not exist.

**Step 4: Implement registry pipelines**

Each pipeline owns `Box<dyn GameAdapter>` plus its own reducer and reusable output. Decode in deterministic order; return recognized adapter index and whether observable state changed. Prefer a concrete malformed error over unsupported errors only when no adapter recognizes the datagram.

**Step 5: Implement selection configuration**

Add `AdapterSelection::{Auto,F1_24,F1_25,Ets2,Ats}` to `HostConfig`, default `Auto`. Main reads `OPENCARPANEL_GAME` with strict values and actionable errors. Auto mode uses a two-second source lease; fixed mode only attempts the selected pipeline while retaining static metadata for every supported adapter.

**Step 6: Make active adapter metadata observable**

Store immutable adapter summaries and an atomic active index in `HostState`. Keep capability union stable for existing WebSocket clients. Update active metadata before publishing the first snapshot after a switch.

**Step 7: Verify Host**

Run:

```powershell
cargo test -p opencarpanel-host --test multi_game_ingestion
cargo test -p opencarpanel-host --test udp_ingestion
cargo test -p opencarpanel-host --test end_to_end_latency --release -- --nocapture
```

Expected: all four sources work and F1 24 p95 remains below 100 ms.

**Step 8: Commit**

```powershell
git add apps/host Cargo.lock
git commit -m "feat(host): auto-detect built-in game adapters"
```

### Task 4: Expand health and diagnostics without leaking input details

**Files:**
- Modify: `apps/host/src/http.rs`
- Modify: `apps/host/src/diagnostics.rs`
- Modify: `apps/host/src/telemetry.rs`
- Modify: `apps/host/tests/health.rs`
- Modify: `apps/host/tests/diagnostics.rs`
- Modify: `README.md`

**Step 1: Write response-contract tests**

Before packets, assert `selection: auto`, `activeAdapter: null`, and four stable supported adapter IDs. After an F1 25 fixture, assert `activeAdapter: f1-25` and its source counters. Serialize the response and reject tokens, sessions, IPs, player names, install paths and raw packet content.

**Step 2: Implement bounded per-source metrics**

Use the compile-time adapter list, not a dynamic map from untrusted strings. Report recognized packets and optional last-packet age per adapter. Keep total receive/recognized/error counters for existing tooling; malformed packets that no adapter claims remain intentionally unassigned.

**Step 3: Run response tests**

Run:

```powershell
cargo test -p opencarpanel-host --test health
cargo test -p opencarpanel-host --test diagnostics
```

Expected: exact JSON assertions pass and no sensitive field appears.

**Step 4: Commit**

```powershell
git add apps/host/src apps/host/tests README.md
git commit -m "feat(host): expose multi-game source diagnostics"
```

### Task 5: Build the minimal SCS SDK plugin

**Files:**
- Create: `plugins/scs-telemetry-bridge/CMakeLists.txt`
- Create: `plugins/scs-telemetry-bridge/src/plugin.cpp`
- Create: `plugins/scs-telemetry-bridge/src/bridge_protocol.{hpp,cpp}`
- Create: `plugins/scs-telemetry-bridge/tests/bridge_protocol_test.cpp`
- Create: `plugins/scs-telemetry-bridge/vendor/scs-sdk-1.14/**`
- Create: `NOTICE`
- Modify: `.gitattributes`

**Step 1: Vendor only official SDK inputs**

Copy required headers unchanged from the official SCS SDK 1.14 archive and include its license. Record archive URL/version and SHA-256 in `vendor/scs-sdk-1.14/README.md`. Do not vendor sample binaries or third-party shared-memory code.

**Step 2: Write a pure encoder contract**

Keep packet serialization in a function that accepts primitive state and writes a fixed `std::array<std::uint8_t, N>`. A native test compares every byte to the Rust fixture, including negative gear and floating-point values.

**Step 3: Implement SDK lifecycle**

Export `scs_telemetry_init` and `scs_telemetry_shutdown` with official macros. Negotiate the official telemetry API 1.01 used by the SDK sample, recognize only official `eut2`/`ats` game IDs, register frame/pause plus speed, engine RPM, displayed gear, effective throttle/brake channels and RPM-limit configuration, and close the socket idempotently on shutdown/re-init.

**Step 4: Implement non-blocking loopback transport**

Create an IPv4 UDP socket targeting only `127.0.0.1:20777`; configure non-blocking mode on every supported OS. Send one datagram at frame end while unpaused. Never allocate, lock, resolve DNS, open inbound ports or retry synchronously on the game thread.

**Step 5: Build and test locally**

Run:

```powershell
cmake -S plugins/scs-telemetry-bridge -B target/scs-plugin -DBUILD_TESTING=ON
cmake --build target/scs-plugin --config Release
ctest --test-dir target/scs-plugin -C Release --output-on-failure
```

Expected: plugin and wire-contract test build; CTest passes.

**Step 6: Inspect exported symbols**

On Windows use `dumpbin /exports`; on macOS use `nm -gU`. Assert both required SCS symbols exist and no Host/network server entrypoint is exported.

**Step 7: Commit**

```powershell
git add plugins vendor NOTICE .gitattributes
git commit -m "feat(scs): add bundled telemetry bridge plugin"
```

### Task 6: Package plugins and document setup

**Files:**
- Create: `tools/build-scs-plugin.mjs`
- Modify: `tools/package-release.mjs`
- Modify: `package.json`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/package.yml`
- Create: `docs/quickstart-multi-game.md`
- Create: `docs/protocols/f1-25.md`
- Create: `docs/protocols/scs-bridge-v1.md`
- Modify: `README.md`
- Modify: `docs/README.md`
- Modify: `docs/release-checklist.md`

**Step 1: Add a cross-platform plugin build wrapper**

Use `spawnSync` argument arrays, never shell-concatenated paths. Configure under `target/scs-plugin`, build Release, run CTest, then print the resolved artifact path. Fail clearly when CMake/compiler is absent.

**Step 2: Add package assertions**

`package-release.mjs` must refuse packaging if the current-platform plugin, SCS license, NOTICE or multi-game guide is missing. Copy them under `plugins/scs/` and add plugin filename/protocol version to `build-info.json`.

**Step 3: Update CI**

Rust/Web CI additionally builds and tests the SCS plugin on Windows/macOS. Package workflow uses the same wrapper before staging artifacts. Keep timeouts and unsigned-preview wording accurate.

**Step 4: Write user setup and troubleshooting docs**

Document F1 25 UDP Telemetry On, loopback IP, port 20777, 60 Hz, and UDP mode `F1 25 / 2025`. Document exact ETS2/ATS plugin directories, SDK confirmation dialog, game restart, diagnostic fields and how to lock selection with `OPENCARPANEL_GAME`.

**Step 5: Run package smoke**

Run:

```powershell
npm run build:scs-plugin
npm run package:host
npm run test:package-smoke
Get-ChildItem -Recurse dist/release/OpenCarpanel-*
```

Expected: Host, platform plugin, Apache license, SCS license, NOTICE, docs and build info are present.

**Step 6: Commit**

```powershell
git add package.json package-lock.json tools .github README.md docs
git commit -m "build: package multi-game adapters and SCS plugin"
```

### Task 7: Run full gates and audit completion

**Files:**
- Modify only files required by discovered defects.

**Step 1: Run formatting and static analysis**

```powershell
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
git diff --check
```

Expected: PASS with Rust 1.91.1.

**Step 2: Run all automated tests**

```powershell
cargo test --locked --workspace
npm run check:web
npm run test:web
npm run build:web
npm run build:scs-plugin
npm run test:host-latency
```

Expected: PASS; latency p95 remains below 100 ms.

**Step 3: Verify release contents**

Run `npm run package:host` and `npm run test:package-smoke`; inspect the staged tree. The smoke runner launches the packaged Host, requests `/api/v1/health` and `/api/v1/diagnostics`, and sends one synthetic contract packet for each of the four adapters.

Expected: all sources become active in turn and the package contains installation material.

**Step 4: Record honest real-game status**

Update `docs/release-checklist.md`: automated protocol/packaging checks may be checked; F1 25/ETS2/ATS real-game tests remain unchecked until run against installed games. Do not claim real-game validation from synthetic fixtures.

**Step 5: Final commit**

```powershell
git add -A
git commit -m "test: verify multi-game telemetry release"
```
