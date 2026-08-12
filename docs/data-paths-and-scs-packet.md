# 游戏数据链路与 SCS 44 字节协议图解

本文用两张可缩放的教材式图示说明 OpenCarpanel `0.1.x` 支持的四款游戏如何把遥测送到手机，以及 ETS2/ATS 原生桥接插件发出的 44 字节数据包如何排列。图中所有连接都是本机或同一局域网连接；运行时不经过云端服务。

## 四款游戏如何进入同一个 Dashboard

<p align="center">
  <img src="./assets/supported-game-data-paths.svg" width="100%" alt="F1 24、F1 25、ETS2 与 ATS 从游戏进程经过本地 Host 到手机或 iPad 的完整数据链路">
</p>

链路分为两种：

1. **F1 24 / F1 25：游戏原生 UDP。** 游戏按官方格式直接把数据报发到 Host 的 UDP `20777`。F1 24 adapter 只接受 format `2024`；F1 25 adapter 依据公共头精确选择原始 `2025` 或 Season Pack `2026` 布局，不按相近偏移猜测。
2. **ETS2 / ATS：SCS SDK 回调桥接。** 游戏加载随包插件，在未暂停帧的 `FRAME_END` 回调中，把必要字段编码为固定 44 字节报文，并非阻塞地发到同机 `127.0.0.1:20777`。

四条输入在 `AdapterRegistry` 中都有独立 adapter/reducer。自动模式对当前来源保持两秒粘性，防止同时运行多款游戏时画面来回切换；也可以用 `OPENCARPANEL_GAME=f1-24|f1-25|ets2|ats` 固定来源。随后 Host 把统一遥测通过已配对的 HTTP/WebSocket 送到手机或 iPad；Dashboard 使用统一模型中的 `meta.gameId` 自动选择游戏视觉与独立用户布局，不从某个速度/RPM 字段反向猜游戏。

## 44 字节数据包：像数组一样逐段看

<p align="center">
  <img src="./assets/scs-bridge-v1-packet.svg" width="100%" alt="OpenCarpanel SCS bridge v1 固定 44 字节数据包的逐字节数组与字段说明">
</p>

最上方的 `00..43` 是从零开始的 byte offset。彩色长条仍是一段连续数组；竖线表示每个单独字节，颜色和标签表示字段边界。所有整数和 IEEE-754 `float32` 均为小端序。

| Offset | 长度 | Wire type | 字段 | 语义与校验 |
| ---: | ---: | --- | --- | --- |
| `0` | 4 | `[u8; 4]` | magic | 固定为 ASCII `OCP\0` |
| `4` | 1 | `u8` | version | 固定为 `1` |
| `5` | 1 | `u8` | game | `1 = ETS2`，`2 = ATS` |
| `6` | 1 | `u8` | flags | v1 必须为 `0` |
| `7` | 1 | `u8` | reserved | v1 必须为 `0` |
| `8` | 8 | `u64` | session nonce | 每次插件初始化生成；Host 只把它视为不透明会话标识 |
| `16` | 4 | `u32` | frame sequence | 插件初始化周期内递增，允许自然回绕 |
| `20` | 4 | `f32` | signed speed | 米/秒；负数代表倒车，速度表显示其幅值 |
| `24` | 4 | `f32` | engine RPM | 必须有限且处于 `0..65535` |
| `28` | 4 | `f32` | RPM limit | 车辆配置的转速上限；`0` 表示暂未知 |
| `32` | 4 | `i32` | displayed gear | `<0 = R`，`0 = N`，`1..255 = 前进档` |
| `36` | 4 | `f32` | effective throttle | 必须有限且处于 `0..1` |
| `40` | 4 | `f32` | effective brake | 必须有限且处于 `0..1` |

选择固定长度而不是直接发送 C++ struct，能避开编译器 padding、alignment 与 ABI 差异。Rust Host 会逐字段读取，并严格拒绝错误 magic、长度、版本、游戏 ID、保留位、`NaN`/`Infinity` 和越界踏板值；原始数据报不会被保留。

## 为什么没有直接复用 ETS2LA 的共享内存

ETS2LA 随包使用 `truckermudgeon/scs-sdk-plugin` 的 RenCloud fork，把 SCS callback 写入 32 KiB `SCSTelemetry` shared memory，并由主程序约以 60 Hz 读取。这是成熟且字段丰富的方案，但默认依赖第三方布局会把版本和故障边界交给外部项目。

OpenCarpanel 首版选择自有、最小、版本化的 loopback UDP bridge：游戏进程内只保留固定状态和非阻塞发送，解析、诊断与网络服务都留在 Rust Host。未来可以把 ETS2LA 共享内存兼容作为可选 ingress，让已安装该插件的用户避免重复安装，而不改变当前协议。

## 延伸阅读

- [SCS bridge v1 完整协议边界](protocols/scs-bridge-v1.md)
- [多游戏输入与适配器设计](plans/2026-08-12-multi-game-adapters-design.md)
- [ADR-0007：版本化本机游戏桥接](adr/0007-versioned-local-game-input-bridges.md)
- [SCS Telemetry SDK 官方文档](https://modding.scssoft.com/wiki/Documentation/Engine/SDK/Telemetry)
- [ETS2LA telemetry reader](https://github.com/ETS2LA/ETS2LA/blob/main/ETS2LA.Game/Telemetry/Program.cs)
- [truckermudgeon/scs-sdk-plugin](https://github.com/truckermudgeon/scs-sdk-plugin)
