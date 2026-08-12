# F1 25 2026 Season Pack UDP 兼容设计

## 目标与边界

OpenCarpanel 的现有 `f1-25` adapter 只接受 F1 25 原始 UDP format `2025`。本次让同一个稳定 adapter 同时接受 EA 官方提供的两种 F1 25 UDP mode：原始 format `2025` 和 2026 Season Pack format `2026`。用户继续使用 `OPENCARPANEL_GAME=f1-25`，自动模式、诊断 API、Dashboard 的 `gameId` 也继续显示 `f1-25`；切换游戏内 UDP mode 不产生新的逻辑游戏来源。

本阶段只扩展 Dashboard 已消费的 Car Telemetry packet（packet id `6`、version `1`）：速度、油门、刹车、档位、RPM、DRS 和转速灯。2026 新增的 Car Telemetry 2 packet（id `16`）以及主动空气动力学等新字段暂不映射，避免在统一模型中加入未经产品设计的字段。

协议依据是 [EA SPORTS F1 25: 2026 Season Pack UDP Specification](https://forums.ea.com/blog/f1-games-game-info-hub-en/ea-sports%E2%84%A2-f1%C2%AE25-2026-season-pack-udp-specification/12187347) Version 10.0。官方结构声明全部字段均为 packed、little-endian，并把 2026 Car Telemetry 定义为 24 个、每个 59 字节的数据项，总包长 1448 字节。

## 架构与数据流

`F1Protocol` 继续描述逻辑 adapter，但不再只保存一个 packet format，而是引用一个或多个不可变 `CarTelemetryLayout`。每个布局显式包含 `packet_format`、车辆数、单车结构长度和总包长度：

| UDP mode | format | 车辆数 | 单车长度 | 总包长度 |
| --- | ---: | ---: | ---: | ---: |
| F1 24 | 2024 | 22 | 60 | 1352 |
| F1 25 original | 2025 | 22 | 60 | 1352 |
| F1 25 2026 Season Pack | 2026 | 24 | 59 | 1448 |

decoder 先安全读取 29 字节公共头，再使用 `packetFormat` 在当前 adapter 允许的布局中精确匹配。找不到布局时返回 `UnsupportedPacketFormat`；找到后才检查 packet id、packet version、玩家索引和该布局的精确总长度。玩家车辆偏移由 `29 + player_index * car_telemetry_data_len` 得出。当前映射字段位于每车结构前 20 字节，其偏移在 2025/2026 中一致；剩余字节按选中的结构长度跳过，不依赖相邻版本猜测。

## 错误处理与兼容性

F1 24 adapter 仍只接受 format `2024`。F1 25 adapter 只接受 `2025` 或 `2026`，其他年份不会因长度相似而被识别。2026 包必须是 1448 字节，玩家索引必须小于 24；2025 包仍必须是 1352 字节且索引小于 22。格式与布局交叉组合（例如 format 2026 加 1352 字节）必须返回明确的长度错误。

`f1-25` adapter ID、用户配置值和 canonical telemetry schema 均不变，因此手机配对、WebSocket、布局持久化和现有 Widget 无需迁移。诊断中的 protocol version 更新为同时列出 `2025/v3` 与 `2026/v10`，用于确认二进制包含新协议。

## 测试与验收

单元测试使用按官方 packed 结构生成的合成数据包，覆盖 2026 format、1448 字节、24 车数组，以及玩家位于索引 23 的边界。负向测试覆盖 format 交叉拒绝、错误总长度、索引 24、错误 packet version、非法 DRS 和非有限/越界踏板值。Host UDP 集成测试分别发送 format 2025 与 2026，并确认二者都激活 `f1-25`；固定 `f1-25` 模式同样接受两者。包内 smoke 增加 2026 报文，但 adapter 列表仍保持四个逻辑游戏。

自动化只能证明结构和数据流符合规范，不能代替真实游戏验收。发布检查清单会新增未勾选项：在 F1 25 2026 Season Pack 实机、UDP format `2026`、玩家索引变化和 60 Hz 情况下验证连续显示。

## 按游戏自动切换前端

Dashboard 从已有 canonical snapshot 的 `meta.gameId` 判断当前逻辑游戏，不读取原始协议，也不根据字段是否为空猜测。`f1-24`、`f1-25`、`ets2`、`ats` 各自映射到稳定的 presentation profile；每个 profile 包含展示名称、游戏家族、持久化 layout ID 和安全内置布局。F1 25 的 2025/2026 UDP mode 共用 `f1-25` profile，因为两者属于同一游戏，切换 UDP mode 不应重置用户布局。

选择方案为“每游戏独立布局”，而不是只切 CSS 或创建四套页面代码。F1 页面保留转速灯地平线、大档位和 DRS 状态；卡车页面把速度置于视觉中心、缩小档位、使用较低备用红线，并把不存在的 DRS 行改为 SCS 数据源状态。F1 24、F1 25、ETS2、ATS 默认主题分别提供可辨识的信号色，但用户仍可在编辑器中覆盖主题与组件位置。

Host 为 `game-f1-24`、`game-f1-25`、`game-ets2`、`game-ats` 提供独立的版本化布局文档；旧的 `default` 保留以兼容已有数据。驾驶页只在 `gameId` 实际变化时触发 Preact 状态更新和布局读取，高频速度/RPM 仍走既有单一 `requestAnimationFrame` DOM 更新路径。来源切换时先立即显示对应内置布局，再异步加载该游戏的用户布局，失败则保留安全默认值。编辑器允许明确选择要编辑的游戏，不会把一种游戏的自定义布局覆盖到另一种游戏。
