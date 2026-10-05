export const ICON_SIZES = [48, 64, 80, 96] as const;
export function validIconSize(value: unknown): number {
  return ICON_SIZES.includes(value as typeof ICON_SIZES[number]) ? Number(value) : 64;
}
