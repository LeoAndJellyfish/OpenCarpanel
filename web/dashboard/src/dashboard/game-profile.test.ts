import {
  BUILTIN_GAME_PLUGINS,
  GAME_DEFAULT_LAYOUTS,
  type GamePluginMetadata,
} from "@opencarpanel/widget-sdk";
import { describe, expect, it } from "vitest";

import { gamePresentation, SUPPORTED_GAME_PRESENTATIONS } from "./game-profile";

describe("game presentation profiles", () => {
  it("maps every built-in game to an independent persisted layout", () => {
    expect(SUPPORTED_GAME_PRESENTATIONS.map((profile) => profile.id)).toEqual(
      BUILTIN_GAME_PLUGINS.map((plugin) => plugin.id),
    );
    expect(new Set(SUPPORTED_GAME_PRESENTATIONS.map((profile) => profile.layoutId)).size).toBe(4);
    for (const profile of SUPPORTED_GAME_PRESENTATIONS) {
      expect(profile.defaultLayout).toEqual(GAME_DEFAULT_LAYOUTS[profile.id as keyof typeof GAME_DEFAULT_LAYOUTS]);
      expect(profile.defaultLayout.id).toBe(profile.layoutId);
    }
  });

  it("uses racing semantics for F1 and truck semantics for SCS games", () => {
    expect(gamePresentation("f1-24").family).toBe("formula");
    expect(gamePresentation("f1-25").statusMode).toBe("drs");
    expect(gamePresentation("ets2").family).toBe("truck");
    expect(gamePresentation("ats").statusMode).toBe("scs");
  });

  it("falls back safely for absent and unknown game ids", () => {
    expect(gamePresentation(undefined).id).toBe("unknown");
    expect(gamePresentation("future-game").layoutId).toBe("default");
  });

  it("creates an independent trusted layout for an external plugin id", () => {
    const plugin: GamePluginMetadata = structuredClone(BUILTIN_GAME_PLUGINS[0]);
    plugin.id = "future-game";
    plugin.name = "Future Game";
    plugin.source = "installed";
    plugin.presentation.family = "generic";
    plugin.presentation.layoutPreset = "generic";
    plugin.presentation.widgets = ["core.speed"];
    plugin.presentation.theme.accent = "#123456";

    const presentation = gamePresentation(plugin.id, [plugin]);
    expect(presentation.id).toBe("future-game");
    expect(presentation.layoutId).toBe("game-future-game");
    expect(presentation.defaultLayout.theme.accent).toBe("#123456");
    expect(presentation.defaultLayout.widgets.map((widget) => widget.componentType)).toEqual([
      "core.speed",
    ]);
  });
});
