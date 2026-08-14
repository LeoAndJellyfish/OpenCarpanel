export type OutputElements = ReadonlyMap<string, HTMLElement>;

export function outputElements(root: HTMLElement): OutputElements {
  return new Map(
    Array.from(root.querySelectorAll<HTMLElement>("[data-value]")).flatMap((element) => {
      const key = element.dataset.value;
      return key ? [[key, element] as const] : [];
    }),
  );
}

export function setOutput(outputs: OutputElements, key: string, value: string): void {
  const element = outputs.get(key);
  if (element && element.textContent !== value) {
    element.textContent = value;
  }
}

export function setActive(outputs: OutputElements, key: string, active: boolean): void {
  const element = outputs.get(key);
  if (element) {
    element.dataset.active = active ? "true" : "false";
  }
}
