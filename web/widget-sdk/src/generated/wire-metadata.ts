/**
 * Generated from the committed OpenCarpanel JSON Schemas.
 * Do not edit by hand; run `npm run generate:web-types`.
 */
export const PROTOCOL_VERSION = 1 as const;
export const CLIENT_MESSAGE_TYPES = ["hello", "event_ack", "snapshot_request"] as const;
export const SERVER_MESSAGE_TYPES = ["hello", "snapshot", "event", "capabilities", "resync_required", "stale", "error"] as const;
