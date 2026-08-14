# OpenCarpanel SCS bridge protocol v1

逐字节阵列图、四款游戏完整链路和 ETS2LA 方案对照见[游戏数据链路与 SCS 数据包协议图解](../data-paths-and-scs-packet.md)。

v1 是仍受 Host 支持的 44-byte 兼容输入。自扩展导航、灯光、油量和任务字段起，随包插件默认发送 [v2](scs-bridge-v2.md)；旧插件无需立即更换，现有基础仪表字段仍会正常解码。

## 数据路径

```text
ETS2 / ATS SCS Telemetry SDK callbacks
  → opencarpanel-scs-telemetry native plugin
  → non-blocking UDP 127.0.0.1:20777
  → adapter-scs defensive decoder
  → canonical telemetry snapshot
```

插件使用官方 [SCS Telemetry SDK](https://modding.scssoft.com/wiki/Documentation/Engine/SDK/Telemetry) 1.14。仓库中仅保留构建所需的未修改头文件、来源哈希与 SCS 许可。

## 固定字节布局

v1 长度必须恰好为 44 bytes。所有整数与 IEEE-754 `float32` 使用小端：

| Offset | Type | Field | Validation |
| ---: | --- | --- | --- |
| 0 | `[u8;4]` | magic `OCP\0` | exact |
| 4 | `u8` | version | `1` |
| 5 | `u8` | game | `1` ETS2；`2` ATS |
| 6 | `u8` | flags | `0` |
| 7 | `u8` | reserved | `0` |
| 8 | `u64` | session nonce | opaque |
| 16 | `u32` | frame sequence | wrapping monotonic counter |
| 20 | `f32` | signed speed m/s | finite；负值代表倒车，仪表取幅值 |
| 24 | `f32` | engine RPM | finite，0..65535 |
| 28 | `f32` | configured RPM limit | finite，0 表示未知 |
| 32 | `i32` | displayed gear | `<0` R，`0` N，`1..255` forward，其余 unknown |
| 36 | `f32` | effective throttle | finite，0..1 |
| 40 | `f32` | effective brake | finite，0..1 |

对应 SDK 来源为 `truck.speed`、`truck.engine.rpm`、`truck.displayed.gear`、`truck.effective.throttle`、`truck.effective.brake` 和 truck configuration 的 `rpm.limit`。

## 运行时语义

- 插件只在未暂停的 `FRAME_END` 发送一份当前状态；暂停/菜单期间 Host 的 stale 机制自然生效。
- SDK channel 只在变化时回调，插件把值保存在固定全局状态中；frame callback 不分配、不加锁、不做文件 I/O 或 DNS。
- UDP send 为非阻塞且无重试；失败只丢弃当前帧，不阻塞游戏线程。
- session nonce 在每次插件初始化时变化；frame sequence 在该初始化周期内递增。
- ETS2/ATS 分别映射到 `ets2`/`ats` reducer，DRS 明确为 unavailable；卡车仪表的转速进度由 RPM/RPM max 计算，不伪造 F1 rev-lights 数据。

## 安全与版本升级

目的地址和端口硬编码为 IPv4 loopback `127.0.0.1:20777`。插件没有监听 socket、LAN 地址、配置文件或远程输入。Host 必须拒绝错误 magic、长度、版本、game、flags、reserved、NaN/Infinity 和越界比例。

v1 的长度和偏移永远不变。新字段使用新的 version 与精确长度；Host 依据 version 选择布局，并对其他版本明确报告 unsupported version。
