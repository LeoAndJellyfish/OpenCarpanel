# OpenSimDash 游戏插件开发指南

OpenSimDash v0.4 的游戏插件把“识别上游数据包并转换成统一遥测”做成可独立安装的单元。第三方插件是一个 `.osd-plugin` 文件：版本化 manifest 加一份跨平台 core WebAssembly decoder。安装插件不需要重新编译 OpenSimDash。

## 能做什么

- 接收 Host 共享 UDP 端口上的原始数据报；
- 输出统一的 `TelemetryUpdate` 与 `TelemetryEvent`；
- 声明游戏名称、协议版本、能力、主题、布局预设、适用内置组件和设置步骤；
- 在桌面控制中心中安装、升级、固定选择和卸载；
- 在手机仪表盘中自动获得独立布局与组件目录。

ABI v1 不允许插件提供可执行 JavaScript/CSS、加载本机动态库、访问文件或网络，也不为插件创建专用端口。需要游戏进程内原生 SDK 的支持仍须作为经过项目审核的内置 bridge 提交。

## 包结构

`.osd-plugin` 是一个不超过 4 MiB 的 JSON 文件：

```text
GamePluginPackage v1
├── packageVersion: 1
├── manifest: GamePluginManifest v1
└── moduleBase64: decoder.wasm 的 Base64
```

正式 JSON Schema：

- [`manifest.schema.json`](../schemas/game-plugin/v1/manifest.schema.json)
- [`package.schema.json`](../schemas/game-plugin/v1/package.schema.json)

manifest 的主要字段如下：

| 字段 | 作用 |
| --- | --- |
| `id` | 稳定的小写插件 ID；同时用于 `meta.gameId` 和 `game-<id>` 布局 ID |
| `version` | 插件自身的 SemVer 版本 |
| `runtime` | 外部插件必须是 `wasm`、ABI `1`、安全 `.wasm` 文件名与 SHA-256 |
| `protocol` | 给用户和诊断显示的上游协议名称/版本 |
| `ingress` | ABI v1 为 `shared_udp`，并声明建议端口与最大数据报 |
| `capabilities` | decoder 能产生的 canonical telemetry 字段 |
| `presentation` | 短名称、视觉 family、状态语义、布局预设、主题、备用转速与组件 |
| `setup` | `f1_udp`、通用 `udp` 或 `none` 设置流程；`scs_sdk` 仅供经项目审核并随包发布的原生 bridge |

插件 ID 发布后不可变。改 ID 会创建另一个数据源和另一份用户布局，而不是升级原插件。

## WASM ABI v1

模块不能包含任何 import，也不会获得 WASI。它必须导出：

```text
memory
osd_plugin_abi_version() -> i32
osd_input_ptr() -> i32
osd_input_capacity() -> i32
osd_output_ptr() -> i32
osd_output_capacity() -> i32
osd_decode(input_len: i32, received_at_us: i64) -> i32
```

Host 先把一份 UDP 数据报复制到 input buffer，再调用 `osd_decode`：

| 返回值 | 含义 |
| --- | --- |
| `0` | 不是本插件协议，自动识别可继续尝试其他插件 |
| `> 0` | output buffer 内有效 UTF-8 JSON 的字节数 |
| `< 0` | 数据报属于本协议但无效 |

正数输出使用 `PluginDecodeOutput` v1。Host 会覆盖 guest 提供的时间戳，并再次验证 JSON 形状、字符串、事件名、extension namespace、update/event 数量和统一遥测类型。

运行边界：

| 资源 | 上限 |
| --- | ---: |
| 单个 WASM 模块 | 2 MiB |
| guest linear memory | 4 MiB |
| 单次输出 | 256 KiB |
| 单次调用 fuel | 5,000,000 |
| 单包 updates / events | 8 / 16 |
| 单个 Host 的外部插件 | 16 |

越界、trap、错误 JSON 或错误 ABI 只会拒绝该次数据/该插件，不会执行其余系统权限，也不会让其他适配器停止工作。

## 用 Rust SDK 创建插件

