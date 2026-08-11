import { cloneLayout, DEFAULT_LAYOUT } from "@opencarpanel/widget-sdk";
import { describe, expect, it } from "vitest";

import {
  createLayoutExport,
  importLayoutText,
  MAX_LAYOUT_TRANSFER_BYTES,
} from "./layout-transfer";

describe("layout import and export", () => {
  it("exports readable versioned JSON and preserves the current Host identity on import", () => {
    const source = {
      ...cloneLayout(DEFAULT_LAYOUT),
      id: "shared-layout",
      revision: 999,
      name: "Shared race layout",
    };
    const exported = createLayoutExport(source);

    expect(exported.filename).toBe("opencarpanel-shared-layout.json");
    expect(exported.content.endsWith("\n")).toBe(true);
    const imported = importLayoutText(`\ufeff${exported.content}`, {
      id: "default",
      revision: 7,
    });
    expect(imported).toEqual({ ...source, id: "default", revision: 7 });
  });

  it("rejects oversized, malformed, future-version and unknown-component files", () => {
    expect(() =>
      importLayoutText(" ".repeat(MAX_LAYOUT_TRANSFER_BYTES + 1), DEFAULT_LAYOUT),
    ).toThrowError("256 KB");
    expect(() => importLayoutText("{bad", DEFAULT_LAYOUT)).toThrowError("有效的 JSON");
    expect(() =>
      importLayoutText(
        JSON.stringify({ ...DEFAULT_LAYOUT, schemaVersion: 2 }),
        DEFAULT_LAYOUT,
      ),
    ).toThrowError("unsupported layout schema version");
    expect(() =>
      importLayoutText(
        JSON.stringify({
          ...DEFAULT_LAYOUT,
          widgets: [
            { ...DEFAULT_LAYOUT.widgets[0], componentType: "community.untrusted" },
          ],
        }),
        DEFAULT_LAYOUT,
      ),
    ).toThrowError("不支持组件");
  });

  it("rejects executable-looking settings, invalid geometry and collisions", () => {
    const speed = DEFAULT_LAYOUT.widgets.find((widget) => widget.componentType === "core.speed")!;
    expect(() =>
      importLayoutText(
        JSON.stringify({
          ...DEFAULT_LAYOUT,
          widgets: [{ ...speed, settings: { unit: "<script>alert(1)</script>" } }],
        }),
        DEFAULT_LAYOUT,
      ),
    ).toThrowError("设置无效");

    expect(() =>
      importLayoutText(
        JSON.stringify({
          ...DEFAULT_LAYOUT,
          widgets: [
            {
              ...speed,
              placements: {
                ...speed.placements,
                phonePortrait: { x: 11, y: 0, width: 2, height: 2 },
              },
            },
          ],
        }),
        DEFAULT_LAYOUT,
      ),
    ).toThrowError("超出 phonePortrait 网格");

    const tachometer = DEFAULT_LAYOUT.widgets[0]!;
    const gear = DEFAULT_LAYOUT.widgets[1]!;
    expect(() =>
      importLayoutText(
        JSON.stringify({
          ...DEFAULT_LAYOUT,
          widgets: [
            tachometer,
            {
              ...gear,
              placements: {
                ...gear.placements,
                phonePortrait: tachometer.placements.phonePortrait,
              },
            },
          ],
        }),
        DEFAULT_LAYOUT,
      ),
    ).toThrowError("重叠");
  });
});
