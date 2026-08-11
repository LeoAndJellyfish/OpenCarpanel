import {
  PROTOCOL_VERSION,
  parseServerMessage,
  type ClientMessage,
  type ServerMessage,
} from "@opencarpanel/widget-sdk";

const DEVICE_SESSION_KEY = "opencarpanel.device-session.v1";
const LAST_EVENT_SEQUENCE_KEY = "opencarpanel.last-event-sequence.v1";

export type ConnectionPhase =
  | "connecting"
  | "pairing_required"
  | "connected"
  | "disconnected"
  | "error";

export interface ConnectionView {
  readonly phase: ConnectionPhase;
  readonly detail: string;
}

type ViewObserver = (view: ConnectionView) => void;

function websocketUrl(location: Location): string {
  const protocol = location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${location.host}/api/v1/ws`;
}

function lastEventSequence(storage: Storage): number {
  const raw = storage.getItem(LAST_EVENT_SEQUENCE_KEY);
  if (raw === null) {
    return 0;
  }
  const value = Number(raw);
  return Number.isSafeInteger(value) && value >= 0 ? value : 0;
}

function helloMessage(
  pairingToken: string | undefined,
  deviceSession: string | undefined,
  sequence: number,
): ClientMessage | undefined {
  if (pairingToken) {
    return {
      v: PROTOCOL_VERSION,
      type: "hello",
      pairingToken,
      lastEventSeq: sequence,
      snapshotHz: 60,
    };
  }
  if (deviceSession) {
    return {
      v: PROTOCOL_VERSION,
      type: "hello",
      deviceSession,
      lastEventSeq: sequence,
      snapshotHz: 60,
    };
  }
  return undefined;
}

export class TelemetryConnection {
  readonly #observe: ViewObserver;
  #socket: WebSocket | undefined;
  #stopped = false;
  #terminalError = false;

  constructor(observe: ViewObserver) {
    this.#observe = observe;
  }

  start(pairingToken?: string): void {
    const deviceSession = window.localStorage.getItem(DEVICE_SESSION_KEY) ?? undefined;
    const hello = helloMessage(
      pairingToken,
      deviceSession,
      lastEventSequence(window.localStorage),
    );
    if (!hello) {
      this.#observe({
        phase: "pairing_required",
        detail: "请从电脑端扫描配对二维码后重新打开此页面。",
      });
      return;
    }

    this.#observe({ phase: "connecting", detail: "正在建立低延迟遥测连接…" });
    const socket = new WebSocket(websocketUrl(window.location));
    this.#socket = socket;
    socket.addEventListener("open", () => {
      socket.send(JSON.stringify(hello));
    });
    socket.addEventListener("message", (event) => this.#handleMessage(event));
    socket.addEventListener("error", () => {
      this.#terminalError = true;
      this.#observe({
        phase: "error",
        detail: "连接发生错误，请确认手机与电脑位于同一局域网。",
      });
    });
    socket.addEventListener("close", () => {
      if (!this.#stopped && !this.#terminalError) {
        this.#observe({ phase: "disconnected", detail: "遥测连接已断开。" });
      }
    });
  }

  stop(): void {
    this.#stopped = true;
    this.#socket?.close(1000, "view_closed");
    this.#socket = undefined;
  }

  #handleMessage(event: MessageEvent<unknown>): void {
    if (typeof event.data !== "string") {
      this.#terminalError = true;
      this.#observe({ phase: "error", detail: "Host 返回了不支持的二进制消息。" });
      this.#socket?.close(1003, "binary_message");
      return;
    }

    let message: ServerMessage;
    try {
      message = parseServerMessage(event.data);
    } catch {
      this.#terminalError = true;
      this.#observe({ phase: "error", detail: "Host 与仪表盘使用了不兼容的协议。" });
      this.#socket?.close(1002, "invalid_protocol");
      return;
    }
    this.#applyMessage(message);
  }

  #applyMessage(message: ServerMessage): void {
    switch (message.type) {
      case "hello":
        if (message.deviceSession) {
          window.localStorage.setItem(DEVICE_SESSION_KEY, message.deviceSession);
        }
        this.#observe({ phase: "connected", detail: `Host ${message.serverVersion} 已就绪。` });
        break;
      case "event": {
        window.localStorage.setItem(LAST_EVENT_SEQUENCE_KEY, String(message.seq));
        const acknowledgement = {
          v: PROTOCOL_VERSION,
          type: "event_ack",
          seq: message.seq,
        } satisfies ClientMessage;
        this.#socket?.send(JSON.stringify(acknowledgement));
        break;
      }
      case "error":
        this.#terminalError = true;
        this.#observe({ phase: "error", detail: message.message });
        break;
      case "resync_required":
        window.localStorage.setItem(LAST_EVENT_SEQUENCE_KEY, String(message.newestEventSeq));
        break;
      case "capabilities":
      case "snapshot":
      case "stale":
        break;
    }
  }
}