仓库提供可运行的 [`examples/game-plugin-rust`](../examples/game-plugin-rust/)：它识别六字节数据包 `OSD1 + little-endian u16 km/h`，再输出标准 `vehicle.speedMps`。

核心实现只需要 `Default + GamePlugin`：

```rust
#[derive(Default)]
struct MyPlugin;

impl GamePlugin for MyPlugin {
    fn decode(&mut self, datagram: &[u8], received_at_us: u64) -> DecodeResult {
        // 先检查 magic/version；不是本协议返回 Unrecognized。
        // 严格检查长度后构造 PluginDecodeOutput。
    }
}

export_game_plugin!(MyPlugin);
```

构建和打包：

```powershell
rustup target add wasm32-unknown-unknown
cargo build -p opensimdash-example-game-plugin --target wasm32-unknown-unknown --release
cargo run -p opensimdash-game-plugin-cli -- pack examples/game-plugin-rust/manifest.json target/wasm32-unknown-unknown/release/opensimdash_example_game_plugin.wasm example-sim.osd-plugin
cargo run -p opensimdash-game-plugin-cli -- validate example-sim.osd-plugin
```

`pack` 会用实际模块文件名、ABI 版本和 SHA-256 替换 manifest 中的 runtime 占位值，然后用与 Host 相同的 loader 完成校验。

## 安装与诊断

在桌面控制中心打开“游戏设置”，选择“安装 `.osd-plugin`”。成功后 Host 会在进程内重载插件注册表，数据源选择、设置步骤和手机布局会立即出现。用户明确点击“卸载此插件”后，桌面端会先把仍固定在该插件上的来源切回自动模式，再删除插件目录并重载 Host。

安装内容只保存在当前用户数据目录的 `game-plugins/<id>/`：

- Windows：`%LOCALAPPDATA%\OpenSimDash\game-plugins\<id>\`
- macOS：`~/Library/Application Support/OpenSimDash/game-plugins/<id>/`

`/api/v1/diagnostics` 的 `supportedAdapters[].plugin` 显示已加载元数据，`pluginLoadIssues` 显示哈希、ABI、导出或冲突等脱敏错误。固定插件缺失或加载失败时，Host 会回退到自动识别并记录原因。

## 版本兼容规则

- package、manifest 与 ABI v1 当前都要求精确版本 `1`；未知版本会被拒绝，不做猜测性兼容。
- manifest 使用严格字段校验；面向未来的字段应先进入新 schema/ABI，再由 Host 明确支持。
- 插件 `version` 使用 SemVer；同一 `id` 的新包视为原地升级。
- canonical telemetry 字段来自版本化 Rust/JSON Schema。新增字段由 OpenSimDash 发版提供，插件不能通过 manifest 发明可执行前端能力。
- `presentation.widgets` 只是一份申请清单；Dashboard 只展示它与当前可信内置 widget registry 的交集。
- 插件状态保存在当前 WASM 实例内，Host 或应用重启后会重建，不能依赖持久 guest 内存。

## 提交新的内置支持

需要最高性能或原生 SDK bridge 的游戏可以向主仓库提交内置插件：

1. 在 `plugins/games/<id>/manifest.json` 添加完整声明；
2. 在独立 adapter crate 中实现 `GameAdapter`；
3. 在 Host built-in factory 注册 entrypoint；
4. 运行 schema/前端生成；
5. 增加真实 fixture、错误包、自动识别、布局和低延迟测试。

CI 会比较 native `AdapterDescriptor` 与 manifest 的 ID、名称、协议版本和能力列表，任何漂移都会失败。

## 发布前检查

```powershell
cargo test -p opensimdash-example-game-plugin
cargo run -p opensimdash-game-plugin-cli -- validate your-plugin.osd-plugin
cargo test -p opensimdash-game-plugin-runtime
cargo test -p opensimdash-host --test game_plugins
npm run check:web
npm run test:web
```

插件包目前没有发布者签名或在线市场。即使 decoder 运行在严格沙箱中，也只应安装来源可信、版本明确且能复现构建的文件。
