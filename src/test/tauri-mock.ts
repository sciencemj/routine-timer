import { vi } from 'vitest';

// vi.mock() is hoisted above imports; its factory may only reference variables whose
// name starts with `mock`. Hold shared mock state in a `mocks` object via vi.hoisted().
const mocks = vi.hoisted(() => ({
  handlers: new Map<string, (e: unknown) => void>(),
  invoke: vi.fn(() => Promise.resolve()),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (event: string, cb: (e: unknown) => void) => {
    mocks.handlers.set(event, cb);
    return () => mocks.handlers.delete(event);
  }),
}));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    label: 'main',
    theme: () => Promise.resolve('light'),
    onThemeChanged: () => Promise.resolve(() => {}),
    setTheme: () => Promise.resolve(),
  }),
}));

export const invokeMock = mocks.invoke;
export function emitTauri(event: string, payload: unknown) { mocks.handlers.get(event)?.({ event, id: 0, payload }); }
export function resetTauri() {
  mocks.handlers.clear();
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation(() => Promise.resolve());
}
