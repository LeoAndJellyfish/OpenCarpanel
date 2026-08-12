# F1 25 2026 Season Pack Compatibility Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the stable `f1-25` adapter decode both original format 2025 and the F1 25 2026 Season Pack format 2026 without weakening packet validation.

**Architecture:** Model each packed Car Telemetry wire shape as immutable layout metadata selected by the header's exact `packetFormat`. Reuse the common 29-byte header and first 20 bytes of verified player telemetry fields, while validating format-specific car counts, per-car sizes, and total datagram lengths.

**Tech Stack:** Rust 1.91.1, Cargo workspace tests, Tokio UDP integration tests, Node.js packaged-host smoke tests, Markdown/SVG documentation.

---

### Task 1: Add failing 2026 protocol contract tests

**Files:**
- Modify: `crates/adapter-f1/tests/header_decoder.rs`
- Modify: `crates/adapter-f1/tests/car_telemetry.rs`

**Step 1: Add the format constant to header coverage**

Add `F1_25_2026_PACKET_FORMAT` and a `(2026, 26)` case to common-header decoding and truncation tests.

**Step 2: Build the official 2026 packed layout in the test helper**

Generate a `1448`-byte packet with `24` entries of `59` bytes, put the player at index `23`, and write the existing mapped fields at offsets `0, 2, 10, 15, 16, 18, 19` within that entry.

**Step 3: Assert logical compatibility and strict isolation**

Assert `F1_25Adapter` and `decode_f1_25_player_car_telemetry` accept both 2025 and 2026, while `F1_24Adapter` rejects both. Assert format 2026 with 1352 bytes and player index 24 are rejected with layout-specific errors.

**Step 4: Run the focused tests and observe failure**

Run: `cargo +stable test -p opencarpanel-adapter-f1`

Expected: FAIL because format 2026 and its layout constants are not implemented yet.

### Task 2: Introduce format-specific Car Telemetry layouts

**Files:**
- Modify: `crates/adapter-f1/src/lib.rs`
- Modify: `crates/adapter-f1/src/header.rs`
- Modify: `crates/adapter-f1/src/error.rs`
- Modify: `crates/adapter-f1/src/packets/mod.rs`
- Modify: `crates/adapter-f1/src/packets/car_telemetry.rs`
- Modify: `crates/adapter-f1/src/adapter.rs`

**Step 1: Define immutable layout metadata**

Add 2024/2025 layout `(22, 60, 1352)` and 2026 layout `(24, 59, 1448)`. Configure F1 24 with one layout and F1 25 with the two supported layouts.

**Step 2: Decode the header before selecting a layout**

Keep `PacketHeader::decode(datagram, expected_format)` for its public strict contract, and add an internal decode path that reads the common header once. Match `header.packet_format` against the current protocol's layouts; return an unsupported-protocol error listing the allowed formats when no match exists.

**Step 3: Parameterize packet validation and player selection**

Change `decode_player_sample` to accept the selected layout. Validate exact total length and index against that layout, calculate the player offset using its per-car length, then skip `layout.car_telemetry_data_len - 20` unmapped bytes.

**Step 4: Run focused tests**

Run: `cargo +stable test -p opencarpanel-adapter-f1`

Expected: PASS, including index 23 and malformed cross-layout cases.

### Task 3: Prove Host selection and packaged behavior

**Files:**
- Modify: `apps/host/src/adapters.rs`
- Modify: `apps/host/tests/game_adapters.rs`
- Modify: `tools/smoke-package.mjs`

**Step 1: Add Host UDP cases for both F1 25 modes**

Send format 2025/1352 and format 2026/1448 through real Tokio UDP sockets. Both must publish `meta.gameId == "f1-25"`; fixed `AdapterSelection::F1_25` must accept both.

**Step 2: Extend package smoke**

Add a fifth datagram case for format 2026 while keeping `supportedAdapters` equal to `f1-24,f1-25,ets2,ats`. Update expected packet counters to 5/5.

**Step 3: Run Host tests**

