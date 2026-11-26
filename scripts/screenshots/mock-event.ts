/** Stub for `@tauri-apps/api/event` used by the screenshot harness only. */
import fixture from "./data.json";

type Handler = (event: { payload: unknown }) => void;

export function listen<T>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<() => void> {
  if (event === "search:hit") {
    // Fills the hit list so the screenshot shows a finished search.
    for (const hit of fixture.hits) {
      queueMicrotask(() => handler({ payload: hit as T }));
    }
  }
  return Promise.resolve(() => undefined);
}
