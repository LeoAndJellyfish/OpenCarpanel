# Rust game plugin example

This example recognizes a six-byte UDP packet: ASCII `OCP1`, followed by a
little-endian `u16` speed in km/h. It converts that value to canonical m/s and
uses the SDK macro to export ABI v1 without WASI or Host imports.

```powershell
rustup target add wasm32-unknown-unknown
cargo build -p opencarpanel-example-game-plugin --target wasm32-unknown-unknown --release
cargo run -p opencarpanel-game-plugin-cli -- pack examples/game-plugin-rust/manifest.json target/wasm32-unknown-unknown/release/opencarpanel_example_game_plugin.wasm example-sim.ocp-plugin
cargo run -p opencarpanel-game-plugin-cli -- validate example-sim.ocp-plugin
```

Install the resulting file from the desktop control center's **游戏设置**
page. The complete contract and compatibility rules are in
[`docs/plugin-development.md`](../../docs/plugin-development.md).
