export type ToolbarForeground = "black" | "white";

/** WCAG's black/white crossover with a small dead band to avoid flicker. */
export function toolbarForeground(luminance: number, previous?: ToolbarForeground): ToolbarForeground {
  if (!Number.isFinite(luminance)) return previous ?? "white";
  const value = Math.max(0, Math.min(1, luminance));
  if (previous === "white" && value < 0.20) return "white";
  if (previous === "black" && value > 0.16) return "black";
  return value > Math.sqrt(1.05 * 0.05) - 0.05 ? "black" : "white";
}
