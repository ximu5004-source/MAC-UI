export function getWidgetConfig() { return {}; }
export function getWegConfig() { return { size: 40 }; }
export const patchWegConfig = () => { throw new Error("The isolated preview cannot write native settings"); };
export function patchWidgetConfig() { throw new Error("The isolated preview cannot write native settings"); }
