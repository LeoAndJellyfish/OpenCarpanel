import {
  PROTOCOL_VERSION,
  parseServerMessage,
  type ClientMessage,
  type ServerMessage,
} from "@opensimdash/widget-sdk";

import { readDeviceSession, writeDeviceSession } from "./session";

const LAST_EVENT_SEQUENCE_KEY = "opensimdash.last-event-sequence.v1";

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
type MessageObserver = (message: ServerMessage) => void;

const RETRY_DELAYS_MS = [250, 500, 1_000, 2_000, 5_000] as const;

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
    deviceName: string,
): ClientMessage | undefined {
  if (pairingToken) {
    return {
      v: PROTOCOL_VERSION,
      type: "hello",
      pairingToken,
      deviceName,
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
  readonly #observeMessage: MessageObserver;
  #socket: WebSocket | undefined;
  #pairingToken: string | undefined;
  #deviceSession: string | undefined;
  #retryTimer: number | undefined;
  #retryAttempt = 0;
  #stopped = false;
  #terminalError = false;

  constructor(observe: ViewObserver, observeMessage: MessageObserver = () => undefined) {
    this.#observe = observe;
    this.#observeMessage = observeMessage;
  }

  start(pairingToken?: string): void {
    this.#stopped = false;
    this.#terminalError = false;
    this.#pairingToken = pairingToken;
    this.#deviceSession = readDeviceSession(window.localStorage);
    const hello = this.#helloMessage();
    if (!hello) {
      this.#observe({
        phase: "pairing_required",
        detail: "请从电脑端扫描配对二维码后重新打开此页面。",
      });
      return;
    }

    this.#connect(hello);
  }

  requestSnapshot(): void {
    const request = {
      v: PROTOCOL_VERSION,
      type: "snapshot_request",
    } satisfies ClientMessage;
    this.#send(request);
  }

  stop(): void {
    this.#stopped = true;
    if (this.#retryTimer !== undefined) {
      window.clearTimeout(this.#retryTimer);
      this.#retryTimer = undefined;
    }
    this.#socket?.close(1000, "view_closed");
    this.#socket = undefined;
  }

  #helloMessage(): ClientMessage | undefined {
    return helloMessage(
      this.#pairingToken,
      this.#deviceSession,
      lastEventSequence(window.localStorage),
      dashboardDeviceName(window.navigator),
    );
  }

  #connect(hello: ClientMessage): void {
    this.#observe({
      phase: "connecting",
      detail: this.#retryAttempt === 0 ? "正在建立低延迟遥测连接…" : "正在重新连接 Host…",
    });
    const socket = new WebSocket(websocketUrl(window.location));
    this.#socket = socket;
    socket.addEventListener("open", () => {
      if (this.#socket === socket) {
        socket.send(JSON.stringify(hello));
      }
    });
    socket.addEventListener("message", (event) => this.#handleMessage(event, socket));
    socket.addEventListener("error", () => {
      if (this.#socket === socket && !this.#terminalError) {
        this.#observe({ phase: "disconnected", detail: "连接受阻，正在准备重试…" });
      }
    });
    socket.addEventListener("close", () => {
      if (this.#socket !== socket) {
        return;
      }
      this.#socket = undefined;
      if (!this.#stopped && !this.#terminalError) {
        this.#scheduleReconnect();
      }
    });
  }

  #scheduleReconnect(): void {
    const delayIndex = Math.min(this.#retryAttempt, RETRY_DELAYS_MS.length - 1);
    const delayMs = RETRY_DELAYS_MS[delayIndex] ?? 5_000;
    this.#retryAttempt += 1;
    this.#observe({
      phase: "disconnected",
      detail: `连接已断开，${(delayMs / 1_000).toFixed(delayMs < 1_000 ? 2 : 0)} 秒后重试。`,
    });
    this.#retryTimer = window.setTimeout(() => {
      this.#retryTimer = undefined;
      const hello = this.#helloMessage();
      if (hello && !this.#stopped) {
        this.#connect(hello);
      }
    }, delayMs);
  }

  #handleMessage(event: MessageEvent<unknown>, socket: WebSocket): void {
    if (this.#socket !== socket) {
      return;
    }
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
    this.#observeMessage(message);
  }

  #applyMessage(message: ServerMessage): void {
    switch (message.type) {
      case "hello":
        if (message.deviceSession) {
          writeDeviceSession(window.localStorage, message.deviceSession);
          this.#deviceSession = message.deviceSession;
        }
        this.#pairingToken = undefined;
        this.#retryAttempt = 0;
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
        this.#terminalError = !message.retryable;
        this.#observe({ phase: "error", detail: message.message });
        if (this.#terminalError) {
          this.#socket?.close(1008, "terminal_error");
        }
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

  #send(message: ClientMessage): void {
    const socket = this.#socket;
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify(message));
    }
  }
}

function dashboardDeviceName(navigator: Navigator): string {
  const userAgent = navigator.userAgent.toLowerCase();
  const device = userAgent.includes("ipad")
    || (userAgent.includes("macintosh") && navigator.maxTouchPoints > 1)
    ? "iPad"
    : userAgent.includes("iphone")
      ? "iPhone"
      : userAgent.includes("android")
        ? "Android"
        : "Browser";
  const browser = userAgent.includes("edg/")
    ? "Edge"
    : userAgent.includes("chrome/") || userAgent.includes("crios/")
      ? "Chrome"
      : userAgent.includes("safari/")
        ? "Safari"
        : "Web";
  return `${device} · ${browser}`;
}
