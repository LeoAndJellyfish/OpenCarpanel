import { invoke, isTauri } from "@tauri-apps/api/core";

export interface HostSettings {
  schemaVersion: number;
  udpBind: string;
  httpBind: string;
  snapshotHz: 20 | 30 | 60;
  adapterSelection: "auto" | "f1-24" | "f1-25" | "ets2" | "ats";
}

export interface DesktopSettings {
  closeToTray: boolean;
  launchAtLogin: boolean;
  notificationsEnabled: boolean;
  automaticUpdates: boolean;
  onboardingComplete: boolean;
}

export interface AppSettings {
  schemaVersion: number;
  host: HostSettings;
  desktop: DesktopSettings;
}

export interface AdapterDiagnostics {
  id: string;
  displayName: string;
  protocolVersion: string;
  capabilities: string[];
  packetsRecognized: number;
  lastPacketAgeMs: number | null;
}

export interface HostDiagnostics {
  status: string;
  version: string;
  protocolVersion: number;
  adapter: string;
  adapterSelection: string;
  activeAdapter: string | null;
  supportedAdapters: AdapterDiagnostics[];
  uptimeMs: number;
  telemetry: {
    packetsReceived: number;
    packetsRecognized: number;
    packetErrors: number;
    lastPacketAgeMs: number | null;
    snapshotsPublished: number;
    eventResyncs: number;
  };
  connections: {
    active: number;
    limit: number;
  };
}

export interface PairedDevice {
  id: string;
  name: string;
  pairedAtUnixMs: number;
  lastSeenUnixMs: number;
}

export interface RuntimeSnapshot {
  version: string;
  settings: AppSettings;
  diagnostics: HostDiagnostics;
  endpoints: {
    dashboardUrl: string;
    editorUrl: string;
    diagnosticsUrl: string;
    httpAddress: string;
    udpAddress: string;
  };
  devices: PairedDevice[];
  recovery: {
    recovered: boolean;
    resetToDefaults: boolean;
    quarantinedPath: string | null;
  };
  dataDirectory: string;
  trayAvailable: boolean;
}

export interface DesktopBootstrap extends RuntimeSnapshot {
  autostartEnabled: boolean;
}

export interface PairingTicket {
  url: string;
  qrSvg: string;
  expiresInSeconds: number;
}

export interface ScsPluginStatus {
  game: "ets2" | "ats";
  gameDirectory: string;
  pluginPath: string;
  state: "missing" | "current" | "outdated";
  bundledSha256: string;
  installedSha256: string | null;
}

export interface UpdateInfo {
  available: boolean;
  currentVersion: string;
  version: string | null;
  notes: string | null;
  publishedAt: string | null;
}

const demoQr = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18"><rect width="18" height="18" fill="#f4f7f1"/><g fill="#07090d"><path d="M1 1h6v6H1zm1 1v4h4V2zm9-1h6v6h-6zm1 1v4h4V2zM1 11h6v6H1zm1 1v4h4v-4z"/><path d="M9 9h2v2H9zm3 0h1v1h-1zm2 0h3v2h-1v-1h-2zm-6 3h2v1H9v2H8zm3 0h2v2h-1v2h-2v-1h1zm3 1h3v1h-1v3h-2v-1h1v-1h-1z"/></g></svg>`;

let demoState: DesktopBootstrap = {
  version: "0.2.0",
  autostartEnabled: false,
  settings: {
    schemaVersion: 1,
    host: {
      schemaVersion: 1,
      udpBind: "0.0.0.0:20777",
      httpBind: "0.0.0.0:20778",
      snapshotHz: 60,
      adapterSelection: "auto",
    },
    desktop: {
      closeToTray: true,
      launchAtLogin: false,
      notificationsEnabled: true,
      automaticUpdates: true,
      onboardingComplete: false,
    },
  },
  diagnostics: {
    status: "ok",
    version: "0.2.0",
    protocolVersion: 1,
    adapter: "f1-25",
    adapterSelection: "auto",
    activeAdapter: "f1-25",
    supportedAdapters: [
      {
        id: "f1-24",
        displayName: "EA Sports F1 24",
        protocolVersion: "2024/v27.2x",
        capabilities: ["speed", "gear", "rpm", "throttle", "brake", "drs"],
        packetsRecognized: 0,
        lastPacketAgeMs: null,
      },
      {
        id: "f1-25",
        displayName: "EA Sports F1 25",
        protocolVersion: "2025/v3 + 2026/v10",
        capabilities: ["speed", "gear", "rpm", "throttle", "brake", "drs"],
        packetsRecognized: 18_426,
        lastPacketAgeMs: 12,
      },
      {
        id: "ets2",
        displayName: "Euro Truck Simulator 2",
        protocolVersion: "SCS bridge/v1",
        capabilities: ["speed", "gear", "rpm", "throttle", "brake"],
        packetsRecognized: 0,
        lastPacketAgeMs: null,
      },
      {
        id: "ats",
        displayName: "American Truck Simulator",
        protocolVersion: "SCS bridge/v1",
        capabilities: ["speed", "gear", "rpm", "throttle", "brake"],
        packetsRecognized: 0,
        lastPacketAgeMs: null,
      },
    ],
    uptimeMs: 2_742_000,
    telemetry: {
      packetsReceived: 18_426,
      packetsRecognized: 18_426,
      packetErrors: 0,
      lastPacketAgeMs: 12,
      snapshotsPublished: 18_426,
      eventResyncs: 0,
    },
    connections: { active: 1, limit: 8 },
  },
  endpoints: {
    dashboardUrl: "http://192.168.31.42:20778",
    editorUrl: "http://192.168.31.42:20778/edit",
    diagnosticsUrl: "http://192.168.31.42:20778/api/v1/diagnostics",
    httpAddress: "0.0.0.0:20778",
    udpAddress: "0.0.0.0:20777",
  },
  devices: [
    {
      id: "demo-ipad",
      name: "iPad · Safari",
      pairedAtUnixMs: Date.now() - 86_400_000,
      lastSeenUnixMs: Date.now() - 4_000,
    },
  ],
  recovery: {
    recovered: false,
    resetToDefaults: false,
    quarantinedPath: null,
  },
  dataDirectory: "C:\\Users\\Driver\\AppData\\Local\\OpenCarpanel",
  trayAvailable: true,
};

