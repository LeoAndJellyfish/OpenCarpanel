# Schemas

这里保存可生成和校验的版本化契约：

- `telemetry/`：统一遥测字段与能力描述。
- `protocol/`：Host 与浏览器之间的消息封装。
- `game-plugin/`：游戏插件 manifest 与可安装 `.ocp-plugin` 包络。
- `layout/`：面板、组件实例和主题配置。

Rust 类型是 MVP 的契约源，CI 生成 JSON Schema 与 TypeScript 类型，并检查生成结果没有漂移。
