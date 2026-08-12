# ADR-0008：Tauri 桌面控制中心内嵌唯一 Host 核心

## Status

Accepted

## Context

v0.1.x 的无头 Rust Host 已能接收 F1/SCS UDP、托管手机 Dashboard、配对并提供诊断，但终端二维码、环境变量和手工复制插件不适合作为大众默认体验。v0.2.0 需要 Windows/macOS 图形控制中心、托盘常驻、开机启动、文件选择、通知与自动更新，同时保留低内存 CLI。GUI 与 CLI 不能各自启动一个 Host，否则会争用 `20777/20778`，并产生两套状态与设备权限。

## Decision

采用 Tauri 2 + Preact/TypeScript 构建默认桌面入口。Tauri 进程直接依赖并启动现有 `opencarpanel-host` library；不 spawn Host 子进程，也不通过 localhost HTTP 管理自身。Rust command 只返回经过裁剪的设置、诊断、设备和安装状态。

GUI 与 `opencarpanel-host` CLI 在启动网络 listener 前获取同一个用户级 OS 文件锁。锁由打开的文件句柄持有；相邻的只读 owner JSON 记录 PID、模式、版本和启动时间，避免 Windows 锁区阻止诊断读取。进程异常退出时由操作系统释放。锁冲突报告“已有 OpenCarpanel 正在运行”并退出；锁成功但端口 bind 失败则明确报告“端口被其他程序占用”。测试可通过专用环境变量把锁目录隔离到临时目录。

主窗口关闭时默认隐藏到系统托盘，Host、UDP 和手机 Dashboard 继续运行；只有托盘“退出”、系统退出或显式更新重启才停止 Host。无头 CLI 与 GUI 使用同一设置仓库、设备仓库、数据目录、日志格式和实例锁。

手机/iPad 继续访问 Rust Host 内嵌的局域网 Web App。Dashboard 编辑器由系统浏览器打开 `http://127.0.0.1:<port>/edit`；远程/localhost 页面不获得 Tauri capability。桌面打包同时包含 GUI、独立 CLI、SCS bridge、许可和文档。

自动更新使用 Tauri updater 的 HTTPS endpoint 与 minisign 签名。公钥编译进应用，私钥只存在于受保护的发布环境；下载、签名或安装失败均保持当前安装不变。用户可禁用自动检查，安装更新必须由用户确认。平台代码签名/notarization 与 updater artifact 签名是两套独立门禁，发布说明不得混淆。

## Consequences

### Positive

- Rust Host、adapter、协议与 Web Dashboard 无需重写，GUI 与 CLI 行为共享。
- WebView 只承载控制中心，实时手机渲染链路和低延迟数据面不变。
- 单实例锁覆盖 GUI/CLI 组合，而不只覆盖两个 Tauri 窗口。
- Tauri capability、严格 CSP 与小型 command allowlist 缩小桌面前端失陷后的权限面。
- 用户可以从托盘、向导和诊断页完成日常操作，不再依赖终端。

### Negative

- GUI 常驻时增加一个系统 WebView 的内存成本，须单独记录 headed/headless RSS。
- Windows/macOS 安装器、更新签名和平台签名扩大了发布矩阵。
- 更改网络 listener 或固定游戏选择需要受监督地重启内嵌 Host，现有手机连接会短暂重连。

### Neutral

- CLI 仍是完整产品入口，而不是 GUI 的 sidecar；安装包中的 CLI 也不会被 GUI 启动。
- 将来若增加只读赛道 overlay，可评估 Slint/egui；这不改变本 ADR 的控制中心边界。

## Alternatives Considered

- **GUI spawn CLI：** 复用可执行文件简单，但带来子进程生命周期、日志转发、崩溃孤儿、端口竞争和第二套 IPC，拒绝。
- **后台系统服务 + GUI client：** 可以独立升级核心，但需要管理员安装、服务权限和版本协商，不符合当前本地轻量目标，拒绝。
- **Electron：** 前端生态成熟，但安装体积和常驻内存更高，且不能直接复用 Rust 生命周期，拒绝。
- **egui/Iced/Slint 全 Rust GUI：** 运行时轻，但现有 Preact 设计系统、响应式表单和编辑器无法直接复用，当前不采用。
- **仅使用 Tauri single-instance plugin：** 只能可靠约束 Tauri GUI，无法覆盖独立 CLI，因此以共享 OS 锁取代。

## References

- [v0.2.0 桌面架构设计](../plans/2026-08-12-v0.2-desktop-control-center-design.md)
- [v0.2.0 实施计划](../plans/2026-08-12-v0.2-desktop-control-center-implementation.md)
- [ADR-0001](0001-rust-modular-monolith.md)
- [ADR-0005](0005-local-http-web-app-first.md)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri updater](https://v2.tauri.app/plugin/updater/)
