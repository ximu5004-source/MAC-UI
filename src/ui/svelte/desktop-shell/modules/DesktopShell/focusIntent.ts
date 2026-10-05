export interface DesktopFocusIntent {
  readonly operation: number;
  readonly blurEpoch: number;
}

/** Focus permission belongs to one interaction, never to an async file result. */
export class DesktopFocusGuard {
  private operation = 0;
  private blurEpoch = 0;
  private disposed = false;

  capture(): DesktopFocusIntent {
    return { operation: this.operation, blurEpoch: this.blurEpoch };
  }

  begin(): DesktopFocusIntent {
    this.invalidate();
    return this.capture();
  }

  blur(): void {
    this.blurEpoch++;
  }

  invalidate(): void {
    this.operation++;
  }

  dispose(): void {
    this.disposed = true;
    this.invalidate();
  }

  isCurrent(intent: DesktopFocusIntent): boolean {
    return !this.disposed && intent.operation === this.operation;
  }

  canFocus(intent: DesktopFocusIntent, documentFocused: boolean): boolean {
    return this.isCurrent(intent) && intent.blurEpoch === this.blurEpoch && documentFocused;
  }

  /** An owned menu intentionally blurred the desktop; only its live action may continue. */
  resumeMenu(intent: DesktopFocusIntent, sourceOperation: unknown): DesktopFocusIntent | null {
    return sourceOperation === intent.operation && this.isCurrent(intent) ? this.begin() : null;
  }
}

/** Track the owned menu chain, including submenus whose immediate owner is another menu. */
export function isDesktopInteractionWindow(
  focused: { hwnd: number; ownerHwnd: number },
  desktopHwnd: number,
  ownedMenus: Set<number>,
): boolean {
  if (focused.hwnd === desktopHwnd) return true;
  if (focused.hwnd !== 0 && (focused.ownerHwnd === desktopHwnd || ownedMenus.has(focused.ownerHwnd))) {
    ownedMenus.add(focused.hwnd);
    return true;
  }
  return ownedMenus.has(focused.hwnd);
}
