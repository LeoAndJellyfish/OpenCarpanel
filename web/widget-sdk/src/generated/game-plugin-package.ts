/**
 * Generated from the committed OpenSimDash JSON Schemas.
 * Do not edit by hand; run `npm run generate:web-types`.
 */

/**
 * Stable fields that an adapter can advertise to clients.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "TelemetryField".
 */
export type TelemetryField =
  | "vehicle.speedMps"
  | "vehicle.gear"
  | "vehicle.rpm"
  | "vehicle.rpmMax"
  | "vehicle.revLights"
  | "vehicle.throttle"
  | "vehicle.brake"
  | "vehicle.drs"
  | "vehicle.fuel"
  | "vehicle.pitLimiter"
  | "lap.current"
  | "lap.position"
  | "lap.currentTimeMs"
  | "lap.lastTimeMs"
  | "lap.deltaToBestMs"
  | "lap.invalid"
  | "lap.raceState"
  | "session.trackId"
  | "session.remainingTimeMs"
  | "session.totalLaps"
  | "session.raceState"
  | "tyres"
  | "conditions"
  | "damage"
  | "aero"
  | "navigation"
  | "lights"
  | "job";
/**
 * High-level dashboard visual family.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "PluginGameFamily".
 */
export type PluginGameFamily = "formula" | "truck" | "generic";
/**
 * Supported plugin ingress transports.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "PluginIngressKind".
 */
export type PluginIngressKind = "shared_udp";
/**
 * Responsive layout template selected by a plugin.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "PluginLayoutPreset".
 */
export type PluginLayoutPreset = "formula" | "truck" | "generic";
/**
 * Decoder implementation referenced by a manifest.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "PluginRuntime".
 */
export type PluginRuntime =
  | {
      /**
       * Factory entrypoint resolved by the built-in registry.
       */
      entrypoint: string;
      kind: "builtin";
      [k: string]: unknown;
    }
  | {
      /**
       * Stable ABI required by the module.
       */
      abiVersion: number;
      kind: "wasm";
      /**
       * Safe package-relative module filename.
       */
      module: string;
      /**
       * Lowercase SHA-256 of the decoded module bytes.
       */
      sha256: string;
      [k: string]: unknown;
    };
/**
 * Declarative setup workflow rendered by the desktop application.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "PluginSetup".
 */
export type PluginSetup =
  | {
      /**
       * Format label the user selects in game.
       */
      format: string;
      kind: "f1_udp";
      /**
       * Recommended game send rate.
       */
      sendRateHz: number;
      [k: string]: unknown;
    }
  | {
      /**
       * Expected Steam install directory name.
       */
      directoryName: string;
      kind: "scs_sdk";
      /**
       * Steam application id used for discovery.
       */
      steamAppId: number;
      [k: string]: unknown;
    }
  | {
      kind: "udp";
      /**
       * Ordered concise configuration steps.
       */
      steps: string[];
      [k: string]: unknown;
    }
  | {
      kind: "none";
      [k: string]: unknown;
    };
/**
 * Built-in status widget semantics.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "PluginStatusMode".
 */
export type PluginStatusMode = "drs" | "scs" | "generic";

/**
 * Single-file distributable plugin package.
 */
export interface GamePluginPackage {
  manifest: GamePluginManifest;
  /**
   * Base64-encoded WASM module bytes.
   */
  moduleBase64: string;
  /**
   * Package envelope version.
   */
  packageVersion: number;
}
/**
 * Embedded external-plugin manifest.
 */
export interface GamePluginManifest {
  /**
   * Canonical telemetry paths this decoder can produce.
   */
  capabilities: TelemetryField[];
  /**
   * Concise purpose and compatibility summary.
   */
  description: string;
  /**
   * Stable lowercase game/plugin identifier.
   */
  id: string;
  ingress: PluginIngress;
  /**
   * SPDX license expression or short license identifier.
   */
  license: string;
  /**
   * Product-facing source name.
   */
  name: string;
  presentation: PluginPresentation;
  protocol: PluginProtocol;
  /**
   * Publisher shown before local installation.
   */
  publisher: string;
  /**
   * Decoder implementation selected by the Host.
   */
  runtime:
    | {
        /**
         * Factory entrypoint resolved by the built-in registry.
         */
        entrypoint: string;
        kind: "builtin";
        [k: string]: unknown;
      }
    | {
        /**
         * Stable ABI required by the module.
         */
        abiVersion: number;
        kind: "wasm";
        /**
         * Safe package-relative module filename.
         */
        module: string;
        /**
         * Lowercase SHA-256 of the decoded module bytes.
         */
        sha256: string;
        [k: string]: unknown;
      };
  /**
   * Manifest schema version.
   */
  schemaVersion: number;
  /**
   * Declarative setup workflow for the desktop control center.
   */
  setup:
    | {
        /**
         * Format label the user selects in game.
         */
        format: string;
        kind: "f1_udp";
        /**
         * Recommended game send rate.
         */
        sendRateHz: number;
        [k: string]: unknown;
      }
    | {
        /**
         * Expected Steam install directory name.
         */
        directoryName: string;
        kind: "scs_sdk";
        /**
         * Steam application id used for discovery.
         */
        steamAppId: number;
        [k: string]: unknown;
      }
    | {
        kind: "udp";
        /**
         * Ordered concise configuration steps.
         */
        steps: string[];
        [k: string]: unknown;
      }
    | {
        kind: "none";
        [k: string]: unknown;
      };
  /**
   * Plugin implementation semantic version.
   */
  version: string;
}
/**
 * Host-owned input transport declaration.
 */
