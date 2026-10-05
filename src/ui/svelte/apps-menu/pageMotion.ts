export interface PageLocation { page: number; resetKey: string }

/** Search, geometry and reopen changes replace content; only page navigation slides. */
export function pageDirection(previous: PageLocation | undefined, next: PageLocation): -1 | 0 | 1 {
  if (!previous || previous.resetKey !== next.resetKey) return 0;
  return Math.sign(next.page - previous.page) as -1 | 0 | 1;
}

export interface WheelSample {
  deltaX: number;
  deltaY: number;
  deltaMode: number;
  ctrlKey: boolean;
}

/** One page per wheel/trackpad gesture, including its momentum tail. */
export class WheelPager {
  private lastAt = -Infinity;
  private total = 0;
  private consumed = false;

  reset() {
    this.lastAt = -Infinity;
    this.total = 0;
    this.consumed = false;
  }

  consume(sample: WheelSample, now: number, pageExtent: number): -1 | 0 | 1 {
    if (sample.ctrlKey) { this.reset(); return 0; }
    const raw = Math.abs(sample.deltaX) > Math.abs(sample.deltaY) ? sample.deltaX : sample.deltaY;
    if (!Number.isFinite(raw) || raw === 0) return 0;
    const multiplier = sample.deltaMode === 1 ? 16 : sample.deltaMode === 2 ? Math.max(1, pageExtent) : 1;
    const delta = raw * multiplier;
    // Momentum usually arrives every frame. An idle gap starts a deliberate new gesture.
    if (now - this.lastAt > 180 || Math.sign(delta) !== Math.sign(this.total)) {
      this.total = 0;
      this.consumed = false;
    }
    this.lastAt = now;
    this.total += delta;
    if (this.consumed || Math.abs(this.total) < 40) return 0;
    this.consumed = true;
    return Math.sign(this.total) as -1 | 1;
  }
}
