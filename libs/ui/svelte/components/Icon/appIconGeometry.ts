/** Normalize transparent padding while preserving the artwork's aspect ratio. */
export function appIconGeometry(rgba: ArrayLike<number>, width: number, height: number, edgeToEdge = false) {
  let left = width, top = height, right = -1, bottom = -1;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if ((rgba[(y * width + x) * 4 + 3] ?? 0) < 16) continue;
      left = Math.min(left, x);
      right = Math.max(right, x);
      top = Math.min(top, y);
      bottom = Math.max(bottom, y);
    }
  }
  if (right < left || bottom < top) return { scale: 1, x: 0, y: 0 };
  // Tiles fill the common rounded mask; freeform artwork keeps antialiasing room.
  const margin = edgeToEdge ? 0 : 2;
  const scale = Math.min(2, width / (right - left + 1 + margin), height / (bottom - top + 1 + margin));
  return {
    scale,
    x: (0.5 - (left + right + 1) / (2 * width)) * scale * 100,
    y: (0.5 - (top + bottom + 1) / (2 * height)) * scale * 100,
  };
}

/** Distinguish existing square/rounded tiles from circles and freestanding marks.
 * Ignore faint drop shadows so they don't turn a circle into a square. */
export function appIconIsTile(rgba: ArrayLike<number>, width: number, height: number): boolean {
  let left = width, top = height, right = -1, bottom = -1, opaque = 0;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if ((rgba[(y * width + x) * 4 + 3] ?? 0) < 128) continue;
      opaque++;
      left = Math.min(left, x);
      right = Math.max(right, x);
      top = Math.min(top, y);
      bottom = Math.max(bottom, y);
    }
  }
  if (!opaque) return false;
  const w = right - left + 1, h = bottom - top + 1;
  return Math.min(w, h) / Math.max(w, h) >= 0.9 && opaque / (w * h) >= 0.88;
}
