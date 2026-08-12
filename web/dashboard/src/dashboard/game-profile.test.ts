import { GAME_DEFAULT_LAYOUTS } from "@opencarpanel/widget-sdk";
import { describe, expect, it } from "vitest";

import { gamePresentation, SUPPORTED_GAME_PRESENTATIONS } from "./game-profile";

describe("game presentation profiles", () => {
  it("maps every built-in game to an independent persisted layout", () => {
    expect(SUPPORTED_GAME_PRESENTATIONS.map((profile) => profile.id)).toEqual([
      "f1-24",
      "f1-25",
      "ets2",
      "ats",
    ]);
    expect(new Set(SUPPORTED_GAME_PRESENTATIONS.map((profile) => profile.layoutId)).size).toBe(4);
    for (const profile of SUPPORTED_GAME_PRESENTATIONS) {
      expect(profile.defaultLayout).toBe(GAME_DEFAULT_LAYOUTS[profile.id]);
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
});
