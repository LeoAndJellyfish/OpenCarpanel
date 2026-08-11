import type { ServerMessage } from "./generated/server-message";
import { PROTOCOL_VERSION, SERVER_MESSAGE_TYPES } from "./generated/wire-metadata";

const serverMessageTypes = new Set<string>(SERVER_MESSAGE_TYPES);

export class ProtocolParseError extends Error {
  readonly code: "invalid_json" | "invalid_message" | "unsupported_version";

  constructor(
    code: "invalid_json" | "invalid_message" | "unsupported_version",
    message: string,
  ) {
    super(message);
    this.name = "ProtocolParseError";
    this.code = code;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function decodeJson(value: string): unknown {
  try {
    return JSON.parse(value) as unknown;
  } catch {
    throw new ProtocolParseError("invalid_json", "Host message is not valid JSON");
  }
}

export function parseServerMessage(input: unknown): ServerMessage {
  const value = typeof input === "string" ? decodeJson(input) : input;
  if (!isRecord(value)) {
    throw new ProtocolParseError("invalid_message", "Host message must be an object");
  }
  if (value.v !== PROTOCOL_VERSION) {
    throw new ProtocolParseError(
      "unsupported_version",
      `Host protocol version ${String(value.v)} is unsupported`,
    );
  }
  if (typeof value.type !== "string" || !serverMessageTypes.has(value.type)) {
    throw new ProtocolParseError("invalid_message", "Host message type is unsupported");
  }
  return value as ServerMessage;
}
