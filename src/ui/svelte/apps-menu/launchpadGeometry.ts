/** Windows monitor coordinates are physical pixels, including negative origins. */
interface MonitorGeometry {
  rect: { left: number; top: number; right: number; bottom: number };
  scaleFactor: number;
}

export function launchpadBounds(monitor: MonitorGeometry, fullscreen: boolean) {
  const { left, top, right, bottom } = monitor.rect;
  if (![left, top, right, bottom].every(Number.isFinite) || right <= left || bottom <= top) return null;
  // Fullscreen must use rcMonitor, not the work area or CSS pixels. Multiplying
  // these already-physical bounds by DPI would spill onto another monitor.
  if (fullscreen) return { left, top, right, bottom };
  const scale = Number.isFinite(monitor.scaleFactor) && monitor.scaleFactor > 0 ? monitor.scaleFactor : 1;
  const width = Math.round(Math.min((right - left) * .9, 1100 * scale));
  const height = Math.round(Math.min((bottom - top) * .88, 850 * scale));
  const x = Math.round(left + (right - left - width) / 2);
  const y = Math.round(top + (bottom - top - height) / 2);
  return { left: x, top: y, right: x + width, bottom: y + height };
}
