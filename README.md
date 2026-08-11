# OpenCarpanel

OpenCarpanel 是本地运行的跨平台驾驶遥测仪表盘：Windows/macOS Host 接收游戏数据，手机或 iPad 通过同一局域网显示高帧率 Dashboard。运行时不依赖远程服务器。

当前 `0.1.x` 是面向 **F1 24** 的首个可用预览版，已经打通：

- Rust UDP Host → F1 24 安全解析 → 统一遥测状态 → 有界 WebSocket。
- 15 分钟一次性二维码配对、设备 session、自动重连与 stale 状态。
- Preact 驾驶视图，速度、档位、RPM/转速灯、油门、刹车和 DRS 以单一 rAF 循环更新。
- 手机竖屏/横屏、iPad 与桌面断点；编辑器支持拖动、缩放、撤销重做、主题、原子保存和安全 JSON 导入导出。
- Dashboard 内嵌 Rust 二进制，严格 CSP，无 CDN、云端账户或远程遥测。
- 本地诊断、合成延迟门禁、四客户端 60 Hz soak 工具，以及 Windows/macOS CI。

当前 F1 adapter 只实现官方 packet id 6 的玩家车辆核心字段；圈速、比赛状态、轮胎、损伤和处罚/事件尚未完成。因此它是可驾驶使用的预览版，不是功能完整的稳定版。

## 快速运行

前置条件：Node.js 22+ 与 Rust stable。

```powershell
npm ci
npm run build:host
.\target\release\opencarpanel-host.exe
```

macOS 最后一行改为：

```bash
./target/release/opencarpanel-host
```

启动后扫描终端二维码。F1 24 使用 UDP Format `2024`、端口 `20777`、发送率 `60Hz`；游戏和 Host 在同一电脑时 IP 填 `127.0.0.1`。

完整步骤、防火墙、异机游戏配置和排障见 [F1 24 快速开始](docs/quickstart-f1-24.md)。

## 常用命令

```powershell
# 全量质量门禁
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
npm run check:web
npm run test:web
npm run build:web

# 生成当前平台 unsigned preview 目录
npm run package:host

# 本机 UDP → WebSocket p95 门禁
npm run test:host-latency

# 默认两小时、四客户端、60 Hz Host soak
npm run test:host-soak
```

Host 运行时可访问：

- `http://127.0.0.1:20778/api/v1/health`
- `http://127.0.0.1:20778/api/v1/diagnostics`

## 架构

```text
F1 24 UDP
    ↓
adapter-f1-24 → telemetry-core → latest snapshot / event ring
                                      ↓
                          paired HTTP + WebSocket Host
                                      ↓
                     phone / iPad Preact Dashboard + Editor
```

仓库按稳定边界组织：

```text
apps/       可执行 Host
crates/     Adapter API、F1 24、telemetry、protocol、config
web/        Dashboard 与 Widget SDK
schemas/    版本化 JSON Schema 和生成类型
tests/      fixture、集成与性能门禁
tools/      回放、类型生成、包体和发布工具
docs/       架构、ADR、协议、首启和发布清单
```

详细资料：

- [系统架构设计](docs/plans/2026-08-11-opencarpanel-architecture-design.md)
- [F1 24 MVP 实施计划](docs/plans/2026-08-11-f1-24-mvp-implementation.md)
- [视觉与动效设计](docs/plans/2026-08-11-f1-dashboard-visual-design.md)
- [架构决策记录](docs/adr/README.md)
- [F1 24 协议基线](docs/protocols/f1-24.md)
- [发布检查清单](docs/release-checklist.md)

## License

OpenCarpanel 使用 [Apache License 2.0](LICENSE)。允许使用、修改、分发和商业使用，但须保留许可证与声明；许可证同时提供明确的专利授权与专利诉讼终止条款。
