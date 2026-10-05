import { debounce } from "../../../utils/async.ts";
import { Widget_3 } from "./3_autosize.ts";
import { SerialQueue } from "../../../utils/serial_queue.ts";

/** Max time given to widgets to play their hide animation before the window is really hidden */
const HIDE_ANIMATION_MAX_MS = 200;

export class Widget_4 extends Widget_3 {
  protected _destroyOnHide = false;
  /**
   * Incremented on every show/hide call, so any in-flight call can detect it was superseded
   * by a newer one after each `await` and bail out.
   */
  private _visibilityToken = 0;
  private _nativeVisibility = new SerialQueue();
  private _debouncedClose = debounce((token: number) => {
    void this._nativeVisibility.run(async () => {
      if (token !== this._visibilityToken) return;
      const visible = await this.window.isVisible();
      if (!visible && token === this._visibilityToken) {
        await this.window.close();
      }
    }).catch(console.error);
  }, 30_000);

  public async show(): Promise<void> {
    const token = ++this._visibilityToken;
    this._debouncedClose.cancel();
    await this._nativeVisibility.run(async () => {
      if (token !== this._visibilityToken) return;
      // Native hide already in flight must finish before show. Token checks alone
      // cannot stop its late completion from hiding a newly opened panel.
      await this.window.show();
      if (token === this._visibilityToken) {
        delete globalThis.document.documentElement.dataset.widgetHidden;
      }
    });
  }

  public async hide(): Promise<void> {
    const token = ++this._visibilityToken;
    globalThis.document.documentElement.dataset.widgetHidden = "";

    await this.waitHideAnimations();
    if (token !== this._visibilityToken) {
      return;
    }
    await this._nativeVisibility.run(async () => {
      if (token !== this._visibilityToken) return;
      await this.window.hide();
      if (this._destroyOnHide && token === this._visibilityToken) {
        this._debouncedClose(token);
      }
    });
  }

  /**
   * Resolves once the finite animations/transitions triggered by the visibility change end.
   * Canceled animations (e.g. a transition reversed by a new show) also resolve it.
   */
  private async waitHideAnimations(): Promise<void> {
    // `getAnimations` forces a style recalc, so it already includes the ones just triggered
    const animations = globalThis.document
      .getAnimations()
      .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity);
    if (animations.length === 0) {
      return;
    }

    let timeout: ReturnType<typeof setTimeout>;
    await Promise.race([
      Promise.allSettled(animations.map((animation) => animation.finished)),
      new Promise((resolve) => (timeout = setTimeout(resolve, HIDE_ANIMATION_MAX_MS))),
    ]);
    clearTimeout(timeout!);
  }
}
