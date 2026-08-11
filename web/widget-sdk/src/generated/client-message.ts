/**
 * Generated from the committed OpenCarpanel JSON Schemas.
 * Do not edit by hand; run `npm run generate:web-types`.
 */

/**
 * Message sent from a dashboard client to the Host.
 */
export type ClientMessage = {
  /**
   * Wire protocol major version.
   */
  v: 1;
  [k: string]: unknown;
} & (
  | (ClientHello & {
      type: "hello";
      [k: string]: unknown;
    })
  | (EventAckMessage & {
      type: "event_ack";
      [k: string]: unknown;
    })
);

/**
 * First message sent by a dashboard client.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "ClientHello".
 */
export interface ClientHello {
  /**
   * Previously issued device session used for reconnects.
   */
  deviceSession?: string | null;
  /**
   * Last reliable event sequence consumed by the client.
   */
  lastEventSeq?: number | null;
  /**
   * One-time token copied from the QR-code fragment.
   */
  pairingToken?: string | null;
  /**
   * Requested snapshot publication frequency.
   */
  snapshotHz: number;
  [k: string]: unknown;
}
/**
 * Acknowledges one reliable event sequence.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "EventAckMessage".
 */
export interface EventAckMessage {
  /**
   * Highest contiguous event sequence consumed by the client.
   */
  seq: number;
  [k: string]: unknown;
}
