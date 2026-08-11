import {
  type LayoutDocument,
  LayoutParseError,
  parseLayoutDocument,
} from "@opencarpanel/widget-sdk";

const DRAFT_PREFIX = "opencarpanel.layout-draft.v1.";

export interface LayoutDraft {
  readonly baseRevision: number;
  readonly savedAt: number;
  readonly document: LayoutDocument;
}

export interface DraftStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export function loadLayoutDraft(
  storage: DraftStorage,
  layoutId: string,
): LayoutDraft | undefined {
  const raw = storage.getItem(draftKey(layoutId));
  if (raw === null) {
    return undefined;
  }
  try {
    const value = JSON.parse(raw) as Record<string, unknown>;
    if (!Number.isSafeInteger(value.baseRevision) || !Number.isFinite(value.savedAt)) {
      throw new LayoutParseError("invalid draft metadata");
    }
    const document = parseLayoutDocument(value.document);
    if (document.id !== layoutId) {
      throw new LayoutParseError("draft layout id does not match its storage key");
    }
    return {
      baseRevision: value.baseRevision as number,
      savedAt: value.savedAt as number,
      document,
    };
  } catch {
    storage.removeItem(draftKey(layoutId));
    return undefined;
  }
}

export function saveLayoutDraft(
  storage: DraftStorage,
  document: LayoutDocument,
  savedAt = Date.now(),
): boolean {
  try {
    const draft: LayoutDraft = {
      baseRevision: document.revision,
      savedAt,
      document,
    };
    storage.setItem(draftKey(document.id), JSON.stringify(draft));
    return true;
  } catch {
    return false;
  }
}

export function clearLayoutDraft(storage: DraftStorage, layoutId: string): void {
  storage.removeItem(draftKey(layoutId));
}

function draftKey(layoutId: string): string {
  return `${DRAFT_PREFIX}${layoutId}`;
}
