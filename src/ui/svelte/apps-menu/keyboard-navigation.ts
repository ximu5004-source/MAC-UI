export type NavigationDirection = "up" | "down" | "left" | "right";

type PageShortcutEvent = Pick<KeyboardEvent, "key" | "ctrlKey" | "repeat" | "isComposing" | "defaultPrevented">;

/** null is not a page shortcut; 0 consumes a held page key without paging again. */
export function pageShortcutStep(event: PageShortcutEvent): -1 | 0 | 1 | null {
  if (event.isComposing || event.defaultPrevented) return null;
  const next = event.key === "PageDown" || (event.ctrlKey && event.key === "ArrowRight");
  const previous = event.key === "PageUp" || (event.ctrlKey && event.key === "ArrowLeft");
  if (!next && !previous) return null;
  // Keep ordinary arrow-key repeat for navigating icons; only page shortcuts are single-shot.
  return event.repeat ? 0 : next ? 1 : -1;
}

export interface SelectionScope {
  container: () => ParentNode;
  getSelected: () => string | null;
  setSelected: (id: string | null) => void;
}

/** Keep a selection valid after paging, resizing or removing a visible item. */
export function reconcilePageSelection(selected: string | null, visibleIds: readonly string[]): string | null {
  // A null selection intentionally leaves the first option tabbable without
  // preselecting or activating it while the user is editing the search input.
  if (selected === null || visibleIds.includes(selected)) return selected;
  return visibleIds[0] ?? null;
}

export function navigateInDirection(
  direction: NavigationDirection,
  scope: SelectionScope,
): void {
  const allItems = Array.from(scope.container().querySelectorAll(".app, .folder")) as HTMLElement[];
  if (allItems.length === 0) return;

  function selectItem(item: HTMLElement) {
    const activeElement = item.ownerDocument.activeElement;
    const gridHasFocus = allItems.some(candidate => candidate === activeElement || candidate.contains(activeElement));
    scope.setSelected(item.dataset.itemId || null);
    // A focused option handles Enter itself. Keep its DOM focus in sync with
    // the roving selection so it cannot activate the previous option. While
    // typing into search, arrows only preselect; do not steal input focus.
    if (gridHasFocus) item.focus({ preventScroll: true });
  }

  const selected = scope.getSelected();

  if (!selected) {
    selectItem(allItems[0]!);
    return;
  }

  const currentElement = allItems.find((item) => item.dataset.itemId === selected) || null;
  if (!currentElement) {
    selectItem(allItems[0]!);
    return;
  }

  const currentRect = currentElement.getBoundingClientRect();
  const candidates = allItems
    // filter items that are not in the same row/column
    .filter((item) => {
      if (item === currentElement) return false;
      const rect = item.getBoundingClientRect();
      switch (direction) {
        case "right":
          return rect.top === currentRect.top && rect.left > currentRect.left;
        case "left":
          return rect.top === currentRect.top && rect.left < currentRect.left;
        case "down":
          return rect.left === currentRect.left && rect.top > currentRect.top;
        case "up":
          return rect.left === currentRect.left && rect.top < currentRect.top;
      }
    });

  const idxToTake = ["right", "down"].includes(direction) ? 0 : -1;
  const toTake = candidates.at(idxToTake);
  if (toTake) {
    selectItem(toTake);
  }
}

// Resolves the element that Enter should activate: the selected item, or the first one.
export function selectPreselectedOrFirst(scope: SelectionScope): HTMLElement | null {
  const selected = scope.getSelected();
  const root = scope.container();

  if (selected) {
    const element = Array.from(root.querySelectorAll<HTMLElement>("[data-item-id]")).find(el => el.dataset.itemId === selected);
    if (element) return element;
  }

  // return root.querySelector<HTMLElement>(".app, .folder");
  return null;
}

export interface InputKeyDownOptions {
  // return true to fully handle Enter here and skip the default "click
  // preselected or first item" behavior of whichever view is listening
  onEnter?: (event: KeyboardEvent, input: HTMLInputElement) => boolean | void;
}

// Keydown handler for the search input. It only deals with input-specific
// concerns; ArrowUp/ArrowDown/ArrowLeft and non-web Enter are left to bubble
// up to whichever view's own window-level listener is currently mounted, so
// this input never needs to know which view (or its selection) is active.
export function createInputKeyDownHandler(options: InputKeyDownOptions = {}) {
  return function handleInputKeyDown(event: KeyboardEvent) {
    // Enter first confirms an IME candidate; it must not launch a web search.
    if (event.isComposing || event.defaultPrevented) return;
    const input = event.currentTarget as HTMLInputElement;

    switch (event.key) {
      case "Enter": {
        if (options.onEnter?.(event, input)) {
          event.preventDefault();
          event.stopPropagation();
        }
        break;
      }
      case "ArrowLeft":
      case "ArrowRight":
        // Preserve normal text editing; Ctrl+arrows are Launchpad page shortcuts.
        if (!event.ctrlKey) {
          event.stopPropagation();
        }
        break;
    }
  };
}
