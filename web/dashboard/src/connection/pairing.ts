export interface FragmentLocation {
  readonly hash: string;
  readonly pathname: string;
  readonly search: string;
}

export interface HistoryWriter {
  replaceState(data: unknown, unused: string, url?: string | URL | null): void;
}

export function consumePairingToken(
  location: FragmentLocation,
  history: HistoryWriter,
): string | undefined {
  const fragment = location.hash.startsWith("#") ? location.hash.slice(1) : location.hash;
  const parameters = new URLSearchParams(fragment);
  const token = parameters.get("pair")?.trim();
  if (!token) {
    return undefined;
  }

  parameters.delete("pair");
  const remaining = parameters.toString();
  const nextFragment = remaining ? `#${remaining}` : "";
  history.replaceState(null, "", `${location.pathname}${location.search}${nextFragment}`);
  return token;
}
