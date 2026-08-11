import { describe, expect, it, vi } from "vitest";

import { consumePairingToken } from "./pairing";

describe("pairing fragment", () => {
  it("returns the token and immediately removes it from visible history", () => {
    const replaceState = vi.fn();
    const token = consumePairingToken(
      {
        hash: "#pair=secret-token&view=drive",
        pathname: "/dashboard",
        search: "?theme=night",
      },
      { replaceState },
    );

    expect(token).toBe("secret-token");
    expect(replaceState).toHaveBeenCalledWith(
      null,
      "",
      "/dashboard?theme=night#view=drive",
    );
  });

  it("does not rewrite history when no token is present", () => {
    const replaceState = vi.fn();
    expect(
      consumePairingToken(
        { hash: "#view=drive", pathname: "/", search: "" },
        { replaceState },
      ),
    ).toBeUndefined();
    expect(replaceState).not.toHaveBeenCalled();
  });
});
