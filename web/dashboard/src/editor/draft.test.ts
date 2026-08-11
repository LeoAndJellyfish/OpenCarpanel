import { describe, expect, it } from "vitest";

import { DEFAULT_LAYOUT, cloneLayout } from "@opencarpanel/widget-sdk";

import { clearLayoutDraft, loadLayoutDraft, saveLayoutDraft } from "./draft";

class MemoryStorage {
  readonly values = new Map<string, string>();

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }

  removeItem(key: string): void {
    this.values.delete(key);
  }
}

describe("layout drafts", () => {
  it("round-trips a bounded layout draft and clears it explicitly", () => {
    const storage = new MemoryStorage();
    const layout = { ...cloneLayout(DEFAULT_LAYOUT), revision: 4 };
    expect(saveLayoutDraft(storage, layout, 123)).toBe(true);
    expect(loadLayoutDraft(storage, "default")).toEqual({
      baseRevision: 4,
      savedAt: 123,
      document: layout,
    });
    clearLayoutDraft(storage, "default");
    expect(loadLayoutDraft(storage, "default")).toBeUndefined();
  });

  it("discards malformed drafts instead of surfacing runtime errors", () => {
    const storage = new MemoryStorage();
    storage.setItem("opencarpanel.layout-draft.v1.default", "{broken");
    expect(loadLayoutDraft(storage, "default")).toBeUndefined();
    expect(storage.values.size).toBe(0);
  });
});
