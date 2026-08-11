# OpenCarpanel

OpenCarpanel 是一个本地运行、跨平台、跨游戏的驾驶遥测仪表盘。电脑端 Host 接收游戏遥测，手机或 iPad 通过同一局域网打开仪表盘。

当前仓库处于架构与工程骨架阶段，首个 MVP 面向 **F1 24**。

## 已确认的方向

- Rust 模块化单体 Host，支持 Windows 和 macOS。
- 手机/iPad 使用 PWA-ready Web App，无需远程运行服务。
- F1 24 UDP 作为首个游戏适配器。
- WebSocket 最新状态通道与可靠事件通道。
- TypeScript + Preact 的组件化面板和拖拽编辑器。
- 普通设备稳定 60 FPS，高刷新率设备争取 120 FPS。

## 文档

- [架构设计](docs/plans/2026-08-11-opencarpanel-architecture-design.md)
- [F1 24 MVP 实施计划](docs/plans/2026-08-11-f1-24-mvp-implementation.md)
- [架构决策记录](docs/adr/README.md)
- [F1 24 协议资料入口](docs/protocols/f1-24.md)

## 工作区

```text
apps/       可执行应用
crates/     Rust 领域模块与游戏适配器
web/        仪表盘与组件 SDK
schemas/    跨语言协议和配置 Schema
tests/      集成、回放和性能测试约定
tools/      开发与发布辅助工具
docs/       架构、ADR、协议资料和实施计划
```

运行基础检查：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run check:web
```
