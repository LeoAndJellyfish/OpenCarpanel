# OpenCarpanel 发布检查清单

本清单用于 F1 24、原始 F1 25/2025 UDP、ETS2 与 ATS 的 `0.1.x` 多游戏预览版。任何没有实测的项目必须保持未勾选，不能由合成协议 fixture 或自动化结果代替真实游戏验收。

## 自动化门禁

- [x] `cargo fmt --all -- --check`（Windows，2026-08-12）
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`（Windows，2026-08-12）
- [x] `cargo test --locked --workspace`（Windows，2026-08-12）
- [x] `npm ci`（Windows，2026-08-12）
- [x] `npm run check:web`（Windows，2026-08-12）
- [x] `npm run test:web`（Windows，2026-08-12）
- [x] `npm run build:web`（Windows，2026-08-12）
- [x] `npm run build:scs-plugin`，Windows x64 插件编译、native wire test 和安装 staging 通过（2026-08-12）
- [x] `npm run package:host`，Windows x64 Host、插件、README 视觉资源、NOTICE、项目/SCS 许可和分游戏指南均在产物中（2026-08-12）
- [x] `npm run test:package-smoke`，Windows 包内 Host 依次识别 `f1-24 → f1-25 → ets2 → ats`，4/4 recognized、0 errors（2026-08-12）
- [x] `npm run test:host-latency`，Windows release 合成 UDP→WebSocket p95 `26.65 ms`（2026-08-12）
- [ ] `npm run test:host-soak`，两小时、四客户端、60 Hz，无崩溃、解析错误或超过 16 MiB 的首末 RSS 增长

## 实机功能

- [ ] Windows 干净环境可以启动，防火墙引导清楚
- [ ] macOS 干净环境可以启动，入站网络权限清楚
- [ ] F1 24 实机 packet format 2024 正确显示速度、档位、RPM、转速灯、油门、刹车与 DRS
- [ ] F1 25 实机选择原始 `F1 25 / 2025` UDP 后正确显示速度、档位、RPM、转速灯、油门、刹车与 DRS
- [ ] F1 25 新用户若默认为 2026 Season Pack，切换到原始 2025 UDP 后可用；预览版不宣称支持 2026 UDP
- [ ] ETS2 实机加载随包 SCS 插件、接受 SDK 提示并正确显示速度、档位、RPM/RPM 上限、油门和刹车
- [ ] ATS 实机加载随包 SCS 插件、接受 SDK 提示并正确显示同一组字段
- [ ] `auto` 在单游戏运行时选中正确来源；固定 `f1-24|f1-25|ets2|ats` 时不会被其他游戏抢占
- [ ] 手机竖屏、手机横屏和 iPad 均可配对、重连并显示 stale/disconnected
- [ ] 编辑器可拖动、缩放、切换断点、保存、处理 409、导入和导出
- [ ] 损坏布局可从最近有效备份恢复，原损坏文件仍可诊断

## 真实性能

- [ ] 基准手机连续运行 10 分钟：60 FPS、帧时间 p95 < 16.7 ms、JS p95 < 3 ms、掉帧 < 1%
- [ ] iPad 连续运行 10 分钟达到同一基线
- [ ] 实机游戏 UDP 接收到浏览器 rAF 完成的 p95 < 100 ms
- [ ] 记录设备、OS、浏览器、采样时长和方法到 `tests/performance/results/`

## 安全、隐私与分发

- [ ] 产物只包含本地运行所需文件，不引用 CDN 或远程运行服务
- [ ] CSP、Origin、消息大小、连接数和布局导入边界测试通过
- [ ] 诊断导出不包含令牌、设备 session、IP、玩家名或原始 UDP
- [ ] `LICENSE`、版本和第三方许可证随包分发
- [ ] Windows 正式产物完成代码签名
- [ ] macOS 正式产物完成签名和 notarization
- [ ] 尚未完成签名时，产物必须明确标记为 unsigned preview，不能称为正式稳定版
- [ ] 更新器只有在签名 manifest、失败回滚和显式关闭选项完成后才启用

## 发布后

- [ ] 保留上一稳定版配置读取/迁移测试
- [ ] 从全新配置和既有 v1 配置各完成一次升级演练
- [ ] 失败更新不会替换当前可运行版本
- [ ] 发布说明准确列出四款游戏的字段范围、F1 25 仅支持原始 2025 UDP、卡车依赖随包插件，以及尚未完成的实机验证
