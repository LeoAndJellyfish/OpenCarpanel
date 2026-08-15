import { describe, expect, it } from "vitest";

import { DEFAULT_LAYOUT } from "@opensimdash/widget-sdk";

import { LayoutConflictError, loadLayout, saveLayout } from "./layouts";

describe("layout API client", () => {
  it("authenticates and parses a generated-schema layout envelope", async () => {
    const calls: Array<{ input: RequestInfo | URL; init?: RequestInit }> = [];
    const fetcher: typeof fetch = async (input, init) => {
      calls.push({ input, ...(init === undefined ? {} : { init }) });
      return new Response(JSON.stringify({ document: DEFAULT_LAYOUT, recovered: false }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    };
    const result = await loadLayout("default", {
      fetcher,
      session: "paired-session",
    });
    expect(result.document).toEqual(DEFAULT_LAYOUT);
    expect(calls).toEqual([
      {
        input: "/api/v1/layouts/default",
        init: expect.objectContaining({
        method: "GET",
        headers: { Authorization: "Bearer paired-session" },
      }),
      },
    ]);
  });

  it("sends the document revision and exposes the current conflict document", async () => {
    const current = { ...DEFAULT_LAYOUT, revision: 9 };
    let capturedBody: BodyInit | null | undefined;
    const fetcher: typeof fetch = async (_input, init) => {
      capturedBody = init?.body;
      return new Response(
        JSON.stringify({
          code: "revision_conflict",
          message: "changed elsewhere",
          current: { document: current, recovered: false },
        }),
        { status: 409, headers: { "Content-Type": "application/json" } },
      );
    };
    const error = await saveLayout(DEFAULT_LAYOUT, {
      fetcher,
      session: "paired-session",
    }).catch((reason: unknown) => reason);
    expect(error).toBeInstanceOf(LayoutConflictError);
    expect((error as LayoutConflictError).current.revision).toBe(9);
    expect(JSON.parse(capturedBody as string).revision).toBe(0);
  });
});
