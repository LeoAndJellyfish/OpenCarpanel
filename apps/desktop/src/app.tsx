import type { JSX } from "preact";
import { useEffect, useMemo, useState } from "preact/hooks";

import {
  type AppSettings,
  type DesktopBootstrap,
  type PairingTicket,
  type RuntimeSnapshot,
  type ScsPluginStatus,
  type UpdateInfo,
  bootstrap,
  checkForUpdates,
  chooseScsDirectory,
  createPairing,
  openDashboard,
  openLogs,
  refreshRuntime,
  revokeDevice,
  saveSettings,
  installScsPlugin,
  installUpdate,
} from "./desktop-api";
import {
  type GameId,
  compactNumber,
  formatAge,
  formatDeviceTime,
  formatUptime,
  gameProfile,
  telemetryIsLive,
} from "./model";

type Section = "overview" | "pairing" | "games" | "dashboard" | "network" | "system";
type SetupGame = Exclude<GameId, "waiting">;

const NAVIGATION: readonly { id: Section; index: string; label: string; detail: string }[] = [
  { id: "overview", index: "01", label: "总览", detail: "LIVE SYSTEM" },
  { id: "pairing", index: "02", label: "设备与配对", detail: "LINK DEVICES" },
  { id: "games", index: "03", label: "游戏设置", detail: "INPUT GARAGE" },
  { id: "dashboard", index: "04", label: "仪表盘", detail: "DRIVER DISPLAY" },
  { id: "network", index: "05", label: "网络", detail: "LOCAL TRANSPORT" },
  { id: "system", index: "06", label: "系统与诊断", detail: "SERVICE BAY" },
];

const GAME_TABS: readonly { id: SetupGame; label: string; detail: string }[] = [
  { id: "f1-24", label: "F1 24", detail: "UDP 2024" },
  { id: "f1-25", label: "F1 25", detail: "2025 + 2026" },
  { id: "ets2", label: "ETS2", detail: "SCS SDK" },
  { id: "ats", label: "ATS", detail: "SCS SDK" },
];

