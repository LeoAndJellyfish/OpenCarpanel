# ADR-0007：使用版本化本机桥接接入游戏内遥测 SDK

## Status

Accepted

## Context

F1 24/25 可直接把官方 UDP 数据发给 Host，ETS2/ATS 的官方接口则是由游戏加载动态库并调用 SCS Telemetry SDK callback。Host 不能在独立进程中直接调用这些 callback。项目需要在不依赖远端服务、第三方常驻程序或不稳定 Rust 动态 ABI 的前提下统一两类输入，同时避免把复杂网络和业务逻辑放进游戏进程。

## Decision

为需要游戏内 SDK 的游戏提供项目自带、最小化的原生桥接插件。SCS 插件只注册必要 telemetry channel，将每帧最新值编码成固定上限、显式版本、小端本机 UDP 数据报，并发送到硬编码的 `127.0.0.1:20777`。Host 负责验证、适配、统一状态、来源选择、诊断和 WebSocket 发布。

桥接数据报不是 C/C++ 内存布局，也不复用 Rust ABI。协议包含 magic、版本、游戏 ID、session、frame 和有界字段；所有保留位必须为零。ETS2 与 ATS 共用 transport schema，但保留独立 adapter ID 和 reducer。

F1 继续使用游戏原生 UDP，不经过桥接插件。Host 的 adapter registry 将原生 UDP 与桥接数据报视为不同 ingress protocol，最终都进入相同 `GameAdapter -> TelemetryReducer` 边界。

## Consequences

### Positive

- 用户不依赖第三方共享内存插件或外部桥接进程。
- 游戏内代码极小、固定内存、非阻塞，复杂错误处理留在 Rust Host。
- 版本化字节协议可以独立 fuzz、回放和升级，不受编译器 ABI 影响。
- ETS2/ATS 与后续 SDK 型游戏可复用本机桥接模式。

### Negative

- 项目需要维护和签名额外的 Windows/macOS 原生插件 artifact。
- 插件必须随 SCS SDK 和游戏更新做实机兼容测试。
- 首次使用欧卡/美卡仍需要把插件安装到游戏目录并接受 SDK 提示。

### Neutral

- Host 仍是唯一面向用户的常驻进程；插件只随游戏进程存在。
- Linux 构建定义可以保留，但当前正式发布矩阵仍为 Windows/macOS。

## Alternatives Considered

**第三方共享内存插件**

ETS2LA 通过 `truckermudgeon/scs-sdk-plugin` 的 RenCloud fork 把 callback 写入 32 KiB `SCSTelemetry` shared memory，并由主程序约以 60 Hz 读取。初始复用更快，也可作为未来的可选兼容 ingress；但安装、版本、布局、供应链和故障归属受外部项目控制，因此不作为默认路径。

**独立桥接进程**

隔离较强，但增加常驻内存、启动顺序和排障复杂度，不符合本地单进程产品体验。

**在 Rust Host 中加载游戏 SDK**

SCS callback 只存在于游戏加载的动态库中，独立 Host 无法直接获得；把 Host 本身注入游戏也扩大崩溃和安全风险。

**插件直接提供 LAN HTTP/WebSocket**

会在游戏进程中复制配对、网络、安全和前端托管逻辑，风险和维护成本不可接受。

## References

- [多游戏输入与适配器设计](../plans/2026-08-12-multi-game-adapters-design.md)
- [SCS Telemetry SDK](https://modding.scssoft.com/wiki/Documentation/Engine/SDK/Telemetry)
- [ETS2LA telemetry reader](https://github.com/ETS2LA/ETS2LA/blob/main/ETS2LA.Game/Telemetry/Program.cs)
- [truckermudgeon/scs-sdk-plugin](https://github.com/truckermudgeon/scs-sdk-plugin)
- [ADR-0001](0001-rust-modular-monolith.md)
- [ADR-0002](0002-adapter-api-and-canonical-telemetry.md)
