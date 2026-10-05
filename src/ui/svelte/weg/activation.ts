const pending = new Map<string, Promise<void>>();

export function isDockPrimaryClick(event: Pick<MouseEvent, "button" | "defaultPrevented">): boolean {
  return event.button === 0 && !event.defaultPrevented;
}

/** Restore if supported; otherwise use the native app entry point. Never launch on restore errors. */
export function restoreOrLaunch(
  key: string,
  restore: () => Promise<boolean>,
  launch: () => Promise<unknown>,
): Promise<void> {
  const existing = pending.get(key);
  if (existing) return existing;
  const action = (async () => {
    if (!await restore()) await launch();
  })();
  const result = action.finally(() => {
    pending.delete(key);
  });
  pending.set(key, result);
  return result;
}
