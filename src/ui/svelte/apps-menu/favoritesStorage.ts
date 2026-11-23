import { canonicalizeFavorites, migrateFavorites, normalizeLayout } from "./launchpad";
import type { FavPinnedItem } from "./launchpad";
import type { StartMenuItem } from "@macui/lib/types";

export const FAVORITES_FILE = "launchpad-favorites-v2.json";

async function readOptional(filename: string, read: (filename: string) => Promise<string>) {
  try { return await read(filename); }
  catch (error) {
    if (error !== null && typeof error === "object" && "code" in error && error.code === "not_found") return null;
    // The restored 3.0.1 backend serializes std::io::Error as an AppError
    // string. Match only its structured Windows NotFound code/kind prefix;
    // permission, sharing, localized text and ambiguous errors must not reset
    // user favorites. This keeps the existing shared ReadFile IPC unchanged.
    if (typeof error === "string" && /^Io\(Os \{\s*code: (?:2|3),\s*kind: NotFound,/.test(error)) return null;
    throw error;
  }
}

/** Loading never writes. Any ambiguous I/O failure leaves all existing files intact. */
export async function loadFavoritesStorage(read: (filename: string) => Promise<string>, catalog: StartMenuItem[]) {
  const current = await readOptional(FAVORITES_FILE, read);
  if (current !== null) {
    const parsed = JSON.parse(current);
    if (parsed?.version !== 2 || !Array.isArray(parsed.items)) throw new Error("Invalid favorites v2 layout");
    return { items: canonicalizeFavorites(normalizeLayout(parsed.items), catalog), needsSave: false, migrated: false };
  }
  const previous = await readOptional("launchpad-v1.json", read);
  let items: FavPinnedItem[];
  if (previous !== null) {
    const parsed = JSON.parse(previous);
    if (parsed?.version !== 1 || !Array.isArray(parsed.items)) throw new Error("Invalid legacy Launchpad layout");
    items = migrateFavorites(parsed).items;
  } else {
    const original = await readOptional("favorites.json", read);
    if (original === null) items = [];
    else {
      const parsed = JSON.parse(original);
      if (!Array.isArray(parsed)) throw new Error("Invalid original favorites layout");
      items = migrateFavorites(null, parsed).items;
    }
  }
  return { items: canonicalizeFavorites(items, catalog), needsSave: true, migrated: items.length > 0 };
}
