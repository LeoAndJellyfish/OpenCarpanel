# Game plugins

Each directory is one complete game-support declaration. Built-in plugins use
the same manifest schema as installable `.ocp-plugin` packages; only their
`runtime.kind` differs.

| Plugin | Decoder | Setup |
| --- | --- | --- |
| `f1-24` | built-in Rust adapter | EA UDP 2024 |
| `f1-25` | built-in Rust adapter | EA UDP 2025 / 2026 Season Pack |
| `ets2` | built-in Rust adapter | bundled SCS SDK bridge |
| `ats` | built-in Rust adapter | bundled SCS SDK bridge |

Third-party plugins must declare `runtime.kind: "wasm"`. See
[`docs/plugin-development.md`](../../docs/plugin-development.md) and the Rust
SDK example before creating a package.
