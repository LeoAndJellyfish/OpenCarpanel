# ADR-0009：使用声明式清单与沙箱化 WASM 游戏插件

## Status

Accepted

## Context

ADR-0002 建立了 `GameAdapter` 与统一遥测模型，但首版把适配器编译进 Host。当前 F1 24、F1 25、ETS2 和 ATS 的身份、能力、设置方式与仪表盘表现仍分别硬编码在 Host、桌面端和 Dashboard。继续沿用这种方式会使每个新游戏都修改多个应用，也无法让第三方开发者独立分发支持。

插件必须保持 OpenCarpanel 的本地运行、低延迟、低内存和稳定性目标。直接加载 Rust/C 动态库没有稳定 ABI，并会让第三方代码取得 Host 进程权限；为每个插件启动独立进程则增加常驻内存、IPC 和生命周期复杂度。

## Decision

采用版本化 `.ocp-plugin` 包。每个游戏插件由一份 `manifest.json` 和可选的 `decoder.wasm` 组成：

- manifest 是游戏身份、版本、发布者、输入协议、canonical capabilities、仪表盘 presentation、组件清单和设置向导的唯一事实来源；
- 内置插件使用相同 manifest，但把 runtime 指向经过编译的 Rust adapter；
- 第三方插件使用 ABI v1 的 WebAssembly 解码器，安装后无需重新编译 Host；
- Host 统一接收 UDP 数据报并传给 decoder。WASM 不启用 WASI，不导入 Host 函数，因此没有网络、文件、时钟或系统调用权限；
- 每次调用限制输入/输出大小、执行 fuel 和线性内存。输出仍需经过 Host 的结构、数量和语义边界验证；
- Dashboard 只执行项目自带组件。插件可选择适用组件、主题和布局预设，但 v1 不执行第三方 JavaScript；
- 需要游戏内 SDK 的原生桥接仍按 ADR-0007 由可信桌面安装器管理，不在 WASM 插件中获得任意文件写权限。

`.ocp-plugin` v1 是有界 JSON envelope，包含 manifest、Base64 WASM 和 SHA-256。安装器先完整验证，再以 manifest 中经过约束的安全文件名原子写入模块，最后原子替换 manifest。损坏、重复或不兼容插件被隔离并出现在诊断中，不阻止 Host 使用其他插件启动。

## Consequences

### Positive

- 新游戏的名称、能力、布局、设置说明和 decoder 成为一个可测试、可分发单元。
- 第三方插件跨 Windows/macOS 使用同一 WASM artifact，不依赖 Rust ABI。
- 失控 decoder 可确定终止，且不能读取本地文件或自行发送遥测。
- 内置与外部插件经过同一注册表、选择、诊断和前端元数据链路。

### Negative

- WASM 解码比静态 Rust adapter 多一次线性内存复制与结构化输出解析。
- v1 只允许项目内置 Dashboard 组件；真正的第三方可执行 UI 需要单独的签名和权限设计。
- 原生游戏内桥接仍需按平台构建，不能由跨平台 WASM 替代。

### Neutral

- 现有 Rust adapter 保留为内置高性能实现，但其 descriptor 必须与 manifest 一致。
- 所有插件当前共用 Host 配置的 UDP ingress；需要固定专用端口的协议将在后续 ABI 版本增加多 ingress 声明。

## Alternatives Considered

**只把 adapter 拆成 Rust crate**

资源占用最低，但使用者必须重新编译 Host，不能独立安装或升级第三方支持。

**本机动态库**

调用开销低，但 Rust ABI 不稳定，C ABI 难以表达统一遥测，而且任意库崩溃或越权会直接影响 Host。

**独立插件进程与 IPC**

语言自由且隔离清晰，但每个进程带来额外常驻内存、启动管理和序列化开销，不符合当前桌面产品边界。

## References

- [ADR-0002](0002-adapter-api-and-canonical-telemetry.md)
- [ADR-0007](0007-versioned-local-game-input-bridges.md)
- [游戏插件系统设计](../plans/2026-08-15-game-plugin-system-design.md)
- [Wasmi resource limits](https://docs.rs/wasmi/latest/wasmi/struct.StoreLimits.html)
- [Wasmi fuel metering](https://docs.rs/wasmi/latest/wasmi/struct.Config.html#method.consume_fuel)
