import type { WidgetManifest, WidgetType } from "./manifest";

export interface WidgetRegistration<Implementation> {
  readonly manifest: WidgetManifest<object>;
  readonly implementation: Implementation;
}

export class WidgetRegistry<Implementation> {
  readonly #registrations = new Map<WidgetType, WidgetRegistration<Implementation>>();

  register(registration: WidgetRegistration<Implementation>): void {
    const type = registration.manifest.type;
    if (this.#registrations.has(type)) {
      throw new Error(`widget type ${type} is already registered`);
    }
    this.#registrations.set(type, registration);
  }

  get(type: WidgetType): WidgetRegistration<Implementation> | undefined {
    return this.#registrations.get(type);
  }

  list(): readonly WidgetRegistration<Implementation>[] {
    return [...this.#registrations.values()].sort((left, right) =>
      left.manifest.type.localeCompare(right.manifest.type),
    );
  }
}
