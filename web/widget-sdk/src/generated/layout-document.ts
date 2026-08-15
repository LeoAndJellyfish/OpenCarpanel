/**
 * Generated from the committed OpenSimDash JSON Schemas.
 * Do not edit by hand; run `npm run generate:web-types`.
 */

/**
 * Validated layout identifier.
 *
 * This interface was referenced by `LayoutDocument`'s JSON-Schema
 * via the `definition` "LayoutId".
 */
export type LayoutId = string;
/**
 * Validated dotted built-in component type.
 *
 * This interface was referenced by `LayoutDocument`'s JSON-Schema
 * via the `definition` "ComponentType".
 */
export type ComponentType = string;
/**
 * Validated widget instance identifier.
 *
 * This interface was referenced by `LayoutDocument`'s JSON-Schema
 * via the `definition` "InstanceId".
 */
export type InstanceId = string;

/**
 * Versioned responsive dashboard document.
 */
export interface LayoutDocument {
  id: LayoutId;
  name: string;
  revision: number;
  schemaVersion: number;
  theme?: ThemeSettings;
  widgets?: WidgetInstance[];
  [k: string]: unknown;
}
/**
 * Safe theme tokens exposed to built-in widgets.
 */
export interface ThemeSettings {
  /**
   * Accent CSS color token.
   */
  accent?: string;
  /**
   * Dashboard background CSS color token.
   */
  background?: string;
  /**
   * Primary foreground CSS color token.
   */
  foreground?: string;
  /**
   * Warning CSS color token.
   */
  warning?: string;
  [k: string]: unknown;
}
/**
 * One built-in widget instance in a layout.
 *
 * This interface was referenced by `LayoutDocument`'s JSON-Schema
 * via the `definition` "WidgetInstance".
 */
export interface WidgetInstance {
  /**
   * Registered built-in component type, such as `core.tachometer`.
   */
  componentType: string;
  /**
   * Stable instance id used by editor operations.
   */
  instanceId: string;
  /**
   * Breakpoint-specific geometry.
   */
  placements?: {
    [k: string]: GridPlacement;
  };
  /**
   * Component settings validated again by the component-specific schema.
   */
  settings?: {
    [k: string]: unknown;
  };
  [k: string]: unknown;
}
/**
 * Integer geometry for one widget at one breakpoint.
 *
 * This interface was referenced by `LayoutDocument`'s JSON-Schema
 * via the `definition` "GridPlacement".
 */
export interface GridPlacement {
  /**
   * Height in grid rows.
   */
  height: number;
  /**
   * Width in grid columns.
   */
  width: number;
  /**
   * Zero-based grid column.
   */
  x: number;
  /**
   * Zero-based grid row.
   */
  y: number;
  [k: string]: unknown;
}
/**
 * Safe theme tokens exposed to built-in widgets.
 *
 * This interface was referenced by `LayoutDocument`'s JSON-Schema
 * via the `definition` "ThemeSettings".
 */
export interface ThemeSettings1 {
  /**
   * Accent CSS color token.
   */
  accent?: string;
  /**
   * Dashboard background CSS color token.
   */
  background?: string;
  /**
   * Primary foreground CSS color token.
   */
  foreground?: string;
  /**
   * Warning CSS color token.
   */
  warning?: string;
  [k: string]: unknown;
}