export function App() {
  const [data, setData] = useState<DesktopBootstrap | null>(null);
  const [draft, setDraft] = useState<AppSettings | null>(null);
  const [section, setSection] = useState<Section>(initialSection);
  const [setupGame, setSetupGame] = useState<SetupGame>("f1-25");
  const [pairing, setPairing] = useState<PairingTicket | null>(null);
  const [scsStatus, setScsStatus] = useState<ScsPluginStatus | null>(null);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void bootstrap()
      .then((value) => {
        if (!active) return;
        setData(value);
        setDraft(value.settings);
        const detected = value.diagnostics.activeAdapter;
        if (detected && detected !== "waiting") setSetupGame(detected as SetupGame);
      })
      .catch((reason: unknown) => active && setError(errorText(reason)));
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    const synchronizeSection = () => setSection(initialSection());
    window.addEventListener("popstate", synchronizeSection);
    window.addEventListener("hashchange", synchronizeSection);
    return () => {
      window.removeEventListener("popstate", synchronizeSection);
      window.removeEventListener("hashchange", synchronizeSection);
    };
  }, []);

  useEffect(() => {
    if (!data) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState !== "visible" || busy === "settings") return;
      void refreshRuntime()
        .then((snapshot) => setData((current) => mergeRuntime(current, snapshot)))
        .catch((reason: unknown) => setError(errorText(reason)));
    }, 800);
    return () => window.clearInterval(timer);
  }, [data !== null, busy]);

  const profile = useMemo(
    () => (data ? gameProfile(data.diagnostics) : null),
    [data?.diagnostics.activeAdapter, data?.diagnostics.adapterSelection],
  );

  async function commitSettings(next: AppSettings, successMessage: string) {
    setBusy("settings");
    setError(null);
    try {
      const value = await saveSettings(next);
      setData(value);
      setDraft(value.settings);
      setMessage(successMessage);
    } catch (reason) {
      setError(errorText(reason));
      setDraft(data?.settings ?? next);
    } finally {
      setBusy(null);
    }
  }

  async function issuePairing() {
    setBusy("pairing");
    setError(null);
    try {
      setPairing(await createPairing());
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(null);
    }
  }

  async function removeDevice(deviceId: string) {
    setBusy(`device:${deviceId}`);
    setError(null);
    try {
      const snapshot = await revokeDevice(deviceId);
      setData((current) => mergeRuntime(current, snapshot));
      setMessage("设备访问已撤销");
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(null);
    }
  }

  async function chooseGameDirectory() {
    setBusy("scs-folder");
    setError(null);
    try {
      if (setupGame !== "ets2" && setupGame !== "ats") return;
      const selected = await chooseScsDirectory(setupGame);
      if (selected) setScsStatus(selected);
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(null);
    }
  }

  async function installGamePlugin() {
    if (!scsStatus) return;
    setBusy("scs-install");
    setError(null);
    try {
      const installed = await installScsPlugin(scsStatus);
      setScsStatus(installed);
      setMessage("SCS bridge 已安装并通过 SHA-256 校验；完全重启游戏后接受 SDK 提示");
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(null);
    }
  }

  async function checkUpdates() {
    setBusy("update-check");
    setError(null);
    try {
      const info = await checkForUpdates();
      setUpdateInfo(info);
      setMessage(info.available ? `发现 OpenCarpanel ${info.version}` : "当前已经是最新版本");
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(null);
    }
  }

  async function applyUpdate() {
    setBusy("update-install");
    setError(null);
    try {
      await installUpdate();
    } catch (reason) {
      setError(errorText(reason));
      setBusy(null);
    }
  }

  if (!data || !draft || !profile) {
    return <BootScreen error={error} />;
  }

  const live = telemetryIsLive(data.diagnostics);
  const activeNavigation =
    NAVIGATION.find((item) => item.id === section) ?? {
      id: "overview" as const,
      index: "01",
      label: "总览",
      detail: "LIVE SYSTEM",
    };

  return (
    <div class="control-center" data-game={profile.id}>
      <aside class="side-rail">
        <div class="brand-lockup">
          <div class="brand-glyph" aria-hidden="true">
            <i />
            <i />
            <i />
          </div>
          <div>
            <strong>OPEN<span>CARPANEL</span></strong>
            <small>PIT WALL / {data.version}</small>
          </div>
        </div>

        <div class="rail-runtime" data-live={live}>
          <span class="live-orbit" aria-hidden="true"><i /></span>
          <div>
            <small>{live ? "TELEMETRY LIVE" : "HOST ARMED"}</small>
            <strong>{profile.shortLabel}</strong>
          </div>
        </div>

        <nav aria-label="控制中心">
          {NAVIGATION.map((item) => (
            <button
              key={item.id}
              class={section === item.id ? "nav-item is-active" : "nav-item"}
              onClick={() => setSection(item.id)}
              type="button"
            >
              <span class="nav-index">{item.index}</span>
              <span>
                <strong>{item.label}</strong>
                <small>{item.detail}</small>
              </span>
              <i aria-hidden="true" />
            </button>
          ))}
        </nav>

        <div class="rail-footer">
          <span class="host-led" data-ok={data.diagnostics.status === "ok"} />
          <span>
            <small>EMBEDDED HOST</small>
            <strong>{data.endpoints.udpAddress}</strong>
          </span>
        </div>
      </aside>

      <main class="workspace">
        <header class="topline">
          <div>
            <p class="section-code">{activeNavigation.index} / {activeNavigation.detail}</p>
            <h1>{activeNavigation.label}</h1>
          </div>
          <div class="game-ident">
            <span data-live={live}>{live ? "LIVE" : "STANDBY"}</span>
            <div>
              <strong>{profile.label}</strong>
              <small>{profile.detail}</small>
            </div>
          </div>
        </header>

        {(error || message) && (
          <div class={error ? "notice notice-error" : "notice notice-ok"} role="status">
            <span>{error ? "!" : "✓"}</span>
            <p>{error ?? message}</p>
            <button
              type="button"
              aria-label="关闭消息"
              onClick={() => {
                setError(null);
                setMessage(null);
              }}
            >×</button>
          </div>
        )}

        <div class="view-stage" key={section}>
          {section === "overview" && (
            <Overview
              data={data}
              live={live}
              onCompleteOnboarding={() =>
                void commitSettings(
                  {
                    ...data.settings,
                    desktop: { ...data.settings.desktop, onboardingComplete: true },
                  },
                  "首启检查已完成，之后仍可从各页面重新配置",
                )
              }
              onNavigate={setSection}
              onOpen={(target) => void openDashboard(target).catch((reason) => setError(errorText(reason)))}
              profileLabel={profile.label}
            />
          )}
          {section === "pairing" && (
            <PairingView
              busy={busy}
              data={data}
              onIssue={() => void issuePairing()}
              onRevoke={(id) => void removeDevice(id)}
              pairing={pairing}
            />
          )}
          {section === "games" && (
            <GamesView
              busy={busy}
              data={data}
              onChooseDirectory={() => void chooseGameDirectory()}
              onInstallPlugin={() => void installGamePlugin()}
              onSelectGame={setSetupGame}
              onSetSelection={(selection) =>
                void commitSettings(
                  {
                    ...data.settings,
                    host: { ...data.settings.host, adapterSelection: selection },
                  },
                  selection === "auto" ? "已启用游戏自动识别" : `已固定到 ${selection}`,
                )
              }
              scsStatus={scsStatus?.game === setupGame ? scsStatus : null}
              selected={setupGame}
            />
          )}
          {section === "dashboard" && (
            <DashboardView
              data={data}
              onOpen={(target) => void openDashboard(target).catch((reason) => setError(errorText(reason)))}
            />
          )}
          {section === "network" && (
            <NetworkView
              busy={busy}
              data={data}
              draft={draft}
              onChange={setDraft}
              onSave={(settings) => void commitSettings(settings, "网络设置已应用并安全保存")}
            />
          )}
          {section === "system" && (
            <SystemView
              busy={busy}
              data={data}
              onChange={(settings, success) => void commitSettings(settings, success)}
              onOpenDiagnostics={() =>
                void openDashboard("diagnostics").catch((reason) => setError(errorText(reason)))
              }
              onOpenLogs={() => void openLogs().catch((reason) => setError(errorText(reason)))}
              onCheckUpdate={() => void checkUpdates()}
              onInstallUpdate={() => void applyUpdate()}
              updateInfo={updateInfo}
            />
          )}
        </div>
      </main>
    </div>
  );
}

