# OpenCarpanel 多游戏输入与适配器设计

## 状态

Implemented。F1 24/25 共用经过格式隔离测试的解析器；ETS2/ATS 使用项目自带的轻量 SCS SDK 插件桥接到同一个 Host UDP ingress。本文记录当时的首版 2025-only 边界；后续 2026 Season Pack 与按游戏前端设计见 [`2026-08-12-f1-25-2026-season-pack-design.md`](2026-08-12-f1-25-2026-season-pack-design.md)。真实游戏兼容性仍按发布检查清单单独验收。

## 1. 需求与边界

本阶段新增三个可实际使用的数据源：EA Sports F1 25、Euro Truck Simulator 2 和 American Truck Simulator，同时保留 F1 24。所有游戏继续输出到现有统一遥测模型和同一个手机 Dashboard，不为每款游戏复制前端。Host 默认自动识别来源，也允许通过启动配置锁定单款游戏，避免多款游戏同时运行时反复切换。

非功能约束沿用项目基线：游戏数据到浏览器 p95 小于 100 ms；输入队列有界且只保留最新连续状态；Host 不加载第三方动态库；插件不得阻塞或崩溃游戏主线程；遥测、配对信息和 IP 不离开本机；损坏或未知数据必须被拒绝而不是触发 panic。单一 Host 进程仍由 Rust 实现，SCS 游戏插件只承担官方 SDK 回调到版本化本机数据报的最小桥接职责。

F1 25 首版支持游戏设置中的原始 `2025` UDP 模式。EA 后续加入的 `2026 Season Pack` 是用户可选的另一套格式；在其完整包结构进入 fixture 和契约测试前，不把它静默当作 2025 解码。ETS2/ATS 首版映射当前 Dashboard 已能展示的速度、档位、RPM、最大 RPM、油门和刹车；仪表可以由 RPM/RPM 上限计算指针进度，但 adapter 不伪造游戏没有提供的转速灯。卡车专属导航、灯光、油量和任务字段留给后续统一 schema 扩展，不用不准确的临时字段代替。

## 2. 方案比较

### A. 项目自带 SCS SDK UDP 桥接插件（推荐）

插件由 ETS2/ATS 在游戏进程中加载，通过官方 Telemetry SDK 1.14 注册必要 channel，在 `FRAME_END` 将一个固定上限、版本化、小端数据报发送到 `127.0.0.1:20777`。Host 与 F1 共用一个 UDP listener，通过协议签名选择 adapter。发布包包含当前平台插件和安装说明。

优点是无外部运行依赖、协议和更新节奏由项目控制、延迟低、内存固定；缺点是需要维护一小段跨平台 C++、构建 DLL/dylib/so，并承担游戏内插件稳定性测试。

### B. 依赖第三方共享内存插件

Host 读取现有 `SCSTelemetry` memory mapping。ETS2LA 的实际方案是随包分发 `truckermudgeon/scs-sdk-plugin` 的 RenCloud fork：插件把 SCS callback 写入 32 KiB `Local\\SCSTelemetry`/POSIX shared memory，ETS2LA 约以 60 Hz 轮询解析。复用它能让已安装 ETS2LA 的用户少装一个插件，但默认依赖第三方布局、版本和交付会让故障边界与发布供应链受外部项目控制。后续可以把兼容该共享内存布局作为可选 ingress，而不替换项目自有的稳定桥接协议。

### C. 独立外部桥接进程

单独程序读取第三方插件或 SDK，再转发给 Host。隔离较强，但增加常驻进程、启动顺序、安装和排障成本，不符合当前模块化单体与低内存目标。

选择 A。插件只使用 SCS 官方 SDK 头文件及其宽松许可证，不引入第三方运行时实现。

## 3. 高层架构

```text
F1 24 UDP (format 2024) ─┐
F1 25 UDP (format 2025) ─┼─> UDP 20777 ─> AdapterRegistry ─> per-game reducer
                          │                       │                  │
ETS2/ATS ─> SCS SDK plugin┘                       │                  ├─> active snapshot
                                                  │                  └─> bounded events
                                                  └─> source selection/diagnostics
                                                                     │
                                                        paired HTTP + WebSocket
                                                                     │
                                                            phone/iPad Dashboard
```

`AdapterRegistry` 拥有四条独立 pipeline：`f1-24`、`f1-25`、`ets2`、`ats`。每条 pipeline 包含一个 `GameAdapter` 和一个以相同 adapter ID 初始化的 `TelemetryReducer`。数据报依次试探 adapter；只有某个 adapter 明确认领成功后才计为有效，全部拒绝才增加一次错误计数。这样某个协议的“不匹配”不会污染另一个协议的错误率。

自动模式采用来源粘性：首个产生有效状态的来源成为 active；只要它在超时窗口内持续更新，其他来源即使被解析也不抢占。active 来源超时后，下一个有效来源接管并立即发布自己的完整 reducer snapshot。显式模式只尝试用户选择的 pipeline。Dashboard 的 capabilities 发送所有内置 adapter 的并集，具体来源由每个 snapshot 的 `meta.gameId` 表达。

## 4. 协议与组件

### F1 共享解析器

把现有 `adapter-f1-24` 重构为 `adapter-f1`，保留一套 cursor、header、Car Telemetry packet 和字段映射。`F1_24Adapter` 与 `F1_25Adapter` 只提供 descriptor、期望 packet format、game year 和 adapter ID。官方 F1 24 v27.2x 与 F1 25 v3 的 Car Telemetry 均使用 29 字节 header、packet id 6、packet version 1、22 台车、1352 字节总长，核心字段偏移一致。每个格式仍有独立 fixture 和负向测试，避免“代码复用”等同于“未经验证的协议兼容”。

