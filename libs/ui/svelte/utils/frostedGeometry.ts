export type FrostedRect = [number, number, number, number, number];
export interface FrostedSnapshot { viewport: [number, number]; rects: FrostedRect[] }

/** Keep the uncut geometry: clipping before rounding bends a sliding Dock's edge. */
export function frostedRect(
  bounds: { left: number; top: number; right: number; bottom: number },
  radius: string,
  viewport: [number, number],
): FrostedRect | null {
  const { left, top, right, bottom } = bounds;
  const width = right - left, height = bottom - top;
  if (![left, top, right, bottom, ...viewport].every(Number.isFinite) || width <= 0 || height <= 0
    || viewport[0] <= 0 || viewport[1] <= 0 || right <= 0 || bottom <= 0
    || left >= viewport[0] || top >= viewport[1]) return null;
  const parsed = Number.parseFloat(radius);
  const pixels = radius.includes("%") ? parsed * Math.min(width, height) / 100 : parsed;
  return [left, top, right, bottom, Math.max(0, Math.min(Number.isFinite(pixels) ? pixels : 0, width / 2, height / 2))];
}

/** Latest-state queue: an old show must never win over a newer hide. */
export class FrostedWriter {
  private pending: FrostedSnapshot | null = null;
  private writing: Promise<void> | null = null;
  private lastKey = "";
  private generation = 0;

  constructor(private send: (snapshot: FrostedSnapshot) => Promise<void>) {}

  invalidate() { this.lastKey = ""; this.generation++; }

  write(snapshot: FrostedSnapshot): Promise<void> {
    this.pending = snapshot;
    if (!this.writing) {
      this.writing = Promise.resolve().then(() => this.drain());
    }
    return this.writing;
  }

  private async drain() {
    let failure: unknown;
    while (this.pending) {
      const next = this.pending;
      this.pending = null;
      const key = JSON.stringify(next);
      if (key === this.lastKey) continue;
      const generation = this.generation;
      try {
        await this.send(next);
        this.lastKey = generation === this.generation ? key : "";
        failure = undefined;
      } catch (error) {
        this.lastKey = ""; // failed geometry is never considered applied
        failure = error;
      }
    }
    // Clear synchronously when the queue becomes empty. A .finally() outside
    // drain leaves a microtask gap where a deduplicated show can strand a hide.
    this.writing = null;
    if (failure !== undefined) throw failure;
  }
}
