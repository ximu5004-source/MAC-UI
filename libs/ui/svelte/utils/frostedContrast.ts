export type FrostedInk = "black" | "white";

/** Wallpaper-average heuristic, not a claim about arbitrary windows behind glass.
 * Estimate the 10% neutral white coat in sRGB, then compare black/white contrast. */
export function frostedForeground(luminance: number, previous?: FrostedInk): FrostedInk {
  if (!Number.isFinite(luminance)) return previous ?? "white";
  const linear = Math.max(0, Math.min(1, luminance));
  const srgb = linear <= .0031308 ? linear * 12.92 : 1.055 * linear ** (1 / 2.4) - .055;
  const coated = srgb * .9 + .1;
  const effective = coated <= .04045 ? coated / 12.92 : ((coated + .055) / 1.055) ** 2.4;
  // The narrow stability band keeps both pure text colors above 4.5:1 on a
  // uniform sample, without flickering when a wallpaper slideshow changes.
  if (previous && effective >= .176 && effective <= .183) return previous;
  return effective >= Math.sqrt(.0525) - .05 ? "black" : "white";
}
