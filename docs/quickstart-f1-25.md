# OpenCarpanel F1 25 快速开始

首版 F1 25 adapter 支持原始 `2025` UDP 格式的玩家车辆速度、档位、RPM、转速灯、油门、刹车和 DRS。圈速、比赛状态、轮胎、损伤与离散赛事事件尚未实现。

> **重要：** F1 25 的 2026 Season Pack 提供另一套 UDP 格式。EA 说明从未启动过 F1 25 的新用户会默认使用 2026 Season Pack UDP；OpenCarpanel 当前必须手动选择原始 **F1 25 / 2025** UDP mode。两种格式不会被模糊兼容。

EA 当前说明与两套规格入口：[F1 25 / 2026 Season Pack UDP Specification](https://forums.ea.com/blog/f1-games-game-info-hub-en/ea-sports%E2%84%A2-f1%C2%AE25-udp-specification/12187347)。

## 1. 启动 Host

按[多游戏快速开始](quickstart-multi-game.md)构建并启动。默认 `auto` 即可；排障时可固定为 F1 25：

```powershell
$env:OPENCARPANEL_GAME = "f1-25"
.\target\release\opencarpanel-host.exe
```

## 2. 配置 F1 25

在游戏的 Telemetry Settings 中设置：

| 设置 | 值 |
| --- | --- |
| UDP Telemetry | On |
| UDP Broadcast Mode | Off |
| UDP IP Address | 游戏与 Host 同机时 `127.0.0.1`；异机时填 Host 局域网 IP |
| UDP Port | `20777` |
| UDP Send Rate | `60Hz` |
| UDP Mode / Format | **F1 25 / 2025**，不是 2026 Season Pack |

进入实际驾驶会话后再观察诊断；部分菜单状态不会产生 Car Telemetry 快照。

## 3. 验证

访问 `http://127.0.0.1:20778/api/v1/diagnostics`。成功时应看到：

```json
{
  "adapterSelection": "auto",
  "activeAdapter": "f1-25",
  "telemetry": {
    "packetsReceived": 1,
    "packetsRecognized": 1,
    "packetErrors": 0
  }
}
```

数值会持续增长，示例只表达字段关系。若收到包但没有识别，首先重新确认 UDP mode 不是 2026 Season Pack、端口为 20777。协议实现边界见 [F1 25 协议说明](protocols/f1-25.md)。
