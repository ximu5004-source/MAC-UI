/** Native display names are labels, not necessarily legal Windows filenames. */
export function newFileBaseName(label: string, fallback: string): string {
  const safe = label.replace(/[<>:"/\\|?*\u0000-\u001f]/g, " ").replace(/\s+/g, " ").trim().replace(/[. ]+$/g, "");
  return Array.from(safe || fallback).slice(0, 80).join("");
}

/** Select the stem only, preserving the file extension when typing a new name. */
export function renameSelectionEnd(name: string, isDirectory: boolean): number {
  const dot = name.lastIndexOf(".");
  return !isDirectory && dot > 0 ? dot : name.length;
}
