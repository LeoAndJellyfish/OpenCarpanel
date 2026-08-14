# OpenCarpanel 多游戏快速开始

OpenCarpanel 桌面控制中心内嵌的 Host 默认在 UDP `20777` 上自动识别四个输入源，并把当前 active 游戏发布给同一套手机/iPad Dashboard。安装目录仍附带独立无头 Host，但不能与 GUI 同时运行。

| 游戏 | 游戏到 Host 的入口 | 首版要求 |
| --- | --- | --- |
| F1 24 | 游戏原生 UDP | Format `2024` |
| F1 25 | 游戏原生 UDP | 原始 **2025** 与 **2026 Season Pack** UDP 模式均可 |
| Euro Truck Simulator 2 | 随包原生 SCS 插件 → loopback UDP | 安装 v2 插件；支持导航、油量、灯光与任务 |
| American Truck Simulator | 随包原生 SCS 插件 → loopback UDP | 安装同一插件；Host 仍兼容旧 v1 插件 |

各游戏详细步骤：

- [F1 24 快速开始](quickstart-f1-24.md)
- [F1 25 快速开始](quickstart-f1-25.md)
- [ETS2 / ATS 快速开始](quickstart-scs.md)

## 安装与启动

从 [GitHub Releases](https://github.com/LeoAndJellyfish/OpenCarpanel/releases/latest) 下载 Windows x64 安装器、Apple Silicon DMG 或 Intel DMG。启动桌面控制中心后：

1. 在“设备与配对”生成二维码并用同一局域网中的手机/iPad 扫描。
2. 在“游戏设置”按向导配置 F1；ETS2/ATS 会先自动查找 Steam 安装目录，未找到时再手动选择目录并安装 bridge。
3. 回到“总览”观察 `GAME → UDP → HOST → MOBILE` 链路和包计数。

从源码构建 GUI：

```powershell
npm ci
npm run build:desktop
```

### 无头模式（高级）

只运行 Host 需要 Node.js 22+ 和仓库指定的 Rust 工具链：

```powershell
npm ci
npm run build:host
.\target\release\opencarpanel-host.exe
```

macOS 最后一行使用：

```bash
./target/release/opencarpanel-host
```

构建 SCS 插件或完整预览包还需要 CMake 3.20+ 和 64 位 C++17 编译器：

```powershell
npm run build:scs-plugin
npm run package:host
```

## 自动识别与固定选择

默认 `auto` 模式会识别四种协议。当前 active 来源持续发包时，Host 会保持两秒来源粘性，避免两个游戏同时运行时仪表盘来回切换。

每个有效 snapshot 都携带稳定的 `meta.gameId`。驾驶页只在这个低频标识变化时切换页面配置：F1 24/25 使用方程式布局与 DRS 状态，ETS2/ATS 使用速度优先的卡车布局与 SCS bridge 状态。四款游戏分别保存为 `game-f1-24`、`game-f1-25`、`game-ets2`、`game-ats`，所以编辑某款游戏不会覆盖另一款；`/edit` 也可以手动选择要预览和编辑的游戏。

排障或只允许一个游戏时，在控制中心“网络”页选择目标 adapter。无头自动化也可在启动前设置：

```powershell
$env:OPENCARPANEL_GAME = "f1-25" # auto | f1-24 | f1-25 | ets2 | ats
.\target\release\opencarpanel-host.exe
```

macOS/Linux：

```bash
OPENCARPANEL_GAME=ets2 ./target/release/opencarpanel-host
```

值严格区分大小写；非法值会让 Host 以可操作错误退出，不会静默回退。

## 判断问题在哪一段

在控制中心“系统与诊断”查看摘要或打开完整 JSON；默认地址是 `http://127.0.0.1:20778/api/v1/diagnostics`：

- `telemetry.packetsReceived = 0`：Host 没收到任何 UDP。检查游戏 UDP 设置或 SCS 插件是否加载。
- `packetsReceived > 0` 但 `packetsRecognized = 0`：端口通，但 format/mode/插件协议不匹配。
- `activeAdapter` 是目标游戏：游戏读取与 Host 适配已经成功；若手机仍无数据，继续检查配对、WebSocket 和 Wi-Fi。
- 对应 `supportedAdapters[].packetsRecognized` 持续增长：该 adapter 正在稳定接收。
- `packetErrors` 增长：收到的包被所有启用 adapter 拒绝；固定选择模式下也可能是选错游戏。

诊断不包含源 IP、玩家名、配对令牌、设备 session、安装路径或原始数据报。
