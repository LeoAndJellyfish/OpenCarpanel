# OpenSimDash F1 24 快速开始

当前 F1 24 adapter 支持驾驶核心字段，以及圈速/比赛状态、天气、轮胎、损伤、燃油/ERS、处罚和离散赛事事件。精确 packet 范围与未实现边界见本文链接的协议说明。

## 1. 安装桌面控制中心

从 [Releases](https://github.com/LeoAndJellyfish/OpenSimDash/releases/latest) 下载 Windows x64 安装器，或与你 Mac 架构匹配的 DMG。源码构建需要 Node.js 22+ 与仓库指定的 Rust：

```powershell
git clone <你的 OpenSimDash 仓库地址>
cd OpenSimDash
npm ci
npm run build:desktop
```

也可以生成包含许可证和本指南的本机预览目录：

```powershell
npm run package:host
```

输出位于 `dist/release/OpenSimDash-<version>-<platform>-<arch>/`。当前产物尚未进行 Windows 代码签名或 macOS notarization，只用于开发和预览。

## 2. 启动与配对

通常直接打开 **OpenSimDash**，在“设备与配对”生成 10 分钟内一次有效的二维码。手机或 iPad 与电脑连接同一局域网后扫描；配对成功的设备会被记住并可随时撤销。

无头模式仍可使用安装目录中的 `opensimdash-host`，或源码产物：

Windows：

```powershell
.\target\release\opensimdash-host.exe
```

macOS：

```bash
./target/release/opensimdash-host
```

Host 默认监听：

- 游戏遥测 UDP：`0.0.0.0:20777`
- Dashboard HTTP/WebSocket：`0.0.0.0:20778`

无头 Host 会在终端打印一个 15 分钟内、仅可使用一次的配对地址和二维码。

配对成功后，驾驶页位于 `/`，布局编辑器位于 `/edit`。编辑器支持响应式断点、拖动/缩放、撤销/重做、主题、Host 持久化以及安全的 JSON 导入导出。

## 3. 配置 F1 24

在 F1 24 的 `Settings → Telemetry Settings` 中使用以下设置：

| 设置 | 游戏和 Host 在同一电脑 | 游戏在另一台电脑/主机 |
| --- | --- | --- |
| UDP Telemetry | On | On |
| UDP Broadcast Mode | Off | Off |
| UDP IP Address | `127.0.0.1` | Host 配对地址中显示的局域网 IP |
| UDP Port | `20777` | `20777` |
| UDP Send Rate | `60Hz` | `60Hz` |
| UDP Format | `2024` | `2024` |

例如配对地址是 `http://192.168.31.155:20778/#pair=…` 时，另一台游戏设备的 UDP IP Address 填 `192.168.31.155`，不要包含 `http://`、端口或路径。

实现依据和 EA 官方规格入口见 [F1 24 协议说明](protocols/f1-24.md)。

## 4. 防火墙与网络

- Windows 首次提示时，仅允许 OpenSimDash 通过受信任的“专用网络”；需要入站 TCP `20778` 和 UDP `20777`。
- macOS 若询问是否允许接收入站连接，请允许当前 Host。
- 手机与 Host 必须处在可互访的局域网。访客 Wi-Fi、AP/客户端隔离和部分企业网络会阻止设备互访。
- 配对页面当前使用局域网 HTTP。不要在不受信任的公共 Wi-Fi 上使用。
- Host 不需要云端账户，不上传遥测或布局；所有运行数据保存在本机。

## 5. 自检与故障排查

在 Host 电脑上访问：

- 健康检查：`http://127.0.0.1:20778/api/v1/health`
- 脱敏诊断：`http://127.0.0.1:20778/api/v1/diagnostics`

常见情况：

| 现象 | 检查 |
| --- | --- |
| 手机打不开页面 | 确认同一 Wi-Fi、配对 IP 属于真实网卡、TCP 20778 防火墙允许、路由器未启用客户端隔离 |
| 页面显示 `DATA STALE` | 确认 F1 24 正在赛道会话中、UDP Telemetry 为 On、IP/端口/Format 正确 |
| `packetsReceived` 增长但 `packetsRecognized` 为 0 | 确认 UDP Format 为 `2024`；若设置了 `OPENSIMDASH_GAME`，确认值为 `f1-24` |
| 配对令牌无效 | 令牌已使用或过期；在控制中心重新生成（无头模式重启 Host） |
| UDP 端口占用 | 关闭占用 20777 的遥测工具，或在控制中心“网络”页换用双方一致的新端口 |
| 布局保存冲突 | 编辑器会显示 revision 冲突；选择加载 Host 版本或明确覆盖，不会静默丢失修改 |

布局默认保存在：

- Windows：`%LOCALAPPDATA%\OpenSimDash`
- macOS：`~/Library/Application Support/OpenSimDash`

Host 使用原子写入和最近有效备份。不要把配对地址、设备 session、真实 UDP capture 或上述数据目录提交到公开 issue。

## 6. 退出

关闭桌面窗口默认只缩到托盘，Host 会继续接收遥测；从托盘菜单选择“退出 OpenSimDash”才会优雅关闭 HTTP/UDP。无头模式在终端按 `Ctrl+C`。已经配对的设备会持久化，除非用户主动撤销。
