import {
  type LayoutDocument,
  LayoutParseError,
  parseLayoutDocument,
} from "@opencarpanel/widget-sdk";

import { readDeviceSession } from "../connection/session";

const MAX_LAYOUT_BYTES = 256 * 1024;

export interface LayoutEnvelope {
  readonly document: LayoutDocument;
  readonly recovered: boolean;
}

export interface LayoutApiOptions {
  readonly fetcher?: typeof fetch;
  readonly session?: string;
  readonly storage?: Storage;
}

export class LayoutApiError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = "LayoutApiError";
    this.status = status;
    this.code = code;
  }
}

export class LayoutConflictError extends LayoutApiError {
  readonly current: LayoutDocument;

  constructor(message: string, current: LayoutDocument) {
    super(409, "revision_conflict", message);
    this.name = "LayoutConflictError";
    this.current = current;
  }
}

export async function loadLayout(
  layoutId: string,
  options: LayoutApiOptions = {},
): Promise<LayoutEnvelope> {
  return requestLayout(layoutId, undefined, options);
}

export async function saveLayout(
  document: LayoutDocument,
  options: LayoutApiOptions = {},
): Promise<LayoutEnvelope> {
  const body = JSON.stringify(document);
  if (new TextEncoder().encode(body).length > MAX_LAYOUT_BYTES) {
    throw new LayoutApiError(413, "layout_too_large", "布局文件超过 256 KB 限制。");
  }
  return requestLayout(document.id, body, options);
}

async function requestLayout(
  layoutId: string,
  body: string | undefined,
  options: LayoutApiOptions,
): Promise<LayoutEnvelope> {
  const session =
    options.session ?? readDeviceSession(options.storage ?? window.localStorage);
  if (!session) {
    throw new LayoutApiError(401, "device_session_required", "需要先与 Host 完成配对。");
  }
  const fetcher = options.fetcher ?? window.fetch.bind(window);
  const response = await fetcher(`/api/v1/layouts/${encodeURIComponent(layoutId)}`, {
    method: body === undefined ? "GET" : "PUT",
    headers: {
      Authorization: `Bearer ${session}`,
      ...(body === undefined ? {} : { "Content-Type": "application/json" }),
    },
    ...(body === undefined ? {} : { body }),
  });

  const value: unknown = await response.json().catch(() => undefined);
  if (response.ok) {
    return parseEnvelope(value);
  }
  const error = objectValue(value);
  const code = typeof error?.code === "string" ? error.code : "layout_request_failed";
  const message =
    typeof error?.message === "string" ? error.message : `布局请求失败（HTTP ${response.status}）。`;
  if (response.status === 409 && code === "revision_conflict") {
    const current = parseEnvelope(error?.current).document;
    throw new LayoutConflictError(message, current);
  }
  throw new LayoutApiError(response.status, code, message);
}

function parseEnvelope(value: unknown): LayoutEnvelope {
  const envelope = objectValue(value);
  if (!envelope || typeof envelope.recovered !== "boolean") {
    throw new LayoutParseError("layout response envelope is invalid");
  }
  return {
    document: parseLayoutDocument(envelope.document),
    recovered: envelope.recovered,
  };
}

function objectValue(value: unknown): Record<string, unknown> | undefined {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}
