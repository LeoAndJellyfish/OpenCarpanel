import {
  DEFAULT_LAYOUT,
  GAME_DEFAULT_LAYOUTS,
  type BuiltinGameId,
  type LayoutDocument,
} from "@opencarpanel/widget-sdk";

export type GameFamily = "formula" | "truck" | "neutral";
export type StatusMode = "drs" | "scs" | "generic";

export interface GamePresentation {
  readonly id: BuiltinGameId | "unknown";
  readonly label: string;
  readonly detail: string;
  readonly family: GameFamily;
  readonly statusMode: StatusMode;
  readonly layoutId: string;
  readonly defaultLayout: LayoutDocument;
}

export interface BuiltinGamePresentation extends GamePresentation {
  readonly id: BuiltinGameId;
}

export const SUPPORTED_GAME_PRESENTATIONS: readonly BuiltinGamePresentation[] = [
  {
    id: "f1-24",
    label: "F1 24",
    detail: "FORMULA / UDP 2024",
    family: "formula",
    statusMode: "drs",
    layoutId: "game-f1-24",
    defaultLayout: GAME_DEFAULT_LAYOUTS["f1-24"],
  },
  {
    id: "f1-25",
    label: "F1 25",
    detail: "FORMULA / UDP 2025 + 2026",
    family: "formula",
    statusMode: "drs",
    layoutId: "game-f1-25",
    defaultLayout: GAME_DEFAULT_LAYOUTS["f1-25"],
  },
  {
    id: "ets2",
    label: "EURO TRUCK SIMULATOR 2",
    detail: "LONG HAUL / SCS SDK",
    family: "truck",
    statusMode: "scs",
    layoutId: "game-ets2",
    defaultLayout: GAME_DEFAULT_LAYOUTS.ets2,
  },
  {
    id: "ats",
    label: "AMERICAN TRUCK SIMULATOR",
    detail: "INTERSTATE / SCS SDK",
    family: "truck",
    statusMode: "scs",
    layoutId: "game-ats",
    defaultLayout: GAME_DEFAULT_LAYOUTS.ats,
  },
];

const UNKNOWN_GAME_PRESENTATION: GamePresentation = {
  id: "unknown",
  label: "OPEN CARPANEL",
  detail: "WAITING FOR GAME TELEMETRY",
  family: "neutral",
  statusMode: "generic",
  layoutId: "default",
  defaultLayout: DEFAULT_LAYOUT,
};

export function gamePresentation(gameId: string | null | undefined): GamePresentation {
  return (
    SUPPORTED_GAME_PRESENTATIONS.find((presentation) => presentation.id === gameId) ??
    UNKNOWN_GAME_PRESENTATION
  );
}

export function isBuiltinGameId(value: string | null | undefined): value is BuiltinGameId {
  return SUPPORTED_GAME_PRESENTATIONS.some((presentation) => presentation.id === value);
}
