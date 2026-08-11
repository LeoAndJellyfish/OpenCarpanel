# Integration tests

端到端测试从本地 UDP 数据报开始，经过适配器和统一状态，最终断言 WebSocket 客户端收到的 snapshot/event 消息。

当前自动化入口：

- `cargo test -p opencarpanel-host --test udp_ingestion`：验证 UDP → adapter → reducer。
- `cargo test -p opencarpanel-host --test websocket_flow`：验证配对、背压、事件补发与连接边界。
- `cargo test -p opencarpanel-host --test end_to_end_latency --release`：以 120 个合成 F1 24 数据报测量 Host 收包到 WebSocket snapshot 的 p95；门限为 100 ms。

最后一项只覆盖本机 Host 传输链路，不等同于手机浏览器完成绘制的真实端到端延迟。真实设备结果必须单独记录在 `tests/performance/results/`。
