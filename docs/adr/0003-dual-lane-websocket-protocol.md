# ADR-0003：使用 Snapshot/Event 双通道 WebSocket 协议

## Status

Accepted

## Context

车辆速度和 RPM 等连续值只需要最新状态；如果客户端处理变慢，补发旧帧会持续增加延迟。处罚、完成圈和进站等事件不能同样丢弃。局域网浏览器需要一个广泛兼容、支持双向控制和断线重连的传输方式。

## Decision

使用单个版本化 WebSocket 连接复用两种逻辑消息：

- Snapshot 是有损最新状态，Host 使用覆盖式槽位并按订阅频率采样。
- Event 是有序离散事实，Host 使用有界环形缓冲区和客户端确认序号支持短时补发。

MVP 采用 JSON。协议封装保留编码协商字段，只有性能数据证明必要时才增加 MessagePack。HTTP 仅负责静态资源、状态查询与低频配置操作。

## Consequences

### Positive

- 慢客户端不会让仪表盘显示越来越旧的数据。
- 重要事件在短时断线后仍可恢复。
- 浏览器、开发工具和测试可以直接查看 MVP 消息。
- 一个连接简化配对、生命周期和防火墙行为。

### Negative

- 客户端必须同时实现 snapshot 重置和 event 序号恢复。
- 环形缓冲区溢出后只能要求完整 resync。
- JSON 的大小和序列化成本高于专用二进制协议。

### Neutral

- WebSocket 基于 TCP；在正常局域网与目标数据量下先测量其表现，不提前引入 WebRTC/WebTransport。

## Alternatives Considered

- **所有消息可靠排队：** 实现直观，但慢客户端会积累不可接受的陈旧遥测。
- **全部使用 UDP 到浏览器：** 浏览器无通用原生 UDP API，且配对和安全更复杂。
- **WebRTC DataChannel：** 可配置不可靠通道，但本地信令与实现成本超过 MVP 收益。

## References

- [主架构设计：实时传输与背压](../plans/2026-08-11-opensimdash-architecture-design.md#8-实时传输与背压)