export interface PluginIngress {
  /**
   * Suggested port when configuring the game or producer.
   */
  defaultPort: number;
  /**
   * v1 transport kind.
   */
  kind: "shared_udp";
  /**
   * Decoder-specific maximum input size.
   */
  maxDatagramBytes: number;
}
/**
 * Trusted Dashboard presentation configuration.
 */
export interface PluginPresentation {
  /**
   * Secondary label rendered next to the game name.
   */
  detail: string;
  /**
   * Safe fallback for games that do not report maximum RPM.
   */
  fallbackRpmMax: number;
  /**
   * Broad visual family.
   */
  family: "formula" | "truck" | "generic";
  /**
   * Built-in responsive placement template.
   */
  layoutPreset: "formula" | "truck" | "generic";
  /**
   * Compact tab/status label.
   */
  shortName: string;
  /**
   * Semantics used by the built-in status widget.
   */
  statusMode: "drs" | "scs" | "generic";
  theme: PluginTheme;
  /**
   * Trusted built-in widget types offered for this game.
   */
  widgets: string[];
}
/**
 * Theme values applied to a new per-game layout.
 */
export interface PluginTheme {
  /**
   * Game accent color.
   */
  accent: string;
  /**
   * Dashboard background.
   */
  background: string;
  /**
   * Primary text and marks.
   */
  foreground: string;
  /**
   * Warning color.
   */
  warning: string;
}
/**
 * Human-readable upstream wire protocol information.
 */
export interface PluginProtocol {
  /**
   * Protocol family, such as `EA UDP` or `SCS bridge`.
   */
  name: string;
  /**
   * Accepted upstream protocol versions.
   */
  version: string;
}
/**
 * Complete source-of-truth declaration for one supported game or telemetry producer.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "GamePluginManifest".
 */
export interface GamePluginManifest1 {
  /**
   * Canonical telemetry paths this decoder can produce.
   */
  capabilities: TelemetryField[];
  /**
   * Concise purpose and compatibility summary.
   */
  description: string;
  /**
   * Stable lowercase game/plugin identifier.
   */
  id: string;
  ingress: PluginIngress;
  /**
   * SPDX license expression or short license identifier.
   */
  license: string;
  /**
   * Product-facing source name.
   */
  name: string;
  presentation: PluginPresentation;
  protocol: PluginProtocol;
  /**
   * Publisher shown before local installation.
   */
  publisher: string;
  /**
   * Decoder implementation selected by the Host.
   */
  runtime:
    | {
        /**
         * Factory entrypoint resolved by the built-in registry.
         */
        entrypoint: string;
        kind: "builtin";
        [k: string]: unknown;
      }
    | {
        /**
         * Stable ABI required by the module.
         */
        abiVersion: number;
        kind: "wasm";
        /**
         * Safe package-relative module filename.
         */
        module: string;
        /**
         * Lowercase SHA-256 of the decoded module bytes.
         */
        sha256: string;
        [k: string]: unknown;
      };
  /**
   * Manifest schema version.
   */
  schemaVersion: number;
  /**
   * Declarative setup workflow for the desktop control center.
   */
  setup:
    | {
        /**
         * Format label the user selects in game.
         */
        format: string;
        kind: "f1_udp";
        /**
         * Recommended game send rate.
         */
        sendRateHz: number;
        [k: string]: unknown;
      }
    | {
        /**
         * Expected Steam install directory name.
         */
        directoryName: string;
        kind: "scs_sdk";
        /**
         * Steam application id used for discovery.
         */
        steamAppId: number;
        [k: string]: unknown;
      }
    | {
        kind: "udp";
        /**
         * Ordered concise configuration steps.
         */
        steps: string[];
        [k: string]: unknown;
      }
    | {
        kind: "none";
        [k: string]: unknown;
      };
  /**
   * Plugin implementation semantic version.
   */
  version: string;
}
/**
 * Host-owned transport used to deliver bytes to a decoder.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "PluginIngress".
 */
export interface PluginIngress1 {
  /**
   * Suggested port when configuring the game or producer.
   */
  defaultPort: number;
  /**
   * v1 transport kind.
   */
  kind: "shared_udp";
  /**
   * Decoder-specific maximum input size.
   */
  maxDatagramBytes: number;
}
/**
 * Safe presentation values consumed by the trusted Dashboard.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "PluginPresentation".
 */
export interface PluginPresentation1 {
  /**
   * Secondary label rendered next to the game name.
   */
  detail: string;
  /**
   * Safe fallback for games that do not report maximum RPM.
   */
  fallbackRpmMax: number;
  /**
   * Broad visual family.
   */
  family: "formula" | "truck" | "generic";
  /**
   * Built-in responsive placement template.
   */
  layoutPreset: "formula" | "truck" | "generic";
  /**
   * Compact tab/status label.
   */
  shortName: string;
  /**
   * Semantics used by the built-in status widget.
   */
  statusMode: "drs" | "scs" | "generic";
  theme: PluginTheme;
  /**
   * Trusted built-in widget types offered for this game.
   */
  widgets: string[];
}
/**
 * Upstream game or software telemetry protocol label.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "PluginProtocol".
 */
export interface PluginProtocol1 {
  /**
   * Protocol family, such as `EA UDP` or `SCS bridge`.
   */
  name: string;
  /**
   * Accepted upstream protocol versions.
   */
  version: string;
}
/**
 * Hexadecimal colors used for a plugin's default layout.
 *
 * This interface was referenced by `GamePluginPackage`'s JSON-Schema
 * via the `definition` "PluginTheme".
 */
export interface PluginTheme1 {
  /**
   * Game accent color.
   */
  accent: string;
  /**
   * Dashboard background.
   */
  background: string;
  /**
   * Primary text and marks.
   */
  foreground: string;
  /**
   * Warning color.
   */
  warning: string;
}
