export const DESKTOP_CONTROLS_REQUEST = "macui-desktop-controls-request";
export const DESKTOP_CONTROLS_RESPONSE = "macui-desktop-controls-response";
export const DESKTOP_CONTROLS_CHANGED = "macui-desktop-controls-changed";

export interface DesktopControls {
  arrangeMode: "name" | "category";
  iconSize: number;
  snapToGrid: boolean;
  showGrid: boolean;
  layoutLocked: boolean;
  glassTransparency: number;
  glassBlur: number;
  snapBoxes: boolean;
  horizontalSpacing: number;
  verticalSpacing: number;
}
export interface DesktopControlsRequest {
  id: string;
  patch?: Partial<DesktopControls>;
}
export interface DesktopControlsResponse {
  id: string;
  controls?: DesktopControls;
  error?: string;
}
export function desktopControls(source: DesktopControls): DesktopControls {
  const {
    arrangeMode,
    iconSize,
    snapToGrid,
    showGrid,
    layoutLocked,
    glassTransparency,
    glassBlur,
    snapBoxes,
    horizontalSpacing,
    verticalSpacing,
  } = source;
  return {
    arrangeMode,
    iconSize,
    snapToGrid,
    showGrid,
    layoutLocked,
    glassTransparency,
    glassBlur,
    snapBoxes,
    horizontalSpacing,
    verticalSpacing,
  };
}
export function validateDesktopControlsPatch(patch: Partial<DesktopControls>): void {
  for (const [key, value] of Object.entries(patch)) {
    if (key === "arrangeMode" && (value === "name" || value === "category")) continue;
    if (key === "iconSize" && typeof value === "number" && Number.isInteger(value) && value >= 16 && value <= 256) {
      continue;
    }
    if (["snapToGrid", "showGrid", "layoutLocked", "snapBoxes"].includes(key) && typeof value === "boolean") continue;
    if (
      ["horizontalSpacing", "verticalSpacing"].includes(key) && typeof value === "number" && Number.isInteger(value) &&
      value >= 0 && value <= 64
    ) continue;
    if (
      ["glassTransparency", "glassBlur"].includes(key) && typeof value === "number" && Number.isInteger(value) &&
      value >= 0 && value <= 100
    ) continue;
    throw new Error(`Invalid desktop setting: ${key}`);
  }
}