function BootScreen({ error }: { error: string | null }) {
  return (
    <main class="boot-shell">
      <div class="boot-mark" aria-hidden="true"><span /><span /><span /></div>
      <p class="eyebrow">OpenCarpanel / Pit Wall</p>
      <h1>{error ? "控制中心未能就绪" : "正在启动桌面控制中心"}</h1>
      <p>{error ?? "Rust Host、局域网与游戏适配器正在完成自检。"}</p>
    </main>
  );
}

interface OverviewProps {
  data: DesktopBootstrap;
  live: boolean;
  profileLabel: string;
  onNavigate: (section: Section) => void;
  onCompleteOnboarding: () => void;
  onOpen: (target: "dashboard" | "editor") => void;
}

function Overview({
  data,
  live,
  profileLabel,
  onNavigate,
  onCompleteOnboarding,
  onOpen,
}: OverviewProps) {
  const diagnostics = data.diagnostics;
  const steps = [
    { code: "01", label: "GAME", value: diagnostics.activeAdapter ?? "WAITING", active: live },
    {
      code: "02",
      label: "UDP IN",
      value: compactNumber(diagnostics.telemetry.packetsReceived),
      active: diagnostics.telemetry.packetsReceived > 0,
    },
    { code: "03", label: "HOST", value: "READY", active: diagnostics.status === "ok" },
    {
      code: "04",
      label: "WEBSOCKET",
      value: `${diagnostics.connections.active} / ${diagnostics.connections.limit}`,
      active: diagnostics.connections.active > 0,
    },
    {
      code: "05",
      label: "DISPLAY",
      value: `${data.devices.length} DEVICE${data.devices.length === 1 ? "" : "S"}`,
      active: data.devices.length > 0,
    },
  ];

  return (
    <>
      {!data.settings.desktop.onboardingComplete && (
        <section class="commissioning-strip">
          <div>
            <span>FIRST RUN / 3 LAPS</span>
            <strong>把第一块仪表盘送上赛道</strong>
          </div>
          <ol>
            <li class={diagnostics.activeAdapter ? "done" : ""}>配置游戏</li>
            <li class={diagnostics.telemetry.packetsReceived > 0 ? "done" : ""}>验证数据</li>
            <li class={data.devices.length > 0 ? "done" : ""}>配对设备</li>
          </ol>
          <div class="commission-actions">
            <button class="button-quiet" type="button" onClick={() => onNavigate("games")}>开始设置</button>
            <button class="text-action" type="button" onClick={onCompleteOnboarding}>稍后提醒</button>
          </div>
        </section>
      )}

      <section class="hero-telemetry">
        <div class="hero-copy">
          <p class="eyebrow">ACTIVE SIGNAL</p>
          <h2>{live ? profileLabel : "监听已就绪"}</h2>
          <p>
            {live
              ? `最新游戏数据距现在 ${formatAge(diagnostics.telemetry.lastPacketAgeMs)}，正在通过本地链路发布。`
              : `Host 正在 ${data.endpoints.udpAddress} 等待受支持的游戏数据，不访问远程运行服务。`}
          </p>
          <div class="hero-actions">
            <button class="button-signal" type="button" onClick={() => onOpen("dashboard")}>
              打开仪表盘 <span>↗</span>
            </button>
            <button class="button-quiet" type="button" onClick={() => onNavigate("pairing")}>
              配对新设备
            </button>
          </div>
        </div>
        <div class="signal-scope" data-live={live} aria-label={live ? "遥测数据活动" : "等待遥测"}>
          <div class="scope-grid" />
          <div class="scope-ring ring-one" />
          <div class="scope-ring ring-two" />
          <div class="scope-sweep" />
          <span class="scope-center"><i /></span>
          <output>{formatAge(diagnostics.telemetry.lastPacketAgeMs)}</output>
          <small>LAST PACKET</small>
        </div>
      </section>

      <section class="data-route" aria-label="当前数据链路">
        <div class="route-heading">
          <p>LOCAL DATA ROUTE</p>
          <span>{live ? "链路在线" : "等待源数据"}</span>
        </div>
        <div class="route-steps">
          {steps.map((step, index) => (
            <div class="route-step" data-active={step.active} key={step.code}>
              <span class="step-node"><i /></span>
              <small>{step.code} / {step.label}</small>
              <strong>{step.value}</strong>
              {index < steps.length - 1 && <span class="route-line" aria-hidden="true" />}
            </div>
          ))}
        </div>
      </section>

      <section class="metric-band">
        <Metric label="接收数据包" value={compactNumber(diagnostics.telemetry.packetsReceived)} detail="UDP DATAGRAMS" />
        <Metric label="识别率" value={recognitionRate(diagnostics)} detail={`${diagnostics.telemetry.packetErrors} ERRORS`} />
        <Metric label="运行时间" value={formatUptime(diagnostics.uptimeMs)} detail="THIS SESSION" />
        <Metric label="手机连接" value={`${diagnostics.connections.active}`} detail={`OF ${diagnostics.connections.limit} SLOTS`} />
      </section>

      <section class="adapter-board">
        <div class="panel-heading">
          <div><p>ADAPTER GRID</p><h3>已编译游戏输入</h3></div>
          <button class="text-action" type="button" onClick={() => onNavigate("games")}>打开游戏设置 →</button>
        </div>
        <div class="adapter-table">
          {diagnostics.supportedAdapters.map((adapter) => (
            <div class="adapter-row" data-active={adapter.id === diagnostics.activeAdapter} key={adapter.id}>
              <span class="adapter-light" />
              <div><strong>{adapter.displayName}</strong><small>{adapter.id}</small></div>
              <code>{adapter.protocolVersion}</code>
              <span>{compactNumber(adapter.packetsRecognized)} PKT</span>
              <span>{formatAge(adapter.lastPacketAgeMs)}</span>
            </div>
          ))}
        </div>
      </section>
    </>
  );
}

