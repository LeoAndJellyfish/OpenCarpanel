# Performance tests

记录固定硬件、操作系统、浏览器版本和采样时长。至少覆盖 60 Hz 遥测、四个客户端、十分钟渲染以及两小时 Host soak test。

## Host soak

先构建嵌入前端的 release Host：

```powershell
npm ci
npm run build:web
cargo build --release --locked -p opencarpanel-host
```

完整的两小时、四客户端、60 Hz 合成 F1 24 回放：

```powershell
node tests/performance/host-soak.mjs
```

日常快速烟雾验证可显式缩短，但不能替代发布门禁：

```powershell
node tests/performance/host-soak.mjs --duration-seconds 30 --sample-interval-seconds 5
```

脚本会启动本地 Host，以一个配对令牌建立四个会话，持续发送有界合成 UDP 数据，检查所有客户端仍收到 snapshot、Host 没有解析错误，并抽样进程 RSS。默认允许首末 RSS 增长不超过 16 MiB。

## 延迟边界

```powershell
cargo test -p opencarpanel-host --test end_to_end_latency --release -- --nocapture
```

这个自动化门禁测量本机 UDP 发包到 WebSocket snapshot，不包含真实游戏发包和浏览器 rAF 绘制；不得把它写成手机端完整延迟成绩。
