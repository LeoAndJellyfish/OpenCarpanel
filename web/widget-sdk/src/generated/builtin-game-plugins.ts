/**
 * Generated from the committed OpenCarpanel JSON Schemas.
 * Do not edit by hand; run `npm run generate:web-types`.
 */
import type { GamePluginMetadata } from "./server-message";

export const BUILTIN_GAME_PLUGINS = [
  {
    "id": "ats",
    "name": "American Truck Simulator",
    "version": "1.0.0",
    "publisher": "OpenCarpanel contributors",
    "description": "American Truck Simulator telemetry through the bundled SCS SDK bridge.",
    "protocolVersion": "scs-bridge/v1+v2 (SDK 1.14)",
    "ingress": {
      "kind": "shared_udp",
      "defaultPort": 20777,
      "maxDatagramBytes": 188
    },
    "capabilities": [
      "vehicle.speedMps",
      "vehicle.gear",
      "vehicle.rpm",
      "vehicle.rpmMax",
      "vehicle.throttle",
      "vehicle.brake",
      "vehicle.drs",
      "vehicle.fuel",
      "navigation",
      "lights",
      "job"
    ],
    "presentation": {
      "shortName": "ATS",
      "detail": "INTERSTATE / SCS SDK",
      "family": "truck",
      "statusMode": "scs",
      "layoutPreset": "truck",
      "theme": {
        "background": "#080d10",
        "foreground": "#f5f0e6",
        "accent": "#ff6a3d",
        "warning": "#ffcf54"
      },
      "fallbackRpmMax": 2500,
      "widgets": [
        "core.gear",
        "core.route",
        "core.speed",
        "core.tachometer",
        "core.status"
      ]
    },
    "setup": {
      "kind": "scs_sdk",
      "steamAppId": 270880,
      "directoryName": "American Truck Simulator"
    },
    "source": "builtin"
  },
  {
    "id": "ets2",
    "name": "Euro Truck Simulator 2",
    "version": "1.0.0",
    "publisher": "OpenCarpanel contributors",
    "description": "Euro Truck Simulator 2 telemetry through the bundled SCS SDK bridge.",
    "protocolVersion": "scs-bridge/v1+v2 (SDK 1.14)",
    "ingress": {
      "kind": "shared_udp",
      "defaultPort": 20777,
      "maxDatagramBytes": 188
    },
    "capabilities": [
      "vehicle.speedMps",
      "vehicle.gear",
      "vehicle.rpm",
      "vehicle.rpmMax",
      "vehicle.throttle",
      "vehicle.brake",
      "vehicle.drs",
      "vehicle.fuel",
      "navigation",
      "lights",
      "job"
    ],
    "presentation": {
      "shortName": "ETS2",
      "detail": "LONG HAUL / SCS SDK",
      "family": "truck",
      "statusMode": "scs",
      "layoutPreset": "truck",
      "theme": {
        "background": "#0e0b08",
        "foreground": "#fff5e5",
        "accent": "#ffbd45",
        "warning": "#ff4b3e"
      },
      "fallbackRpmMax": 2500,
      "widgets": [
        "core.gear",
        "core.route",
        "core.speed",
        "core.tachometer",
        "core.status"
      ]
    },
    "setup": {
      "kind": "scs_sdk",
      "steamAppId": 227300,
      "directoryName": "Euro Truck Simulator 2"
    },
    "source": "builtin"
  },
  {
    "id": "f1-24",
    "name": "EA Sports F1 24",
    "version": "1.0.0",
    "publisher": "OpenCarpanel contributors",
    "description": "EA Sports F1 24 original 2024 UDP telemetry.",
    "protocolVersion": "2024/v27.2x",
    "ingress": {
      "kind": "shared_udp",
      "defaultPort": 20777,
      "maxDatagramBytes": 65507
    },
    "capabilities": [
      "vehicle.speedMps",
      "vehicle.gear",
      "vehicle.rpm",
      "vehicle.rpmMax",
      "vehicle.revLights",
      "vehicle.throttle",
      "vehicle.brake",
      "vehicle.drs",
      "vehicle.fuel",
      "vehicle.pitLimiter",
      "lap.current",
      "lap.position",
      "lap.currentTimeMs",
      "lap.lastTimeMs",
      "lap.invalid",
      "lap.raceState",
      "session.trackId",
      "session.remainingTimeMs",
      "session.totalLaps",
      "session.raceState",
      "tyres",
      "conditions",
      "damage"
    ],
    "presentation": {
      "shortName": "F1 24",
      "detail": "FORMULA / UDP 2024",
      "family": "formula",
      "statusMode": "drs",
      "layoutPreset": "formula",
      "theme": {
        "background": "#07090c",
        "foreground": "#f2f0e9",
        "accent": "#d9ff43",
        "warning": "#ff4b3e"
      },
      "fallbackRpmMax": 12000,
      "widgets": [
        "core.gear",
        "core.race",
        "core.speed",
        "core.tachometer",
        "core.tyres",
        "core.status"
      ]
    },
    "setup": {
      "kind": "f1_udp",
      "format": "F1 24 / 2024",
      "sendRateHz": 60
    },
    "source": "builtin"
  },
  {
    "id": "f1-25",
    "name": "EA Sports F1 25",
    "version": "1.0.0",
    "publisher": "OpenCarpanel contributors",
    "description": "EA Sports F1 25 original 2025 and 2026 Season Pack UDP telemetry.",
    "protocolVersion": "2025/v3 + 2026/v10",
    "ingress": {
      "kind": "shared_udp",
      "defaultPort": 20777,
      "maxDatagramBytes": 65507
    },
    "capabilities": [
      "vehicle.speedMps",
      "vehicle.gear",
      "vehicle.rpm",
      "vehicle.rpmMax",
      "vehicle.revLights",
      "vehicle.throttle",
      "vehicle.brake",
      "vehicle.drs",
      "vehicle.fuel",
      "vehicle.pitLimiter",
      "lap.current",
      "lap.position",
      "lap.currentTimeMs",
      "lap.lastTimeMs",
      "lap.invalid",
      "lap.raceState",
      "session.trackId",
      "session.remainingTimeMs",
      "session.totalLaps",
      "session.raceState",
      "tyres",
      "conditions",
      "damage",
      "aero"
    ],
    "presentation": {
      "shortName": "F1 25",
      "detail": "FORMULA / UDP 2025 + 2026",
      "family": "formula",
      "statusMode": "drs",
      "layoutPreset": "formula",
      "theme": {
        "background": "#061015",
        "foreground": "#eefcff",
        "accent": "#42e8ff",
        "warning": "#ff5e6c"
      },
      "fallbackRpmMax": 12000,
      "widgets": [
        "core.gear",
        "core.race",
        "core.speed",
        "core.tachometer",
        "core.tyres",
        "core.status"
      ]
    },
    "setup": {
      "kind": "f1_udp",
      "format": "F1 25 / 2025 或 2026 Season Pack",
      "sendRateHz": 60
    },
    "source": "builtin"
  }
] as const satisfies readonly GamePluginMetadata[];
