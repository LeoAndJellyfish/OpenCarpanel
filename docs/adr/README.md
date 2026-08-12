# Architecture Decision Records

ADR 记录已经确认、会长期影响实现的决策。已有决策不直接重写；如果方向改变，新增 ADR 并在原文标记 `Superseded`。

| ADR | 状态 | 决策 |
| --- | --- | --- |
| [0001](0001-rust-modular-monolith.md) | Superseded in part | Rust 模块化单体 Host；桌面入口由 ADR-0008 更新 |
| [0002](0002-adapter-api-and-canonical-telemetry.md) | Accepted | Adapter API 与统一遥测模型 |
| [0003](0003-dual-lane-websocket-protocol.md) | Accepted | Snapshot/Event 双通道 WebSocket |
| [0004](0004-versioned-json-persistence.md) | Accepted | 版本化 JSON 持久化，MVP 不使用数据库 |
| [0005](0005-local-http-web-app-first.md) | Accepted | MVP 先交付本地 HTTP Web App |
| [0006](0006-render-loop-and-motion-budget.md) | Accepted | 独立渲染循环与运动性能预算 |
| [0007](0007-versioned-local-game-input-bridges.md) | Accepted | 版本化本机桥接接入游戏内遥测 SDK |
| [0008](0008-tauri-desktop-embedded-host.md) | Accepted | Tauri 桌面控制中心内嵌唯一 Host 核心 |

新 ADR 使用四位递增编号，并包含 Context、Decision、Consequences、Alternatives 和 References。
