# OpenSimDash SCS bridge protocol v2

v2 是随包 SCS SDK 1.14 插件当前发送的 188-byte 本机协议，在 v1 的前 44 bytes 后追加导航、油量、灯光和配送任务。Host 同时接受 [v1](scs-bridge-v1.md) 与 v2，不会按长度猜测版本。

## 数据路径

```text
ETS2 / ATS SCS Telemetry SDK callbacks
  → opensimdash-scs-telemetry native plugin
  → fixed 188-byte UDP 127.0.0.1:20777
  → adapter-scs v2 defensive decoder
  → canonical vehicle / navigation / lights / job state
```

## 固定字节布局

所有整数与 IEEE-754 `float32` 使用小端。offset `0..43` 与 v1 相同，只有 version 改为 `2`：

| Offset | Bytes | Type | Field | Validation / semantics |
| ---: | ---: | --- | --- | --- |
| 0 | 4 | `[u8;4]` | magic `OSD\0` | exact |
| 4 | 1 | `u8` | version | `2` |
| 5 | 1 | `u8` | game | `1` ETS2；`2` ATS |
| 6 | 2 | `u8 + u8` | flags / reserved | 均为 `0` |
| 8 | 8 | `u64` | session nonce | opaque |
| 16 | 4 | `u32` | frame sequence | wrapping monotonic counter |
| 20 | 24 | v1 vehicle fields | speed、RPM、RPM max、gear、throttle、brake | 与 v1 完全相同 |
| 44 | 4 | `f32` | navigation distance | metres，finite，≥ 0 |
| 48 | 4 | `f32` | navigation time | seconds，finite，≥ 0 |
| 52 | 4 | `f32` | navigation speed limit | m/s，finite；`≤ 0` 是 SDK 特殊状态，不展示为限速 |
| 56 | 4 | `f32` | fuel | litres，finite，≥ 0 |
| 60 | 4 | `f32` | fuel capacity | litres，finite，≥ 0 |
| 64 | 4 | `f32` | fuel range | km，finite，≥ 0 |
| 68 | 2 | `u16` | light bits | 只允许下列 9 bits |
| 70 | 2 | `u16` | state bits | 只允许下列 4 bits |
| 72 | 4 | `u32` | delivery time | SCS 绝对游戏时间 |
| 76 | 4 | `u32` | planned distance | simulated km |
| 80 | 8 | `u64` | income | 游戏原生货币 |
| 88 | 4 | `f32` | cargo mass | kg，finite，≥ 0 |
| 92 | 32 | `[u8;32]` | cargo | UTF-8，NUL padding |
| 124 | 32 | `[u8;32]` | source city | UTF-8，NUL padding |
| 156 | 32 | `[u8;32]` | destination city | UTF-8，NUL padding |

固定文本由插件在 UTF-8 code-point 边界截断，最长 31 bytes，并以 NUL/零填充。Host 拒绝缺少 NUL、非零 padding、无效 UTF-8、未知 bit、错误长度和无效数值；不保留原始数据报。

### Light bits（offset 68）

| Bit | 状态 |
| ---: | --- |
| 0 | parking |
| 1 | low beam |
| 2 | high beam |
| 3 | beacon |
| 4 | brake |
| 5 | reverse |
| 6 | logical left indicator |
| 7 | logical right indicator |
| 8 | hazard warning |

### State bits（offset 70）

| Bit | 状态 |
| ---: | --- |
| 0 | fuel warning |
| 1 | job active |
| 2 | cargo loaded |
| 3 | special transport job |

## SDK 来源与运行时语义

- 导航来自 `truck.navigation.distance/time/speed.limit`；油量来自 `truck.fuel.amount/range/warning` 与 truck configuration 的 `fuel.capacity`。
- [SCS 官方工程师说明](https://forum.scssoft.com/viewtopic.php?t=186527)：traffic subsystem 会让限速 channel 用非正数表达特殊情况；adapter 保留协议有效性，但把 `≤ 0` 映射为统一模型中的 absent，避免显示负限速。
- 灯光使用官方 physical/logical light channels；任务文字、质量、收入、期限和里程来自 job configuration attributes。
- SDK channel 只在值变化时回调；插件把最新值保存在固定状态中，在未暂停的 `FRAME_END` 编码并非阻塞发送一份快照。
- 帧路径不分配、不加锁、不做文件 I/O、DNS 或重试。目的地址固定为 `127.0.0.1:20777`，插件不监听任何端口。
- 当 `job active` 变为 false，reducer 会清除上一任务的货物、城市、金额和时限，避免旧任务残留在 Dashboard。

教材式分段图见[游戏数据链路与 SCS 数据包协议图解](../data-paths-and-scs-packet.md)。