Run: `cargo +stable test -p opencarpanel-host`

Expected: PASS with both UDP formats sharing the stable adapter.

### Task 4: Add automatic game presentation profiles

**Files:**
- Modify: `web/widget-sdk/src/layout.ts`
- Modify: `web/widget-sdk/src/layout.test.ts`
- Create: `web/dashboard/src/dashboard/game-profile.ts`
- Create: `web/dashboard/src/dashboard/game-profile.test.ts`
- Modify: `web/dashboard/src/telemetry/store.ts`
- Modify: `web/dashboard/src/telemetry/store.test.ts`
- Modify: `web/dashboard/src/telemetry/use-runtime.ts`
- Modify: `web/dashboard/src/routes/drive.tsx`
- Modify: `web/dashboard/src/routes/edit.tsx`
- Modify: `web/dashboard/src/dashboard/dashboard.tsx`
- Modify: `web/dashboard/src/dashboard/layout-grid.tsx`
- Modify: `web/dashboard/src/dashboard/widget-view.tsx`
- Modify: `web/dashboard/src/dashboard/status-rail.tsx`
- Modify: `web/dashboard/src/styles/dashboard.css`
- Modify: `apps/host/src/layout_api.rs`
- Modify: `apps/host/tests/layout_api.rs`

**Step 1: Add failing profile and layout tests**

Assert four game IDs map to four stable layout IDs, F1 and truck families use different collision-free responsive placements, unknown games get a safe neutral profile, and `meta.gameId` notifies only when it changes.

**Step 2: Add per-game built-in and persisted layouts**

Define immutable defaults in the Widget SDK and matching Host defaults for `game-f1-24`, `game-f1-25`, `game-ets2`, and `game-ats`. Keep the existing `default` layout for backward compatibility.

**Step 3: Switch the driving page on logical game changes**

Expose low-frequency `gameId` state from the telemetry runtime. On a change, immediately apply the built-in profile and asynchronously load the game-specific user layout. Pass the profile through widgets so truck pages do not show DRS.

**Step 4: Make each profile visually distinct and editable**

Set game/family data attributes, dynamic footer labels, profile-specific backgrounds, and an editor game selector. Keep high-frequency numeric rendering in the existing rAF loop.

**Step 5: Run frontend and Host layout tests**

Run: `npm run test:web`

Run: `cargo +stable test -p opencarpanel-host --test layout_api`

Expected: PASS with profile switching and five safe default layout IDs including the legacy default.

### Task 5: Update user-facing protocol boundaries

**Files:**
- Modify: `README.md`
- Modify: `docs/protocols/f1-25.md`
- Modify: `docs/quickstart-f1-25.md`
- Modify: `docs/quickstart-multi-game.md`
- Modify: `docs/data-paths-and-scs-packet.md`
- Modify: `docs/assets/supported-game-data-paths.svg`
- Modify: `docs/release-checklist.md`
- Modify: `docs/README.md`

**Step 1: Remove obsolete unsupported warnings**

State that the default 2026 Season Pack mode works directly and that original 2025 mode remains supported.

**Step 2: Document exact wire layouts**

Record format, packet id/version, car count, entry size, total size, mapped fields, and unsupported Car Telemetry 2 fields with the official EA source.

**Step 3: Preserve honest validation status**

Add synthetic/CI checks as automated evidence and leave real-game Season Pack validation unchecked.

### Task 6: Verify the complete change

**Files:**
- Review: all modified files

**Step 1: Format and lint**

Run: `cargo +stable fmt --all -- --check`

Run: `cargo +stable clippy --locked --workspace --all-targets -- -D warnings`

**Step 2: Test Rust and web/package boundaries**

Run: `cargo +stable test --locked --workspace`

Run: `npm run check:web`

Run: `npm run test:web`

Run: `npm run package:host`

Run: `npm run test:package-smoke`

**Step 3: Inspect repository hygiene**

Run: `git diff --check`

Run: `git status --short`

Expected: no generated research files are tracked; only source, tests, and documentation appear in the diff.
