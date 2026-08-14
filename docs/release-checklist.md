# OpenCarpanel v0.2.2 发布检查清单

本清单区分“自动化或合成数据已验证”和“真实游戏/设备已验证”。没有实测的项目保持未勾选，不能由 fixture、截图或 CI 代替。

## 自动化门禁

- [x] Windows：Rust fmt、workspace Clippy `-D warnings`、workspace tests（2026-08-12）
- [x] Windows：桌面 Preact typecheck、3 个 model tests、production build（2026-08-14）
- [x] Windows：Tauri desktop 7 个 Rust tests，覆盖 Host 重启回滚、SCS 安装/备份、两种打包资源布局与更新检查节流（2026-08-13）
- [x] Windows：SCS x64 native wire test 与当前平台资源 staging（2026-08-12）
- [x] Windows：Tauri MSI/NSIS 实际生成，安装清单包含 GUI、独立 Host、SCS bridge、LICENSE/NOTICE/docs（2026-08-12；本机 smoke 使用临时 updater key）
- [x] 1240×800、860×800 production UI 视觉检查：总览、配对和游戏向导无水平溢出（2026-08-12）
- [x] 1440×1000 production UI 视觉检查：总览、配对、游戏、仪表盘、网络与系统页文案精简后布局完整（2026-08-14）
- [x] 共享实例锁进程测试：CLI↔CLI 拒绝重复启动；GUI 与 CLI 使用同一 guard（2026-08-12）
- [x] 设置文件原子保存、三份备份/损坏隔离、设备凭据只存 SHA-256 摘要的测试（2026-08-12）
- [x] GitHub Actions：Windows x64、macOS Apple Silicon、macOS Intel 安装包全部成功（Release run `31584105321`，2026-08-12）
- [x] Release `latest.json` 同时包含 `windows-x86_64`、`darwin-aarch64`、`darwin-x86_64`，并逐项匹配对应 updater `.sig`（2026-08-12）
- [x] 从公开 Release 下载 Windows NSIS updater，使用应用编译内的 Minisign 公钥完成独立密码学验签；SHA-256 与 GitHub asset digest 一致（2026-08-12）
- [ ] `npm run test:host-soak`：两小时、四客户端、60 Hz，无崩溃、解析错误或超过 16 MiB 的首末 RSS 增长

## 桌面与更新实机

- [ ] Windows 干净环境安装/卸载、首次防火墙提示、托盘、关闭到托盘、开机启动
- [ ] macOS Apple Silicon 干净环境安装、Privacy & Security 放行、托盘、关闭到托盘、开机启动
- [ ] macOS Apple Silicon：原生 arm64 桌面端安装 x86_64 SCS bridge，并由 ETS2/ATS 在 Intel/Rosetta 插件 ABI 下成功加载
- [ ] macOS Intel 干净环境完成同一流程
- [ ] 安装目录中的 GUI 与无头 Host 都能运行；任一已运行时另一个报告所有者并退出
- [ ] 配置坏文件可恢复；端口冲突应用失败后旧 Host 与旧配置继续工作
- [ ] 从 v0.1.1 配置升级并保留布局、配对设备与网络设置
- [ ] 从 v0.2.1 执行一次有效签名更新到 v0.2.2；下载/验签/安装失败均保留原版本
- [ ] 系统日志目录可从控制中心打开，日志不包含 pairing/session secret

## 游戏与手机实机

- [ ] F1 24 format 2024：速度、档位、RPM、转速灯、油门、刹车、DRS
- [ ] F1 25 format 2025 与 2026 Season Pack：上述字段和 `activeAdapter = f1-25`
- [ ] ETS2/ATS：GUI 安装 bridge、游戏 SDK 提示、对应字段持续更新
- [ ] 手机竖屏、横屏与 iPad：配对、重连、stale/disconnected、每游戏独立布局
- [ ] 游戏变化时 Dashboard 自动切换 F1/卡车视觉；控制中心同步更新当前游戏
- [ ] 实机 UDP 到浏览器 rAF 完成 p95 < 100 ms；基准手机/iPad 连续 10 分钟达到帧预算

## 分发边界

- [x] 驾驶和配置完全本地；仅启用更新检查时访问 GitHub Release
- [x] 严格 CSP；WebView 仅能调用白名单 Rust commands，远程页面无 Tauri capability
- [x] 自动更新可关闭；更新包在 Host 停止前下载并验签；同步安装失败会恢复 Host
- [x] Apache-2.0、NOTICE、SCS SDK 独立许可随包
- [x] macOS 采用 ad-hoc 签名，Release/README 明示尚未 notarize
- [ ] Windows Authenticode 代码签名
- [ ] macOS Developer ID 签名与 notarization

在最后两项操作系统签名完成前，`v0.2.2` 应称为 **public preview**，不能暗示系统安装器已由受信任商业身份签名。