function Metric({ label, value, detail }: { label: string; value: string; detail: string }) {
  return <div class="metric"><small>{label}</small><strong>{value}</strong><span>{detail}</span></div>;
}

function PairingView({
  data,
  pairing,
  busy,
  onIssue,
  onRevoke,
}: {
  data: DesktopBootstrap;
  pairing: PairingTicket | null;
  busy: string | null;
  onIssue: () => void;
  onRevoke: (id: string) => void;
}) {
  const qrSource = pairing
    ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(pairing.qrSvg)}`
    : null;
  return (
    <div class="split-view pairing-view">
      <section class="pair-console">
        <div class="panel-heading">
          <div><p>ONE-TIME LINK</p><h2>扫描并接入驾驶屏</h2></div>
          <span class="security-tag">LOCAL ONLY</span>
        </div>
        <p class="lede">手机或 iPad 与电脑连接同一局域网。配对凭据只在二维码 URL 片段中出现，一次使用后立即失效。</p>
        <div class={pairing ? "qr-stage is-ready" : "qr-stage"}>
          {qrSource ? (
            <img src={qrSource} alt="OpenCarpanel 一次性配对二维码" />
          ) : (
            <div class="qr-idle" aria-hidden="true"><span /><span /><span /><span /><i>QR</i></div>
          )}
          <div class="qr-meta">
            <small>{pairing ? "PAIRING WINDOW OPEN" : "NO ACTIVE TICKET"}</small>
            <strong>{pairing ? `${Math.round(pairing.expiresInSeconds / 60)} 分钟内一次有效` : "需要时再生成，凭据不会写入磁盘"}</strong>
          </div>
        </div>
        {pairing && <code class="pair-url">{pairing.url}</code>}
        <button class="button-signal full-button" type="button" disabled={busy === "pairing"} onClick={onIssue}>
          {busy === "pairing" ? "正在创建安全凭据…" : pairing ? "换一个新二维码" : "生成配对二维码"}
        </button>
        <p class="microcopy">仪表盘地址：<span>{data.endpoints.dashboardUrl}</span></p>
      </section>

      <section class="device-console">
        <div class="panel-heading">
          <div><p>REMEMBERED DISPLAYS</p><h2>已配对设备</h2></div>
          <strong class="count-badge">{data.devices.length.toString().padStart(2, "0")}</strong>
        </div>
        {data.devices.length === 0 ? (
          <div class="empty-state"><span>00</span><strong>还没有已配对的显示设备</strong><p>生成左侧二维码并用手机浏览器扫描。</p></div>
        ) : (
          <div class="device-list">
            {data.devices.map((device, index) => (
              <article class="device-row" key={device.id}>
                <span class="device-index">{String(index + 1).padStart(2, "0")}</span>
                <div class="device-icon" aria-hidden="true"><i /></div>
                <div class="device-copy">
                  <strong>{device.name}</strong>
                  <small>最后连接 {formatDeviceTime(device.lastSeenUnixMs)}</small>
                  <span>首次配对 {formatDeviceTime(device.pairedAtUnixMs)}</span>
                </div>
                <button
                  class="button-danger"
                  type="button"
                  disabled={busy === `device:${device.id}`}
                  onClick={() => onRevoke(device.id)}
                >撤销</button>
              </article>
            ))}
          </div>
        )}
        <div class="device-footnote"><span>SECURITY</span><p>电脑仅保存会话凭据的 SHA-256 摘要；原始会话只保存在该设备浏览器中。</p></div>
      </section>
    </div>
  );
}

function GamesView({
  data,
  selected,
  scsStatus,
  busy,
  onSelectGame,
  onChooseDirectory,
  onInstallPlugin,
  onSetSelection,
}: {
  data: DesktopBootstrap;
  selected: SetupGame;
  scsStatus: ScsPluginStatus | null;
  busy: string | null;
  onSelectGame: (game: SetupGame) => void;
  onChooseDirectory: () => void;
  onInstallPlugin: () => void;
  onSetSelection: (selection: AppSettings["host"]["adapterSelection"]) => void;
}) {
  const isScs = selected === "ets2" || selected === "ats";
  const adapter = data.diagnostics.supportedAdapters.find((item) => item.id === selected);
  const udpPort = data.endpoints.udpAddress.split(":").at(-1) ?? "20777";
  return (
    <>
      <div class="game-tabs" role="tablist" aria-label="选择要配置的游戏">
        {GAME_TABS.map((game) => (
          <button
            role="tab"
            aria-selected={selected === game.id}
            class={selected === game.id ? "is-active" : ""}
            type="button"
            onClick={() => onSelectGame(game.id)}
            key={game.id}
          >
            <span>{game.label}</span><small>{game.detail}</small>
          </button>
        ))}
      </div>

      <div class="split-view game-setup">
        <section class="setup-main">
          <div class="panel-heading">
            <div><p>INPUT COMMISSIONING</p><h2>{adapter?.displayName ?? selected}</h2></div>
            <span class="protocol-tag">{adapter?.protocolVersion ?? "COMPILED"}</span>
          </div>
          {isScs ? (
            <ScsSetup
              busy={busy}
              game={selected}
              status={scsStatus}
              onChooseDirectory={onChooseDirectory}
              onInstall={onInstallPlugin}
              udpPort={udpPort}
            />
          ) : (
            <F1Setup game={selected} udpPort={udpPort} />
          )}
        </section>

        <aside class="setup-aside">
          <div class="panel-heading"><div><p>SOURCE POLICY</p><h3>输入选择</h3></div></div>
          <div class="selection-readout">
            <small>CURRENT MODE</small>
            <strong>{data.settings.host.adapterSelection === "auto" ? "自动识别" : data.settings.host.adapterSelection}</strong>
            <p>自动模式会锁定最近活动来源 2 秒，避免多游戏数据来回抢占。</p>
          </div>
          <button
            class={data.settings.host.adapterSelection === "auto" ? "choice-button selected" : "choice-button"}
            disabled={busy === "settings"}
            type="button"
            onClick={() => onSetSelection("auto")}
          ><span>AUTO</span><div><strong>自动识别</strong><small>大众默认选择</small></div></button>
          <button
            class={data.settings.host.adapterSelection === selected ? "choice-button selected" : "choice-button"}
            disabled={busy === "settings"}
            type="button"
            onClick={() => onSetSelection(selected)}
          ><span>LOCK</span><div><strong>固定 {selected}</strong><small>排障或多游戏并行</small></div></button>
          <div class="capability-list">
            <small>CANONICAL FIELDS</small>
            <div>{adapter?.capabilities.map((field) => <span key={field}>{field}</span>)}</div>
          </div>
        </aside>
      </div>
    </>
  );
}

function F1Setup({ game, udpPort }: { game: "f1-24" | "f1-25"; udpPort: string }) {
  const mode = game === "f1-24" ? "F1 24 / 2024" : "F1 25 / 2025 或 2026 Season Pack";
  return (
    <div class="wizard-body">
      <p class="lede">F1 系列直接使用游戏内置 UDP，不需要安装插件。进入游戏的 Telemetry Settings 并逐项核对。</p>
      <ol class="setup-steps">
        <li><span>01</span><div><strong>开启 UDP Telemetry</strong><p>把 UDP Telemetry 设置为 <code>On</code>。</p></div><b>ON</b></li>
        <li><span>02</span><div><strong>设置本机目标</strong><p>UDP IP Address 使用当前游戏电脑的回环地址。</p></div><b>127.0.0.1</b></li>
        <li><span>03</span><div><strong>对齐端口与频率</strong><p>发送端口必须与 Host 监听一致，建议 60 Hz。</p></div><b>{udpPort} / 60 HZ</b></li>
        <li><span>04</span><div><strong>选择精确 UDP 格式</strong><p>适配器按官方包头和精确包长识别，不猜测相邻版本。</p></div><b>{mode}</b></li>
      </ol>
      {game === "f1-25" && (
        <div class="season-note"><span>2026</span><p><strong>Season Pack 已兼容</strong>新用户可保留默认 2026 UDP mode；原始 2025 mode 也继续支持。</p></div>
      )}
    </div>
  );
}

function ScsSetup({
  game,
  status,
  busy,
  onChooseDirectory,
  onInstall,
  udpPort,
}: {
  game: "ets2" | "ats";
  status: ScsPluginStatus | null;
  busy: string | null;
  onChooseDirectory: () => void;
  onInstall: () => void;
  udpPort: string;
}) {
  return (
    <div class="wizard-body">
      <p class="lede">SCS 游戏通过项目随附的原生 SDK bridge 读取遥测，再以固定本地数据包送到同一个 UDP 入口。</p>
      <div class="folder-picker">
        <div><small>GAME ROOT / {game.toUpperCase()}</small><strong>{status?.gameDirectory ?? "尚未选择游戏目录"}</strong></div>
        <button class="button-quiet" type="button" disabled={busy === "scs-folder"} onClick={onChooseDirectory}>
          {busy === "scs-folder" ? "正在打开…" : "选择文件夹"}
        </button>
      </div>
      <ol class="setup-steps compact">
        <li><span>01</span><div><strong>选择游戏根目录</strong><p>控制中心只从原生目录选择器接受路径。</p></div><b>{status ? "SELECTED" : "WAITING"}</b></li>
        <li><span>02</span><div><strong>检查并安装 Bridge</strong><p>插件进入 <code>bin/win_x64/plugins</code> 或对应 macOS 目录。</p></div><b>{status?.state.toUpperCase() ?? "SAFE COPY"}</b></li>
        <li><span>03</span><div><strong>重启游戏并确认 SDK</strong><p>SCS 首次载入时会显示 SDK 提示，必须由玩家确认。</p></div><b>RESTART</b></li>
        <li><span>04</span><div><strong>验证本地遥测</strong><p>Bridge 仅向回环地址的 UDP {udpPort} 发送 44 字节数据包。</p></div><b>127.0.0.1</b></li>
      </ol>
      <button class="button-signal full-button" type="button" disabled={!status || status.state === "current" || busy === "scs-install"} onClick={onInstall}>
        {busy === "scs-install" ? "正在备份、安装并校验…" : status?.state === "current" ? "SCS Bridge 已是当前版本" : "检查并安装 SCS Bridge"}
      </button>
      <p class="microcopy">{status ? `目标：${status.pluginPath}` : "当前界面已限制为系统文件夹选择；安装器会校验随包 artifact、备份旧插件并原子替换。"}</p>
    </div>
  );
}

function DashboardView({
  data,
  onOpen,
}: {
  data: DesktopBootstrap;
  onOpen: (target: "dashboard" | "editor") => void;
}) {
  return (
    <div class="dashboard-view">
      <section class="dashboard-launcher">
        <div class="launcher-copy">
          <p class="eyebrow">DRIVER DISPLAY</p>
          <h2>一块屏幕，跟随游戏自动换挡</h2>
          <p>手机驾驶页从可信的 <code>gameId</code> 切换视觉与独立布局。速度、RPM 等高频数值继续由单一渲染循环更新，不让 Preact 全树跟着 60 Hz 重绘。</p>
          <div class="hero-actions">
            <button class="button-signal" type="button" onClick={() => onOpen("dashboard")}>打开驾驶页 <span>↗</span></button>
            <button class="button-quiet" type="button" onClick={() => onOpen("editor")}>打开布局编辑器 <span>↗</span></button>
          </div>
          <div class="url-plate"><small>MOBILE URL</small><code>{data.endpoints.dashboardUrl}</code></div>
        </div>
        <DashboardMiniature />
      </section>
      <section class="profile-lanes">
        <div class="panel-heading"><div><p>GAME-AWARE PROFILES</p><h3>四套隔离的用户布局</h3></div><span>SWITCH ON GAME ID</span></div>
        {GAME_TABS.map((game, index) => (
          <div class="profile-lane" key={game.id}>
            <span>{String(index + 1).padStart(2, "0")}</span>
            <div><strong>{game.label}</strong><small>{game.detail}</small></div>
            <code>game-{game.id}</code>
            <p>{game.id.startsWith("f1") ? "方程式布局 · 转速灯 · DRS" : "卡车布局 · 速度优先 · SCS 状态"}</p>
          </div>
        ))}
      </section>
    </div>
  );
}

function DashboardMiniature() {
  return (
    <div class="display-mock" aria-label="F1 仪表盘预览">
      <div class="display-bezel">
        <div class="shift-lights">{Array.from({ length: 12 }, (_, index) => <i key={index} />)}</div>
        <span class="mock-game">F1 25 / 2026</span>
        <strong class="mock-gear">7</strong>
        <div class="mock-speed"><b>312</b><small>KM/H</small></div>
        <div class="mock-rpm"><span /></div>
        <div class="mock-status"><span>DRS <b>ACTIVE</b></span><span>RPM <b>11,820</b></span></div>
      </div>
      <span class="display-foot">60 FPS / LOCAL WEBSOCKET</span>
    </div>
  );
}

function NetworkView({
  data,
  draft,
  busy,
  onChange,
  onSave,
}: {
  data: DesktopBootstrap;
  draft: AppSettings;
  busy: string | null;
  onChange: (settings: AppSettings) => void;
  onSave: (settings: AppSettings) => void;
}) {
  function updateHost<K extends keyof AppSettings["host"]>(key: K, value: AppSettings["host"][K]) {
    onChange({ ...draft, host: { ...draft.host, [key]: value } });
  }
  function submit(event: JSX.TargetedSubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    onSave(draft);
  }
  return (
    <div class="split-view network-view">
      <form class="network-form" onSubmit={submit}>
        <div class="panel-heading"><div><p>LISTENER CONTROL</p><h2>本地端口与发布频率</h2></div><span class="security-tag">VALIDATED</span></div>
        <p class="lede">保存前会验证地址。影响 Host 的变更会监督重启；新端口无法绑定时自动恢复原设置。</p>
        <label class="field-row">
          <span><strong>游戏遥测 UDP</strong><small>游戏或 SCS bridge 的目标监听地址</small></span>
          <input value={draft.host.udpBind} onInput={(event) => updateHost("udpBind", event.currentTarget.value)} spellcheck={false} />
        </label>
        <label class="field-row">
          <span><strong>Dashboard HTTP</strong><small>手机浏览器与 WebSocket 的局域网入口</small></span>
          <input value={draft.host.httpBind} onInput={(event) => updateHost("httpBind", event.currentTarget.value)} spellcheck={false} />
        </label>
        <label class="field-row">
          <span><strong>Snapshot 上限</strong><small>每个手机客户端的最新状态发布上限</small></span>
          <select
            value={draft.host.snapshotHz}
            onChange={(event) => updateHost("snapshotHz", Number(event.currentTarget.value) as 20 | 30 | 60)}
          ><option value="20">20 Hz</option><option value="30">30 Hz</option><option value="60">60 Hz</option></select>
        </label>
        <label class="field-row">
          <span><strong>游戏来源</strong><small>自动识别或固定一条已编译 adapter</small></span>
          <select
            value={draft.host.adapterSelection}
            onChange={(event) => updateHost("adapterSelection", event.currentTarget.value as AppSettings["host"]["adapterSelection"])}
          >
            <option value="auto">自动识别</option><option value="f1-24">F1 24</option><option value="f1-25">F1 25</option><option value="ets2">ETS2</option><option value="ats">ATS</option>
          </select>
        </label>
        <div class="form-actions">
          <span>实际绑定：HTTP {data.endpoints.httpAddress} · UDP {data.endpoints.udpAddress}</span>
          <button class="button-signal" disabled={busy === "settings"} type="submit">{busy === "settings" ? "正在切换…" : "应用设置"}</button>
        </div>
      </form>
      <aside class="network-map">
        <div class="panel-heading"><div><p>TRUST BOUNDARY</p><h3>本地数据面</h3></div></div>
        <div class="map-stack">
          <div><span>GAME</span><strong>UDP / {data.endpoints.udpAddress.split(":").at(-1)}</strong><small>untrusted datagrams</small></div>
          <i />
          <div class="active"><span>RUST HOST</span><strong>VALIDATE + NORMALIZE</strong><small>single process</small></div>
          <i />
          <div><span>MOBILE</span><strong>PAIRED WEBSOCKET</strong><small>{data.diagnostics.connections.active} active client(s)</small></div>
        </div>
        <div class="network-facts"><p><span>REMOTE SERVER</span><strong>NONE</strong></p><p><span>PROTOCOL</span><strong>v{data.diagnostics.protocolVersion}</strong></p><p><span>CLIENT LIMIT</span><strong>{data.diagnostics.connections.limit}</strong></p></div>
      </aside>
    </div>
  );
}

function SystemView({
  data,
  busy,
  onChange,
  onOpenDiagnostics,
  onOpenLogs,
  onCheckUpdate,
  onInstallUpdate,
  updateInfo,
}: {
  data: DesktopBootstrap;
  busy: string | null;
  onChange: (settings: AppSettings, message: string) => void;
  onOpenDiagnostics: () => void;
  onOpenLogs: () => void;
  onCheckUpdate: () => void;
  onInstallUpdate: () => void;
  updateInfo: UpdateInfo | null;
}) {
  function toggle(key: keyof AppSettings["desktop"], value: boolean, message: string) {
    onChange({ ...data.settings, desktop: { ...data.settings.desktop, [key]: value } }, message);
  }
  return (
    <div class="system-view">
      {(data.recovery.recovered || data.recovery.resetToDefaults) && (
        <div class="recovery-banner"><span>RECOVERY</span><p>{data.recovery.recovered ? "已从有效备份恢复设置。" : "设置损坏且无有效备份，已使用安全默认值。"}</p><code>{data.recovery.quarantinedPath}</code></div>
      )}
      <div class="split-view">
        <section class="preference-panel">
          <div class="panel-heading"><div><p>DESKTOP BEHAVIOUR</p><h2>常驻与系统集成</h2></div></div>
          <Toggle
            checked={data.settings.desktop.closeToTray}
            detail={data.trayAvailable ? "关闭窗口后 Host 继续接收遥测；从系统托盘重新打开。" : "本次启动未能注册系统托盘；关闭窗口将正常退出。"}
            disabled={busy === "settings" || !data.trayAvailable}
            label="关闭到系统托盘"
            onChange={(value) => toggle("closeToTray", value, "托盘行为已更新")}
          />
          <Toggle
            checked={data.autostartEnabled}
            detail="使用操作系统登录项启动桌面控制中心。"
            disabled={busy === "settings"}
            label="登录时自动启动"
            onChange={(value) => toggle("launchAtLogin", value, "开机启动设置已同步到操作系统")}
          />
          <Toggle
            checked={data.settings.desktop.notificationsEnabled}
            detail="仅报告本地游戏切换、Host 故障和更新结果。"
            disabled={busy === "settings"}
            label="桌面通知"
            onChange={(value) => toggle("notificationsEnabled", value, "通知偏好已保存")}
          />
          <Toggle
            checked={data.settings.desktop.automaticUpdates}
            detail="允许每天最多一次访问 GitHub Release 签名清单。"
            disabled={busy === "settings"}
            label="自动检查签名更新"
            onChange={(value) => toggle("automaticUpdates", value, "自动更新偏好已保存")}
          />
        </section>

        <section class="update-panel">
          <div class="panel-heading"><div><p>SECURE UPDATE</p><h2>软件更新</h2></div><span class="security-tag">SIGNED</span></div>
          <div class="version-lockup"><small>{updateInfo?.available ? "UPDATE READY" : "INSTALLED"}</small><strong>v{updateInfo?.version ?? data.version}</strong><span>{updateInfo?.available ? `当前 v${data.version}` : "Release channel / GitHub"}</span></div>
          <p>{updateInfo?.notes ?? "更新包必须通过编译进应用的公钥验证。下载或验签失败时不会运行安装器，当前版本继续工作。"}</p>
          {updateInfo?.available ? (
            <button class="button-signal full-button" disabled={busy === "update-install"} type="button" onClick={onInstallUpdate}>{busy === "update-install" ? "正在下载、验签并安装…" : `安装 v${updateInfo.version}`}</button>
          ) : (
            <button class="button-quiet full-button" disabled={busy === "update-check"} type="button" onClick={onCheckUpdate}>{busy === "update-check" ? "正在访问签名发布清单…" : "手动检查更新"}</button>
          )}
          <small class="microcopy">仅检查时访问互联网；驾驶与配置始终本地运行。</small>
        </section>
      </div>

      <section class="diagnostic-panel">
        <div class="panel-heading">
          <div><p>SERVICE TELEMETRY</p><h2>诊断快照</h2></div>
          <div class="diagnostic-actions">
            <button class="button-quiet" type="button" onClick={onOpenLogs}>打开日志目录</button>
            <button class="button-quiet" type="button" onClick={onOpenDiagnostics}>打开完整 JSON ↗</button>
          </div>
        </div>
        <div class="diagnostic-grid">
          <Diagnostic label="HOST STATUS" value={data.diagnostics.status.toUpperCase()} tone="good" />
          <Diagnostic label="ACTIVE ADAPTER" value={data.diagnostics.activeAdapter ?? "NONE"} />
          <Diagnostic label="PACKET ERRORS" value={String(data.diagnostics.telemetry.packetErrors)} tone={data.diagnostics.telemetry.packetErrors ? "bad" : "good"} />
          <Diagnostic label="EVENT RESYNCS" value={String(data.diagnostics.telemetry.eventResyncs)} />
          <Diagnostic label="LAST PACKET" value={formatAge(data.diagnostics.telemetry.lastPacketAgeMs)} />
          <Diagnostic label="UPTIME" value={formatUptime(data.diagnostics.uptimeMs)} />
        </div>
        <div class="path-readout"><span>APPLICATION DATA</span><code>{data.dataDirectory}</code></div>
      </section>
    </div>
  );
}

function Toggle({
  checked,
  label,
  detail,
  disabled,
  onChange,
}: {
  checked: boolean;
  label: string;
  detail: string;
  disabled: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div class="toggle-row">
      <div><strong>{label}</strong><p>{detail}</p></div>
      <button type="button" role="switch" aria-checked={checked} disabled={disabled} onClick={() => onChange(!checked)}><i /></button>
    </div>
  );
}

function Diagnostic({ label, value, tone }: { label: string; value: string; tone?: "good" | "bad" }) {
  return <div class="diagnostic" data-tone={tone}><small>{label}</small><strong>{value}</strong></div>;
}

function recognitionRate(diagnostics: DesktopBootstrap["diagnostics"]): string {
  const received = diagnostics.telemetry.packetsReceived;
  if (received === 0) return "—";
  return `${((diagnostics.telemetry.packetsRecognized / received) * 100).toFixed(1)}%`;
}

function mergeRuntime(
  current: DesktopBootstrap | null,
  snapshot: RuntimeSnapshot,
): DesktopBootstrap {
  return { ...snapshot, autostartEnabled: current?.autostartEnabled ?? false };
}

function errorText(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  return typeof reason === "string" ? reason : "发生了未知错误";
}

function initialSection(): Section {
  const requested = new URLSearchParams(window.location.search).get("section");
  return NAVIGATION.some((item) => item.id === requested) ? (requested as Section) : "overview";
}
