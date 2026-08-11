# Windows synthetic smoke — 2026-08-11

这份记录只证明本机合成链路和短时 Host 稳定性，不是 F1 24 实机、手机/iPad rAF 或两小时发布 soak 的替代品。

## 被测环境

- Code under test: `c6ff6b3`
- OS: Microsoft Windows 11 家庭版中文版，10.0.26200（build 26200）
- CPU: Intel Core i7-14650HX，24 logical processors
- Memory: 31.8 GiB
- Rust: `rustc 1.91.1 (ed61e7d7e 2025-11-07)`
- Node.js: `v24.16.0`
- Host: release profile，Windows x64，unsigned preview

## UDP → WebSocket latency

命令：

```powershell
npm run test:host-latency
```

120 个顺序合成 F1 24 car telemetry 数据报：

| 分位 | 延迟 |
| --- | ---: |
| p50 | 15.6589 ms |
| p95 | 30.6368 ms |
| p99 | 31.1044 ms |

结果：通过 100 ms p95 的 Host 本机链路门限。该测量终点是 WebSocket snapshot，不包含浏览器下一次绘制。

## Four-client Host smoke

命令：

```powershell
node tests/performance/host-soak.mjs --duration-seconds 30 --sample-interval-seconds 5
```

- Target rate: 60 Hz
- Sent packets: 1818
- Concurrent clients: 4
- Client snapshot counts: 1818, 1819, 1817, 1814
- Packet decode errors: 0
- RSS: 9.73 MiB → 9.79 MiB
- Peak RSS: 9.79 MiB
- First-to-last RSS growth: 0.05 MiB

结果：短时 smoke 通过。发布前仍必须运行默认两小时版本。

## Web and package budgets

- Driving JS: 43.54 kB raw / 16.64 kB gzip
- Lazy editor JS: 14.79 kB raw / 6.03 kB gzip
- Packaged Host executable: 3,602,944 bytes
- Executable SHA-256: `409D076672AB41D13CB3E59D6D07E0CBF5F87A611F25D36C0BA6CBFCC7565FD9`

## 尚未由本记录验证

- F1 24 实机 UDP 与字段正确性
- 手机/iPad 真实端到端 p95 和 10 分钟 60 FPS trace
- macOS release 运行与网络权限
- 两小时四客户端 soak
- Windows 代码签名与 macOS notarization
