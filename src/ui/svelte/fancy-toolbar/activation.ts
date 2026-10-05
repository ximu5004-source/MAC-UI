/** Ignore non-primary and drag-consumed clicks; context menus have a separate event. */
export function isPrimaryToolbarClick(
  event: Pick<MouseEvent, "button" | "defaultPrevented">,
  isDragging = false,
): boolean {
  return event.button === 0 && !event.defaultPrevented && !isDragging;
}

export function isToolbarActivationKey(
  event: Pick<KeyboardEvent, "key" | "repeat" | "altKey" | "ctrlKey" | "metaKey">,
): boolean {
  return (event.key === "Enter" || event.key === " ") && !event.repeat &&
    !event.altKey && !event.ctrlKey && !event.metaKey;
}
