# OpenCarpanel ETS2 / ATS 快速开始

Euro Truck Simulator 2 与 American Truck Simulator 不会像 F1 一样直接向外发送仪表 UDP。OpenCarpanel 随包提供一个最小 SCS Telemetry SDK 插件：游戏调用插件，插件把当前帧的必要仪表数据以固定 44 字节报文发送到同机 `127.0.0.1:20777`，随后由 Rust Host 转发给手机/iPad。

## 1. 获取插件

预览包中插件位于 `plugins/scs/`。从源码构建：

```powershell
npm run build:scs-plugin
```

产物位于 `target/scs-plugin-package/`。当前平台对应文件：

- Windows：`opencarpanel-scs-telemetry.dll`
- macOS：`opencarpanel-scs-telemetry.dylib`
- Linux：`opencarpanel-scs-telemetry.so`

## 2. 安装到游戏

完全退出游戏，创建缺失的 `plugins` 目录，然后复制插件：

| 平台 | 目录 |
| --- | --- |
| Windows | `<游戏目录>/bin/win_x64/plugins/` |
| macOS | `<游戏>.app/Contents/MacOS/plugins/` |
| Linux | `<游戏目录>/bin/linux_x64/plugins/` |

ETS2 和 ATS 可以各放一份相同文件。不要把 DLL 放在游戏根目录或 `bin/win_x86`。

重新启动游戏时，SCS 会提示已启用高级 SDK 功能；接受提示并进入驾驶状态。插件文件名与 ETS2LA 的 `scs-telemetry` 不同，两者可以并存。

## 3. 启动与验证 Host

Host 使用默认 UDP 20777。通常保留自动识别即可；排障时固定游戏：

```powershell
$env:OPENCARPANEL_GAME = "ets2" # 美卡改为 ats
.\target\release\opencarpanel-host.exe
```

成功驾驶后，`/api/v1/diagnostics` 的 `activeAdapter` 应为 `ets2` 或 `ats`，对应 adapter 的 `packetsRecognized` 应持续增长。

## 4. 常见问题

| 现象 | 检查 |
| --- | --- |
| `packetsReceived` 始终为 0 | 文件是否在正确的 64 位 `plugins` 目录；是否重启游戏并接受 SDK 提示；Host 是否监听默认 20777 |
| 游戏没有 SDK 提示 | 插件路径/扩展名/CPU 架构不正确，或游戏尚未完全重启 |
| `packetsReceived` 增长但不识别 | 固定选择是否选错；是否安装了当前 OpenCarpanel 插件而非同名旧文件 |
| RPM 上限暂时为空 | 车辆配置事件尚未到达；进入车辆并开始驾驶后会更新 |
| 仪表显示 `DATA STALE` | 游戏暂停或进入菜单时插件有意停止发送；回到驾驶后自动恢复 |
| 插件初始化失败 | 在游戏的 `game.log.txt` 中搜索 `OpenCarpanel:`；插件会记录 socket 或 SDK callback 注册错误 |

插件不监听端口、不接受网络输入、不读取存档，只向 IPv4 loopback 发送；手机仍只连接 Host 的配对 HTTP/WebSocket。协议和安全边界见 [SCS bridge v1](protocols/scs-bridge-v1.md)。
