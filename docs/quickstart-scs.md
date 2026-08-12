# OpenCarpanel ETS2 / ATS 快速开始

Euro Truck Simulator 2 与 American Truck Simulator 不会像 F1 一样直接向外发送仪表 UDP。OpenCarpanel 随包提供一个最小 SCS Telemetry SDK 插件：游戏调用插件，插件把当前帧的必要仪表数据以固定 44 字节报文发送到同机 `127.0.0.1:20777`，随后由 Rust Host 转发给手机/iPad。

## 1. 使用桌面安装向导（推荐）

完全退出 ETS2/ATS。在 OpenCarpanel“游戏设置”中选择对应游戏，点击“选择游戏目录”，再执行安装/更新。Rust 后端只会从用户选中的规范化游戏目录推导 64 位插件位置，拒绝符号链接逃逸；若已有不同版本，会保留最多三份备份并在原子写入后核对 SHA-256。

重新启动游戏，接受 SCS 的高级 SDK 功能提示并进入驾驶状态。控制中心的总览应自动显示 `ets2` 或 `ats`，手机页也会切换到对应卡车布局。

## 2. 手工获取插件（无头模式）

预览包中插件位于 `plugins/scs/`。从源码构建：

```powershell
npm run build:scs-plugin
```

产物位于 `target/scs-plugin-package/`。当前平台对应文件：

- Windows：`opencarpanel-scs-telemetry.dll`
- macOS：`opencarpanel-scs-telemetry.dylib`（SCS SDK 1.14 游戏插件 ABI 为 `x86_64`）
- Linux：`opencarpanel-scs-telemetry.so`

## 3. 手工安装到游戏

完全退出游戏，创建缺失的 `plugins` 目录，然后复制插件：

| 平台 | 目录 |
| --- | --- |
| Windows | `<游戏目录>/bin/win_x64/plugins/` |
| macOS | `<游戏>.app/Contents/MacOS/plugins/` |
| Linux | `<游戏目录>/bin/linux_x64/plugins/` |

ETS2 和 ATS 可以各放一份相同文件。不要把 DLL 放在游戏根目录或 `bin/win_x86`。

Apple Silicon 版 OpenCarpanel 桌面应用本身仍为原生 `arm64`；随包的 SCS bridge 则固定为 `x86_64`，因为它由游戏进程加载，而 SDK 1.14 头文件只定义了 x86/x64 ABI。桌面应用仅负责复制该文件，不会在自己的进程内加载它。Apple Silicon 上的实际游戏加载仍依赖游戏支持的 Intel/Rosetta 插件 ABI，并保留在发布清单中进行实机验收。

重新启动游戏时，SCS 会提示已启用高级 SDK 功能；接受提示并进入驾驶状态。插件文件名与 ETS2LA 的 `scs-telemetry` 不同，两者可以并存。

## 4. 启动与验证 Host

Host 使用默认 UDP 20777。通常保留自动识别即可；排障时固定游戏：

```powershell
$env:OPENCARPANEL_GAME = "ets2" # 美卡改为 ats
.\target\release\opencarpanel-host.exe
```

成功驾驶后，`/api/v1/diagnostics` 的 `activeAdapter` 应为 `ets2` 或 `ats`，对应 adapter 的 `packetsRecognized` 应持续增长。

## 5. 常见问题

| 现象 | 检查 |
| --- | --- |
| `packetsReceived` 始终为 0 | 文件是否在正确的 64 位 `plugins` 目录；是否重启游戏并接受 SDK 提示；Host 是否监听默认 20777 |
| 游戏没有 SDK 提示 | 插件路径/扩展名/CPU 架构不正确，或游戏尚未完全重启 |
| `packetsReceived` 增长但不识别 | 固定选择是否选错；是否安装了当前 OpenCarpanel 插件而非同名旧文件 |
| RPM 上限暂时为空 | 车辆配置事件尚未到达；进入车辆并开始驾驶后会更新 |
| 仪表显示 `DATA STALE` | 游戏暂停或进入菜单时插件有意停止发送；回到驾驶后自动恢复 |
| 插件初始化失败 | 在游戏的 `game.log.txt` 中搜索 `OpenCarpanel:`；插件会记录 socket 或 SDK callback 注册错误 |

插件不监听端口、不接受网络输入、不读取存档，只向 IPv4 loopback 发送；手机仍只连接 Host 的配对 HTTP/WebSocket。协议和安全边界见 [SCS bridge v1](protocols/scs-bridge-v1.md)。
