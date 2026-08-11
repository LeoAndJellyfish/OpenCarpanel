# ADR-0001：使用 Rust 模块化单体 Host

## Status

Accepted

## Context

OpenCarpanel 需要在 Windows/macOS 常驻运行，接收高频 UDP、托管本地 Web App，并保持较低内存和简单安装。项目初期团队和部署规模都不需要多个独立服务。Electron/Node 可以提升早期开发速度，Tauri 可以提供完整桌面窗口，但两者都会让常驻 WebView 成为基础成本。

## Decision

使用 Rust 构建一个 Host 可执行程序。运行时是单进程，代码通过 Cargo Workspace 划分适配器、领域核心、协议、配置和应用外壳。桌面端只保留轻量托盘与浏览器设置入口，不常驻完整 WebView。

## Consequences

### Positive

- 一个安装包和进程，启动、排障、更新与回滚路径简单。
- Rust 的所有权和类型系统适合处理不可信二进制输入和长期运行任务。
- 模块边界可以独立测试，而不引入跨进程协议。
- 更容易达到 Host RSS 小于 50 MB 的目标。

### Negative

- Rust 二进制协议、异步任务和跨平台托盘需要更严格的工程能力。
- 前端与 Host 使用两种语言，需要生成并测试跨语言契约。
- 某个模块的严重缺陷仍可能终止整个进程，因此必须有任务监督和 panic 防线。

### Neutral

- 如果未来确实需要运行不受信任的社区适配器，可将适配器隔离为子进程；当前不提前承担该复杂度。

## Alternatives Considered

- **Tauri + Rust：** 配置窗口更完整，但 WebView 常驻成本与移动 Web App 重复。
- **Electron/Node：** 开发速度快，但内存和安装体积不符合当前优先级。
- **微服务：** 没有独立扩缩容或团队自治需求，运维成本不合理。

## References

- [主架构设计](../plans/2026-08-11-opencarpanel-architecture-design.md)
