import { describe, expect, it } from "vitest";

import eventFixture from "./fixtures/server-event.json";
import helloFixture from "./fixtures/server-hello.json";
import snapshotFixture from "./fixtures/server-snapshot.json";
import { ProtocolParseError, parseServerMessage } from "./protocol";

describe("generated wire protocol", () => {
  it("narrows Rust-produced hello, snapshot, and event fixtures", () => {
    const hello = parseServerMessage(helloFixture);
    const snapshot = parseServerMessage(snapshotFixture);
    const event = parseServerMessage(eventFixture);

    expect(hello.type).toBe("hello");
    if (hello.type === "hello") {
      expect(hello.deviceSession).toBe("fixture-device-session");
    }

    expect(snapshot.type).toBe("snapshot");
    if (snapshot.type === "snapshot") {
      expect(snapshot.data.vehicle?.rpm).toBeUndefined();
      expect(snapshot.data.extensions).toBeUndefined();
    }

    expect(event.type).toBe("event");
    if (event.type === "event") {
      expect(event.seq).toBe(8);
      expect(event.data.name).toBe("lap.completed");
    }
  });

  it("rejects an unknown protocol version before payload use", () => {
    expect(() => parseServerMessage({ ...helloFixture, v: 99 })).toThrowError(
      expect.objectContaining<Partial<ProtocolParseError>>({ code: "unsupported_version" }),
    );
  });

  it("rejects malformed JSON and unknown message tags", () => {
    expect(() => parseServerMessage("{")).toThrowError(ProtocolParseError);
    expect(() => parseServerMessage({ v: 1, type: "future_message" })).toThrowError(
      expect.objectContaining<Partial<ProtocolParseError>>({ code: "invalid_message" }),
    );
  });
});
