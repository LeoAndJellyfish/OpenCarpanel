# OpenCarpanel 发布检查清单

本清单用于 F1 24 `0.1.x` 预览版。任何没有实测的项目必须保持未勾选，不能由自动化合成结果代替。

## 自动化门禁

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [ ] `cargo test --locked --workspace`
- [ ] `npm ci`
- [ ] `npm run check:web`
- [ ] `npm run test:web`
- [ ] `npm run build:web`
- [ ] `npm run test:host-latency`，本机 Host 链路 p95 小于 100 ms
- [ ] `npm run test:host-soak`，两小时、四客户端、60 Hz，无崩溃、解析错误或超过 16 MiB 的首末 RSS 增长

## 实机功能

- [ ] Windows 干净环境可以启动，防火墙引导清楚
- [ ] macOS 干净环境可以启动，入站网络权限清楚
- [ ] F1 24 实机 packet format 2024 正确显示速度、档位、RPM、转速灯、油门、刹车与 DRS
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
- [ ] 发布说明准确列出当前 F1 24 packet 支持范围和已知限制
