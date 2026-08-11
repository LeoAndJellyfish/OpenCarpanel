import {
  ALL_DASHBOARD_FIELDS,
  type DashboardField,
  type TelemetryStore,
} from "./store";

export interface FrameDriver {
  request(callback: FrameRequestCallback): number;
  cancel(handle: number): void;
}

export type RenderBinding = (store: TelemetryStore, nowMs: number) => void;

interface Binding {
  readonly fields: ReadonlySet<DashboardField>;
  readonly render: RenderBinding;
}

const browserFrameDriver: FrameDriver = {
  request: (callback) => window.requestAnimationFrame(callback),
  cancel: (handle) => window.cancelAnimationFrame(handle),
};

export class TelemetryRenderLoop {
  readonly #store: TelemetryStore;
  readonly #driver: FrameDriver;
  readonly #bindings = new Map<number, Binding>();
  readonly #pendingFields = new Set<DashboardField>();
  readonly #unsubscribeStore: () => void;
  #nextBindingId = 1;
  #frameHandle: number | undefined;
  #paused = false;

  constructor(store: TelemetryStore, driver: FrameDriver = browserFrameDriver) {
    this.#store = store;
    this.#driver = driver;
    this.#unsubscribeStore = store.subscribe(ALL_DASHBOARD_FIELDS, (changed) => {
      this.invalidate(changed);
    });
  }

  bind(fields: readonly DashboardField[], render: RenderBinding): () => void {
    const id = this.#nextBindingId;
    this.#nextBindingId += 1;
    this.#bindings.set(id, { fields: new Set(fields), render });
    this.invalidate(fields);
    return () => {
      this.#bindings.delete(id);
    };
  }

  invalidate(fields: Iterable<DashboardField>): void {
    for (const field of fields) {
      this.#pendingFields.add(field);
    }
    this.#schedule();
  }

  pause(): void {
    this.#paused = true;
    if (this.#frameHandle !== undefined) {
      this.#driver.cancel(this.#frameHandle);
      this.#frameHandle = undefined;
    }
  }

  resume(): void {
    this.#paused = false;
    this.invalidate(ALL_DASHBOARD_FIELDS);
  }

  attachVisibility(source: Document, onResume: () => void): () => void {
    const handleVisibility = () => {
      if (source.hidden) {
        this.pause();
        return;
      }
      this.#store.resetInterpolation(performance.now());
      this.resume();
      onResume();
    };
    source.addEventListener("visibilitychange", handleVisibility);
    if (source.hidden) {
      this.pause();
    }
    return () => source.removeEventListener("visibilitychange", handleVisibility);
  }

  destroy(): void {
    this.pause();
    this.#bindings.clear();
    this.#pendingFields.clear();
    this.#unsubscribeStore();
  }

  #schedule(): void {
    if (this.#paused || this.#frameHandle !== undefined || this.#bindings.size === 0) {
      return;
    }
    this.#frameHandle = this.#driver.request((nowMs) => this.#renderFrame(nowMs));
  }

  #renderFrame(nowMs: number): void {
    this.#frameHandle = undefined;
    const active = this.#store.activeContinuousFields(nowMs);
    const affected = new Set<DashboardField>(this.#pendingFields);
    this.#pendingFields.clear();
    for (const field of active) {
      affected.add(field);
    }

    for (const binding of this.#bindings.values()) {
      if (setsIntersect(binding.fields, affected)) {
        binding.render(this.#store, nowMs);
      }
    }

    if (active.size > 0) {
      for (const field of active) {
        this.#pendingFields.add(field);
      }
      this.#schedule();
    }
  }
}

function setsIntersect(
  left: ReadonlySet<DashboardField>,
  right: ReadonlySet<DashboardField>,
): boolean {
  for (const field of left) {
    if (right.has(field)) {
      return true;
    }
  }
  return false;
}
