# OpenSimDash 标准游戏插件系统设计

## 状态

Accepted，按 ADR-0009 实施。

## 1. 目标与完成条件

游戏支持不再由分散的 `match`、TypeScript union 和页面常量共同定义。一个插件必须能声明自己的身份、数据协议、可提供字段、仪表盘表现和设置方式；内置与第三方插件进入相同注册表。第三方开发者应能使用公开 SDK 构建一个 decoder、打包为 `.osd-plugin`、通过桌面端安装，并在重载 Host 后出现在诊断、数据源选择和 Dashboard 中。

非功能约束：数据链路 p95 仍小于 100 ms；外部 decoder 单次执行有确定的 CPU、内存、输入和输出上限；插件损坏不能阻止 Host 启动；包安装不得目录穿越或覆盖插件目录外文件；插件不访问文件、网络、环境变量和系统时钟；现有四款游戏行为与布局不回归。

## 2. 组件与数据流

```text
plugins/games/<id>/manifest.json ─┐
                                  ├─> PluginCatalog ─> AdapterRegistry
<data>/game-plugins/<id>/         │         │                │
  manifest.json + decoder.wasm ───┘         │                ├─> native GameAdapter
                                            │                └─> WasmGameAdapter
                                            │                         │
                                     public metadata             bounded ABI v1
                                            │                         │
                         Desktop diagnostics / WebSocket capabilities │
                                            │                         │
                         setup UI / layout / widget filtering <───────┘
```

Host 始终拥有 UDP socket。每个数据报按注册顺序交给启用插件；插件只能返回“未识别”“已识别并附 updates/events”或“已识别但无效”。自动来源粘性、per-game reducer、snapshot/event 双通道保持不变。

## 3. Manifest v1

manifest 包含以下稳定部分：

- `schemaVersion`、`id`、`name`、`version`、`publisher`、`license`、`description`；
- `runtime`：内置 entrypoint，或 ABI 版本、WASM 文件名和 SHA-256；
- `protocol`：面向用户的协议名称与版本；
- `ingress`：v1 为共享 UDP，并声明建议端口和最大数据报；
- `capabilities`：统一遥测字段；
- `presentation`：短名称、family、status mode、布局预设、主题、备用红线与可用组件；
- `setup`：`f1_udp`、`scs_sdk`、`udp` 或 `none` 的声明式向导数据。

所有标识符、字符串、数组、颜色、端口和路径都有边界。外部包不得声明 `builtin` runtime；内置清单不得引用 WASM。插件 ID 同时作为 `TelemetrySnapshot.meta.gameId` 和独立布局 ID 的后缀，因此安装后不可变。

## 4. WASM ABI v1

模块不得包含 imports，并必须导出：

```text
memory
osd_plugin_abi_version() -> i32
osd_input_ptr() -> i32
osd_input_capacity() -> i32
osd_output_ptr() -> i32
osd_output_capacity() -> i32
osd_decode(input_len: i32, received_at_us: i64) -> i32
```

Host 把原始数据报写入 input buffer。返回 `0` 表示未识别；正数是 output buffer 中 UTF-8 JSON 的字节数；负数表示 decoder 拒绝了已识别数据。输出 envelope 使用 telemetry schema v1 的 updates/events。Host 覆盖所有时间戳，限制最多 8 个 update、16 个 event、256 KiB 输出、4 MiB linear memory 和每次 5,000,000 fuel。连续 trap 只影响该插件，其他 pipeline 继续工作。

ABI 不提供导入函数，也不启用 WASI。插件若需要跨包保存状态，只能保存在当前实例内存中，Host 重启后重建。

## 5. 包安装与失败处理

`.osd-plugin` 是 package schema v1 的单文件 JSON。WASM 使用 Base64 编码，manifest 中记录 SHA-256。包大小、解码后模块大小、manifest 和所有字段先在内存中有界验证。安装路径固定为 `<data>/game-plugins/<id>/`；模块使用内容哈希文件名，manifest 最后原子提交，因此中断最多导致插件被跳过，不会加载混合版本。

启动时按插件 ID 排序加载。内置 ID 优先，外部重复 ID、哈希不符、ABI 不兼容、imports、缺失导出或实例化失败都形成脱敏 `loadIssues`。固定到已缺失插件的配置退回自动模式并报告问题，Host 本身继续启动。

## 6. 前端契约

WebSocket `capabilities` 消息附带插件 public metadata。Dashboard 根据 metadata 动态生成 presentation、默认布局和编辑器游戏列表；组件由 `presentation.widgets` 与本地可信 widget registry 取交集。未知或加载中的 game ID 使用 neutral fallback，不执行插件提供的 HTML、CSS 或 JavaScript。

桌面端的游戏页和网络数据源列表来自 diagnostics 中的同一 metadata。特殊安装操作按 `setup.kind` 分派，而不是按游戏 ID；通用 UDP 插件直接展示 manifest 中的步骤。

## 7. 验证

1. 四个内置 manifest 通过 JSON Schema 与 Rust 语义验证，并和 native adapter descriptor 完全一致。
2. SDK 示例插件可以生成 ABI v1 模块；runtime 契约测试覆盖识别、未识别、错误输出、越界内存、超 fuel、imports 和哈希错误。
3. `.osd-plugin` pack/install/reload 后可在 Host diagnostics、固定选择和 WebSocket metadata 中观察到。
4. Dashboard 测试证明未知第三方 ID 能生成独立布局并按 manifest 过滤组件。
5. Rust fmt、Clippy、workspace tests、Web 类型生成/检查/测试/build、原生 SCS bridge 与性能门禁全部通过。
