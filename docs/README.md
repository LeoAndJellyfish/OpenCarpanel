# OpenCarpanel 文档索引

## 核心文档

- [`plans/2026-08-11-opencarpanel-architecture-design.md`](plans/2026-08-11-opencarpanel-architecture-design.md)：已确认的系统架构与质量指标。
- [`plans/2026-08-12-multi-game-adapters-design.md`](plans/2026-08-12-multi-game-adapters-design.md)：F1 24/25 与 SCS 游戏的输入、选择、故障和发布架构。
- [`data-paths-and-scs-packet.md`](data-paths-and-scs-packet.md)：四款游戏的端到端数据链路与 SCS bridge 44 字节数组图解。
- [`plans/2026-08-11-f1-24-mvp-implementation.md`](plans/2026-08-11-f1-24-mvp-implementation.md)：按测试优先顺序拆分的实施计划。
- [`adr/`](adr/)：重要且长期有效的架构决策记录。
- [`protocols/f1-24.md`](protocols/f1-24.md)：F1 24 官方资料入口和适配器实现边界。
- [`protocols/f1-25.md`](protocols/f1-25.md)：F1 25 原始 2025 UDP 格式与 2026 Season Pack 边界。
- [`protocols/scs-bridge-v1.md`](protocols/scs-bridge-v1.md)：ETS2/ATS 原生插件到 Host 的固定本机协议。
- [`quickstart-multi-game.md`](quickstart-multi-game.md)：四款游戏的统一入口、来源选择和分段排障。
- [`quickstart-f1-24.md`](quickstart-f1-24.md)、[`quickstart-f1-25.md`](quickstart-f1-25.md)、[`quickstart-scs.md`](quickstart-scs.md)：各游戏首启指南。
- [`release-checklist.md`](release-checklist.md)：自动化、实机性能、签名与发布验收清单。
- [`releases/v0.1.0.md`](releases/v0.1.0.md)：首个公开预览版的发布说明与已知限制。

架构设计描述“系统应当是什么样”，实施计划描述“按什么顺序把它建出来”，ADR 解释“为什么选择该方案”。
