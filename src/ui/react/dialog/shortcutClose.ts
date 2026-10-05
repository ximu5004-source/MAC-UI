import type { Dialog } from "@seelen-ui/lib/types";

/** Only the backend shortcut dialog opts into close-as-cancel. */
export function shortcutCancelEvent(dialog: Dialog | null): string | null {
  if (!dialog) return null;
  const event = `shortcut_register_cancelled:${dialog.identifier}`;
  return dialog.footer.some((entry) => entry.type === "button" && entry.onClick === event)
    ? event
    : null;
}
