import type { StartMenuItem } from "@macui/lib/types";
import { Document } from "flexsearch";
import { globalState } from "./mod.svelte";

let rawSearch = $state("");
const search = $derived(rawSearch.trim());

// Search exactly the same catalog as All Apps. Personal folders must never
// promote downloaded installers or documents into registered applications.
const items = $derived(globalState.allItems.toSorted((a, b) => a.display_name.localeCompare(b.display_name)));
const searchIndex = $derived.by(() => {
  const itemMap = new Map<string, StartMenuItem>();
  const index = new Document<{ id: string; name: string; filename: string; initials: string; folder: string }>({
    document: { id: "id", index: ["name", "filename", "initials", "folder"] },
    tokenize: "forward",
  });
  for (const item of items) {
    const id = getItemKey(item);
    itemMap.set(id, item);
    const path = item.target || item.path || "";
    index.add({
      id, name: item.display_name,
      filename: (path.split(/[\\/]/).pop() || "").replace(/\.[^.]+$/, ""),
      initials: item.display_name.split(/\s+/).map(word => word[0] || "").join(""),
      folder: item.start_menu_folder?.join(" ") || "",
    });
  }
  return { index, itemMap };
});

const searchedItems = $derived.by(() => {
  if (!search) return items;
  const { index, itemMap } = searchIndex;
  const seen = new Set<string>();
  const matches: StartMenuItem[] = [];
  for (const result of index.search(search, { limit: 1000, enrich: false })) {
    for (const id of result.result as string[]) {
      const item = itemMap.get(id);
      if (item && !seen.has(id)) { seen.add(id); matches.push(item); }
    }
  }
  return matches;
});

class SearchState {
  get searchQuery() { return rawSearch; }
  set searchQuery(value: string) { rawSearch = value; }
  get searchedItems() { return searchedItems; }
}
export const searchState = new SearchState();
export const getItemKey = (item: StartMenuItem) => globalState.getItemId(item);
