export interface HistoryState<Value> {
  readonly past: readonly Value[];
  readonly present: Value;
  readonly future: readonly Value[];
}

export function createHistory<Value>(initial: Value): HistoryState<Value> {
  return { past: [], present: initial, future: [] };
}

export function commitHistory<Value>(
  history: HistoryState<Value>,
  next: Value,
  limit = 50,
): HistoryState<Value> {
  if (Object.is(history.present, next)) {
    return history;
  }
  return {
    past: [...history.past, history.present].slice(-limit),
    present: next,
    future: [],
  };
}

export function undoHistory<Value>(history: HistoryState<Value>): HistoryState<Value> {
  const previous = history.past.at(-1);
  if (previous === undefined) {
    return history;
  }
  return {
    past: history.past.slice(0, -1),
    present: previous,
    future: [history.present, ...history.future],
  };
}

export function redoHistory<Value>(history: HistoryState<Value>): HistoryState<Value> {
  const [next, ...remaining] = history.future;
  if (next === undefined) {
    return history;
  }
  return {
    past: [...history.past, history.present],
    present: next,
    future: remaining,
  };
}
