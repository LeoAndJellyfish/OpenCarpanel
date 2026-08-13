# OpenCarpanel F1 25 快速开始

当前 F1 25 adapter 同时支持原始 `2025` UDP 与 2026 Season Pack 的 `2026` UDP，显示玩家车辆的速度、档位、RPM、转速灯、油门、刹车和 DRS。圈速、比赛状态、轮胎、损伤与离散赛事事件尚未实现。

> **版本提示：** EA 说明新用户会默认使用 2026 Season Pack UDP；`v0.2.1` 可直接识别，不需要切回旧模式。历史 `v0.1.0` 下载包仍只支持原始 2025 UDP。

EA 当前说明与两套规格入口：[F1 25 / 2026 Season Pack UDP Specification](https://forums.ea.com/blog/f1-games-game-info-hub-en/ea-sports%E2%84%A2-f1%C2%AE25-2026-season-pack-udp-specification/12187347)。

## 1. 启动控制中心

按[多游戏快速开始](quickstart-multi-game.md)安装并启动。默认 `auto` 即可；排障时可在“网络”页固定为 F1 25。无头模式也可使用：

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
| UDP Mode / Format | **F1 25 / 2025** 或 **2026 Season Pack**，均可 |

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

数值会持续增长，示例只表达字段关系。若收到包但没有识别，确认使用的是 `v0.2.1`，再检查端口为 20777、packet format 为 2025 或 2026。识别后手机页会自动切换到 F1 25 青色方程式布局，并加载 `game-f1-25` 的独立用户配置。协议实现边界见 [F1 25 协议说明](protocols/f1-25.md)。