### SCS 桥接数据报

桥接协议使用固定长度而非 C/C++ struct 直接上网，插件逐字段写入小端字节，Host 逐字段读取。v1 包含：4 字节 magic、协议版本、游戏 ID、flags、保留字节、session nonce、frame sequence、speed m/s、RPM、最大 RPM、显示档位、有效油门和有效刹车。所有浮点值必须有限，归一化输入必须处于 0..1；长度、版本、游戏 ID、flags 和保留字段都严格校验。

ETS2 与 ATS 共享字节格式和解析实现，但分别暴露 `ets2`、`ats` descriptor。插件从 `scs_sdk_init_params` 的官方 `game_id` 决定数据报游戏 ID。它只创建非阻塞 loopback UDP socket，不监听端口、不接收网络输入、不分配无界内存。发送失败只静默丢弃当前帧，不在逐帧路径写日志、重试、阻塞或终止游戏；Host 通过来源未激活/数据 stale 来呈现链路故障。

## 5. 失败处理与可观测性

- 未知包、截断包、错误 format/version、NaN/Infinity 和越界值：拒绝并增加一次脱敏错误计数。
- F1 非 Car Telemetry 包：视为已识别但不产生 snapshot，不算错误。
- SCS 插件未安装或未被游戏接受：Host 保持运行，诊断显示所选/支持来源但无 active adapter。
- active 来源停止：浏览器沿用现有 stale 机制；超时后允许另一来源接管。
- 两个游戏同时发送：来源粘性防止 Dashboard 每帧切换；显式选择可完全禁止竞争。
- UDP 端口被占用：Host 启动错误继续包含端口和防火墙上下文。
- 插件 ABI 或 SDK 初始化版本不支持：插件返回 `SCS_RESULT_unsupported`，游戏日志给出版本，不访问未知结构。

诊断接口保留总接收、识别与错误指标，并增加 `adapterSelection`、`activeAdapter`、`supportedAdapters`，以及各 adapter 的 `packetsRecognized`/`lastPacketAgeMs`。无法归属来源的坏包只计入总错误，所有字段不得包含源 IP、玩家名、游戏安装路径或原始数据报。

## 6. 安全、许可与发布

SCS 插件只向硬编码的 IPv4 loopback 和固定端口发送，不从环境、网络或游戏字符串接受目标地址，因此不会意外广播遥测。Host 仍对 LAN Dashboard 使用既有一次性配对、设备 session、Origin/Host 校验和 CSP。真实捕获继续受 `.gitignore` 保护。

仓库 vendor SCS SDK 1.14 必需头文件和 `sdk_license.txt`，在 `NOTICE` 与发布包中保留 SCS Software 版权和许可。插件源代码使用项目 Apache-2.0；官方 SDK 头文件保持原许可。Windows、macOS 构建进入 package workflow；Linux 源码和构建定义保持可用，但当前发布矩阵不承诺 Linux Host artifact。

每个平台发布包在 `plugins/scs/` 放入本平台插件、SCS SDK 许可和安装说明，并在包根提供统一及分游戏快速开始。Windows 安装到 `<game>/bin/win_x64/plugins/`；macOS 使用游戏 app bundle 内官方插件目录。用户仍只启动一个 OpenCarpanel Host。

## 7. 验证与完成条件

1. F1 24 既有 fixture、Host UDP 集成和性能门禁不回归。
2. F1 25 format 2025 合成/官方结构 fixture 能输出正确速度、档位、RPM、转速灯、油门、刹车和 DRS；2024/2025 互相拒绝。
3. ETS2 与 ATS v1 数据报各自映射速度、档位、RPM、最大 RPM、油门和刹车，DRS 明确为 unavailable；错误 magic、游戏 ID、版本、长度和非有限值均拒绝。
4. 自动模式在 active 超时前不切换，超时后可从 F1 切至 SCS；显式模式拒绝其他游戏。
5. 插件在 Windows 与 macOS CI 编译成功，导出 SCS 要求的初始化/关闭符号，发布包包含插件、SDK 许可和安装说明。
6. Rust fmt、Clippy `-D warnings`、workspace tests、Web 类型/测试/构建、插件编译和 package smoke 全部通过。

## References

- [EA SPORTS F1 25 UDP specification](https://forums.ea.com/blog/f1-games-game-info-hub-en/ea-sports%E2%84%A2-f1%C2%AE25-udp-specification/12187347)
- [SCS Modding Wiki: Telemetry SDK](https://modding.scssoft.com/wiki/Documentation/Engine/SDK/Telemetry)
- [SCS Telemetry SDK 1.14](https://download.eurotrucksimulator2.com/scs_sdk_1_14.zip)
- [ETS2LA telemetry reader](https://github.com/ETS2LA/ETS2LA/blob/main/ETS2LA.Game/Telemetry/Program.cs)
- [ETS2LA bundled SDK sources](https://github.com/ETS2LA/ETS2LA/blob/main/Assets/SDKs/1.60/Windows/sources.txt)
- [truckermudgeon/scs-sdk-plugin](https://github.com/truckermudgeon/scs-sdk-plugin)
- [ADR-0001: Rust modular monolith](../adr/0001-rust-modular-monolith.md)
- [ADR-0002: Adapter API and canonical telemetry](../adr/0002-adapter-api-and-canonical-telemetry.md)
