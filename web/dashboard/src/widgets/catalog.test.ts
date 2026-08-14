import { describe, expect, it } from "vitest";

import {
  GAME_DEFAULT_LAYOUTS,
  type BuiltinGameId,
  type WidgetType,
} from "@opencarpanel/widget-sdk";

import { addWidget, removeWidgetsByType } from "../editor/document";
import { builtinWidgetManifestsForGame } from "./catalog";

const FORMULA_GAMES: readonly BuiltinGameId[] = ["f1-24", "f1-25"];
const TRUCK_GAMES: readonly BuiltinGameId[] = ["ets2", "ats"];

function componentTypes(gameId: BuiltinGameId): WidgetType[] {
  return builtinWidgetManifestsForGame(gameId).map((manifest) => manifest.type);
}

describe("built-in widget availability", () => {
  it.each(FORMULA_GAMES)("shows only formula widgets for %s", (gameId) => {
    expect(componentTypes(gameId)).toEqual([
      "core.gear",
      "core.race",
      "core.speed",
      "core.tachometer",
      "core.tyres",
      "core.status",
    ]);
  });

  it.each(TRUCK_GAMES)("shows only truck widgets for %s", (gameId) => {
    expect(componentTypes(gameId)).toEqual([
      "core.gear",
      "core.route",
      "core.speed",
      "core.tachometer",
      "core.status",
    ]);
  });

  it.each([...FORMULA_GAMES, ...TRUCK_GAMES])(
    "can re-enable every supported component across all breakpoints for %s",
    (gameId) => {
      for (const manifest of builtinWidgetManifestsForGame(gameId)) {
        const disabled = removeWidgetsByType(GAME_DEFAULT_LAYOUTS[gameId], manifest.type);
        const enabled = addWidget(disabled, manifest);

        expect(enabled, manifest.type).toBeDefined();
        expect(enabled!.widgets.at(-1)?.componentType).toBe(manifest.type);
        expect(Object.keys(enabled!.widgets.at(-1)!.placements)).toHaveLength(4);
      }
    },
  );
});