function cloneDemo(): DesktopBootstrap {
  return structuredClone(demoState);
}

export async function bootstrap(): Promise<DesktopBootstrap> {
  return isTauri() ? invoke<DesktopBootstrap>("bootstrap") : cloneDemo();
}

export async function refreshRuntime(): Promise<RuntimeSnapshot> {
  if (isTauri()) {
    return invoke<RuntimeSnapshot>("refresh_runtime");
  }
  demoState.diagnostics.uptimeMs += 800;
  demoState.diagnostics.telemetry.packetsReceived += 48;
  demoState.diagnostics.telemetry.packetsRecognized += 48;
  demoState.diagnostics.telemetry.snapshotsPublished += 48;
  demoState.diagnostics.telemetry.lastPacketAgeMs = 12;
  const active = demoState.diagnostics.supportedAdapters.find(
    (adapter) => adapter.id === demoState.diagnostics.activeAdapter,
  );
  if (active) {
    active.packetsRecognized += 48;
    active.lastPacketAgeMs = 12;
  }
  const { autostartEnabled: _autostart, ...runtime } = cloneDemo();
  return runtime;
}

export async function createPairing(): Promise<PairingTicket> {
  if (isTauri()) {
    return invoke<PairingTicket>("create_pairing");
  }
  return {
    url: `${demoState.endpoints.dashboardUrl}/#pair=preview-token`,
    qrSvg: demoQr,
    expiresInSeconds: 600,
  };
}

export async function revokeDevice(deviceId: string): Promise<RuntimeSnapshot> {
  if (isTauri()) {
    return invoke<RuntimeSnapshot>("revoke_device", { deviceId });
  }
  demoState.devices = demoState.devices.filter((device) => device.id !== deviceId);
  const { autostartEnabled: _autostart, ...runtime } = cloneDemo();
  return runtime;
}

export async function saveSettings(settings: AppSettings): Promise<DesktopBootstrap> {
  if (isTauri()) {
    return invoke<DesktopBootstrap>("save_settings", { settings });
  }
  demoState.settings = structuredClone(settings);
  demoState.autostartEnabled = settings.desktop.launchAtLogin;
  return cloneDemo();
}

export async function openDashboard(
  target: "dashboard" | "editor" | "diagnostics",
): Promise<void> {
  if (isTauri()) {
    await invoke("open_dashboard", { target });
  }
}

export async function openLogs(): Promise<void> {
  if (isTauri()) {
    await invoke("open_logs");
  }
}

export async function chooseScsDirectory(game: "ets2" | "ats"): Promise<ScsPluginStatus | null> {
  return isTauri()
    ? invoke<ScsPluginStatus | null>("choose_scs_directory", { game })
    : {
        game,
        gameDirectory: `C:\\Program Files (x86)\\Steam\\steamapps\\common\\${game === "ets2" ? "Euro Truck Simulator 2" : "American Truck Simulator"}`,
        pluginPath: `C:\\Program Files (x86)\\Steam\\steamapps\\common\\${game === "ets2" ? "Euro Truck Simulator 2" : "American Truck Simulator"}\\bin\\win_x64\\plugins\\opencarpanel-scs-telemetry.dll`,
        state: "missing",
        bundledSha256: "4b5b1c8f6f4de25e9ec83c19df438f622474dcb1ad8ff19ee728b74b5f45c3b1",
        installedSha256: null,
      };
}

export async function installScsPlugin(status: ScsPluginStatus): Promise<ScsPluginStatus> {
  if (isTauri()) {
    return invoke<ScsPluginStatus>("install_scs_plugin", {
      game: status.game,
      selectedDirectory: status.gameDirectory,
    });
  }
  return { ...status, state: "current", installedSha256: status.bundledSha256 };
}

export async function checkForUpdates(): Promise<UpdateInfo> {
  return isTauri()
    ? invoke<UpdateInfo>("check_for_updates")
    : {
        available: false,
        currentVersion: demoState.version,
        version: null,
        notes: null,
        publishedAt: null,
      };
}

export async function installUpdate(): Promise<void> {
  if (isTauri()) {
    await invoke("install_update");
  }
}
