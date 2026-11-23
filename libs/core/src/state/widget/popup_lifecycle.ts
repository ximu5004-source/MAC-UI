/** Per-webview intent: a late preparation must not reopen a dismissed popup. */
export class PopupIntent {
  private revision = 0;
  private open = false;
  private source: string | null = null;

  begin(toggleSource: string | null): { token: number; close: boolean } {
    const close = this.open && toggleSource !== null && toggleSource === this.source;
    this.revision += 1;
    this.open = !close;
    this.source = close ? null : toggleSource;
    return { token: this.revision, close };
  }

  dismiss(): void {
    this.revision += 1;
    this.open = false;
    this.source = null;
  }

  isCurrent(token: number): boolean {
    return token === this.revision && this.open;
  }
  get isOpen(): boolean {
    return this.open;
  }
  get generation(): number {
    return this.revision;
  }
}

export function popupOwnsFocus(windowId: number, focused: { hwnd: number; ownerHwnd?: number | null }): boolean {
  return windowId !== 0 && (focused.hwnd === windowId || focused.ownerHwnd === windowId);
}

/** Hidden preparation is canceled only by a real foreground transition. */
export function shouldDismissPopupAfterFocusLoss(
  visible: boolean,
  openingHwnd: number | undefined,
  focusedHwnd: number,
): boolean {
  return visible || (openingHwnd !== undefined && openingHwnd !== focusedHwnd);
}

/** Only explicit toolbar click sources toggle. Hover-driven previews keep showing. */
export function popupToggleSource(args: Record<string, unknown> | null | undefined): string | null {
  const source = args?.__macuiPopupToggleSource;
  return typeof source === "string" && source.length > 0 && source.length <= 256 ? source : null;
}
