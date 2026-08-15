export const DEVICE_SESSION_KEY = "opensimdash.device-session.v1";

export function readDeviceSession(storage: Storage): string | undefined {
  const value = storage.getItem(DEVICE_SESSION_KEY)?.trim();
  return value || undefined;
}

export function writeDeviceSession(storage: Storage, session: string): void {
  storage.setItem(DEVICE_SESSION_KEY, session);
}
