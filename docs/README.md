# OpenCarpanel 文档索引

## 核心文档

- [`plans/2026-08-11-opencarpanel-architecture-design.md`](plans/2026-08-11-opencarpanel-architecture-design.md)：已确认的系统架构与质量指标。
- [`plans/2026-08-12-multi-game-adapters-design.md`](plans/2026-08-12-multi-game-adapters-design.md)：F1 24/25 与 SCS 游戏的输入、选择、故障和发布架构。
- [`plans/2026-08-12-f1-25-2026-season-pack-design.md`](plans/2026-08-12-f1-25-2026-season-pack-design.md)：2026 UDP 包布局以及按 `gameId` 自动切换前端、隔离布局的设计。
- [`plans/2026-08-12-f1-25-2026-season-pack-implementation.md`](plans/2026-08-12-f1-25-2026-season-pack-implementation.md)：对应的测试优先实施与验证步骤。
- [`plans/2026-08-12-v0.2-desktop-control-center-design.md`](plans/2026-08-12-v0.2-desktop-control-center-design.md)：Tauri 桌面控制中心、单 Host、配置、托盘和签名更新设计。
- [`plans/2026-08-14-dashboard-widget-toggle-design.md`](plans/2026-08-14-dashboard-widget-toggle-design.md)：按游戏过滤的 Steam 浮层式组件开关与兼容性设计。
- [`plans/2026-08-14-desktop-update-progress-design.md`](plans/2026-08-14-desktop-update-progress-design.md)：应用内更新下载、验签和安装阶段的进度反馈设计。
- [`data-paths-and-scs-packet.md`](data-paths-and-scs-packet.md)：四款游戏的端到端数据链路、三套 F1 精确包长与 SCS bridge v1/v2 数组图解。
- [`plans/2026-08-11-f1-24-mvp-implementation.md`](plans/2026-08-11-f1-24-mvp-implementation.md)：按测试优先顺序拆分的实施计划。
- [`adr/`](adr/)：重要且长期有效的架构决策记录。
- [`protocols/f1-24.md`](protocols/f1-24.md)：F1 24 官方资料入口和适配器实现边界。
- [`protocols/f1-25.md`](protocols/f1-25.md)：F1 25 原始 2025 与 2026 Season Pack UDP 的精确实现边界。
- [`protocols/scs-bridge-v1.md`](protocols/scs-bridge-v1.md)：ETS2/ATS 44-byte 旧插件兼容协议。
- [`protocols/scs-bridge-v2.md`](protocols/scs-bridge-v2.md)：当前 188-byte 插件的导航、油量、灯光与任务协议。
- [`quickstart-multi-game.md`](quickstart-multi-game.md)：四款游戏的统一入口、来源选择和分段排障。
- [`quickstart-f1-24.md`](quickstart-f1-24.md)、[`quickstart-f1-25.md`](quickstart-f1-25.md)、[`quickstart-scs.md`](quickstart-scs.md)：各游戏首启指南。
- [`release-checklist.md`](release-checklist.md)：自动化、实机性能、签名与发布验收清单。
- [`releases/v0.1.0.md`](releases/v0.1.0.md)：首个公开预览版的发布说明与已知限制。
- [`releases/v0.1.1.md`](releases/v0.1.1.md)：2026 Season Pack 与游戏自适应 Dashboard 小版本说明。
- [`releases/v0.2.0.md`](releases/v0.2.0.md)：桌面控制中心大版本说明、安装与已知边界。
- [`releases/v0.2.1.md`](releases/v0.2.1.md)：SCS 游戏目录选择修复、兼容性与验证说明。
- [`releases/v0.2.2.md`](releases/v0.2.2.md)：桌面控制中心文案精简、兼容性与验证说明。
- [`releases/v0.3.0.md`](releases/v0.3.0.md)：F1 与 SCS 遥测扩展、bridge v2、兼容性与验证说明。
- [`releases/v0.3.1.md`](releases/v0.3.1.md)：游戏自适应遥测面板、默认布局安全迁移与响应式验证说明。
- [`releases/v0.3.2.md`](releases/v0.3.2.md)：按游戏过滤的组件开关、桌面更新进度与兼容性说明。
- [`releases/v0.3.3.md`](releases/v0.3.3.md)：ETS2/ATS Steam 自动发现、本地路径缓存与安全兜底说明。

架构设计描述“系统应当是什么样”，实施计划描述“按什么顺序把它建出来”，ADR 解释“为什么选择该方案”。
