export type {
  ClientHello,
  ClientMessage,
  EventAckMessage,
} from "./generated/client-message";
export type {
  CapabilitiesMessage,
  ErrorMessage,
  EventMessage,
  DrsState,
  Gear,
  ServerHello,
  ServerMessage,
  SnapshotMessage,
  TelemetryEvent,
  TelemetryField,
  TelemetrySnapshot,
} from "./generated/server-message";
export {
  CLIENT_MESSAGE_TYPES,
  PROTOCOL_VERSION,
  SERVER_MESSAGE_TYPES,
} from "./generated/wire-metadata";
export { ProtocolParseError, parseServerMessage } from "./protocol";
export {
  WIDGET_MANIFEST_VERSION,
  defineWidgetManifest,
  type WidgetGridSize,
  type WidgetManifest,
  type WidgetType,
} from "./manifest";
export {
  WidgetRegistry,
  type WidgetRegistration,
} from "./registry";
