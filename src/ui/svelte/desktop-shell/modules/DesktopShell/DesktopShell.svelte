<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke, SeelenCommand, SeelenEvent, subscribe, Widget } from "@seelen-ui/lib";
  import type { ContextMenu, ContextMenuCallbackPayload, DesktopEntry, DesktopNewFileType } from "@seelen-ui/lib/types";
  import { FolderType } from "@seelen-ui/lib/types";
  import { FileIcon, Icon } from "libs/ui/svelte/components/Icon";
  import { t } from "../../i18n";
  import { desktopState } from "../../state.svelte";
  import DesktopStack from "./DesktopStack.svelte";
  import { alignmentGridVisible, defaultStackPosition, isStackPoint, isStackSize, STACK_GRID, type StackPoint, type StackSize } from "./stackLayout";
  import { newFileBaseName, renameSelectionEnd } from "./newFiles";
  import { ICON_SIZES, validIconSize } from "./iconSize";
  import { DesktopFocusGuard, isDesktopInteractionWindow, type DesktopFocusIntent } from "./focusIntent";
  import { trackDesktopRegions } from "./interactiveRegions";

  type ArrangeMode = "name" | "category";
  type Category = "folders" | "images" | "documents" | "videos" | "audio" | "archives" | "apps" | "other";

  interface DesktopPreferences {
    arrangeMode: ArrangeMode;
    positions?: Partial<Record<Category, StackPoint>>;
    sizes?: Partial<Record<Category, StackSize>>;
    snapToGrid?: boolean;
    showGrid?: boolean;
    collapsed?: Category[];
    iconSize?: number;
  }

  const PREFERENCES_FILE = "desktop-shell.json";
  const ITEM_MENU_EVENT = `desktop-shell::item-menu-${crypto.randomUUID()}`;
  const DESKTOP_MENU_EVENT = `desktop-shell::desktop-menu-${crypto.randomUUID()}`;
  const ITEM_MENU_ID = crypto.randomUUID();
  const DESKTOP_MENU_ID = crypto.randomUUID();
  const ARRANGE_SUBMENU_ID = crypto.randomUUID();
  const NEW_FILE_SUBMENU_ID = crypto.randomUUID();
  const ICON_SIZE_SUBMENU_ID = crypto.randomUUID();

  const categoryOrder: Category[] = [
    "folders",
    "images",
    "documents",
    "videos",
    "audio",
    "archives",
    "apps",
    "other",
  ];

  const imageExtensions = new Set(["avif", "bmp", "gif", "heic", "jpeg", "jpg", "png", "svg", "webp"]);
  const documentExtensions = new Set(["csv", "doc", "docx", "md", "pdf", "ppt", "pptx", "rtf", "txt", "xls", "xlsx"]);
  const videoExtensions = new Set(["avi", "m4v", "mkv", "mov", "mp4", "webm", "wmv"]);
  const audioExtensions = new Set(["aac", "flac", "m4a", "mp3", "ogg", "wav", "wma"]);
  const archiveExtensions = new Set(["7z", "gz", "rar", "tar", "zip"]);
  const appExtensions = new Set(["appref-ms", "bat", "cmd", "exe", "lnk", "msi"]);

  let items = $state<DesktopEntry[]>([]);
  let selected = $state<string | null>(null);
  let ready = $state(false);
  let arrangeMode = $state<ArrangeMode>("name");
  let collapsedCategories = $state(new Set<Category>());
  let stackPositions = $state<Partial<Record<Category, StackPoint>>>({});
  let stackSizes = $state<Partial<Record<Category, StackSize>>>({});
  let snapToGrid = $state(true);
  let showGrid = $state(false);
  let iconSize = $state(64);
  let interacting = $state(false);
  let activeCategory = $state<Category | null>(null);
  let board = $state<HTMLDivElement>();
  let boardSize = $state({ width: 0, height: 0 });
  let preferenceWrite = Promise.resolve();
  let renaming = $state<string | null>(null);
  let renameDraft = $state("");
  let renameInput = $state<HTMLInputElement | null>(null);
  let errorMessage = $state("");
  let busy = $state(false);
  let disposed = false;
  const focusGuard = new DesktopFocusGuard();
  const ownedMenus = new Set<number>();
  let menuFocusIntent: DesktopFocusIntent | null = null;
  let pendingRenameFocus: { path: string; intent: DesktopFocusIntent } | null = null;
  let renameRevision = 0;
  let renameCommit: { path: string; revision: number } | null = null;
  let nativeFileTypes = $state<DesktopNewFileType[]>([]);
  const newFileTypes = $derived([
    { extension: ".txt", label: $t("desktop_shell.file_types.text"), baseName: $t("desktop_shell.new_text_name") },
    { extension: ".md", label: $t("desktop_shell.file_types.markdown"), baseName: $t("desktop_shell.new_markdown_name") },
    ...nativeFileTypes.map((type) => ({
      ...type,
      baseName: newFileBaseName($t("desktop_shell.new_file_name", { type: type.label }), $t("desktop_shell.new_file_fallback")),
    })),
  ]);
  const monitor = $derived(desktopState.relativeMonitors.find((m) => m.isPrimary) ?? desktopState.relativeMonitors[0]);
  const monitorStyle = $derived(monitor
    ? `left:${monitor.rect.left}px;top:${monitor.rect.top}px;width:${monitor.rect.right - monitor.rect.left}px;height:${monitor.rect.bottom - monitor.rect.top}px`
    : "inset:0");

  function reportError(error: unknown) {
    if (disposed) return;
    errorMessage = String(error);
    console.error(error);
  }

  function displayName(entry: DesktopEntry) {
    return entry.extension === "lnk" ? entry.name.replace(/\.lnk$/i, "") : entry.name;
  }

  const sortedItems = $derived.by(() => {
    return [...items].sort((a, b) => a.name.localeCompare(b.name, undefined, {
      numeric: true,
      sensitivity: "base",
    }));
  });

  const groups = $derived.by(() => {
    return categoryOrder
      .map((category) => ({
        category,
        items: sortedItems.filter((entry) => categoryFor(entry) === category),
      }))
      .filter((group) => group.items.length > 0);
  });

  $effect(() => {
    if (!board) return;
    const observer = new ResizeObserver(() => {
      if (board) boardSize = { width: board.clientWidth, height: board.clientHeight };
    });
    observer.observe(board);
    return () => observer.disconnect();
  });

  function savePreferences() {
    const content = JSON.stringify({ arrangeMode, positions: stackPositions, sizes: stackSizes, snapToGrid, showGrid, iconSize, collapsed: [...collapsedCategories] } satisfies DesktopPreferences);
    preferenceWrite = preferenceWrite.then(() => invoke(SeelenCommand.WriteFile, {
      filename: PREFERENCES_FILE, content,
    })).catch(reportError);
  }

  function moveStack(category: Category, point: StackPoint, commit: boolean) {
    stackPositions = { ...stackPositions, [category]: point };
    if (commit) savePreferences();
  }

  function resetLayout() {
    stackPositions = {};
    stackSizes = {};
    collapsedCategories = new Set();
    errorMessage = "";
    savePreferences();
  }

  function resizeStack(category: Category, point: StackPoint, size: StackSize | undefined, commit: boolean) {
    stackPositions = { ...stackPositions, [category]: point };
    stackSizes = { ...stackSizes, [category]: size };
    if (commit) savePreferences();
  }

  function toggleSnap() {
    snapToGrid = !snapToGrid;
    savePreferences();
  }

  function toggleGrid() {
    showGrid = !showGrid;
    savePreferences();
  }

  function setIconSize(size: unknown) {
    iconSize = validIconSize(size);
    savePreferences();
  }

  function categoryFor(entry: DesktopEntry): Category {
    if (entry.isDirectory) return "folders";
    const extension = entry.extension?.toLowerCase() || "";
    if (imageExtensions.has(extension)) return "images";
    if (documentExtensions.has(extension)) return "documents";
    if (videoExtensions.has(extension)) return "videos";
    if (audioExtensions.has(extension)) return "audio";
    if (archiveExtensions.has(extension)) return "archives";
    if (appExtensions.has(extension)) return "apps";
    return "other";
  }

  async function refresh() {
    const entries = await invoke(SeelenCommand.GetDesktopEntries);
    if (disposed) return;
    items = entries;
    if (selected && !items.some((entry) => entry.path === selected)) selected = null;
  }

  function open(path: string) {
    errorMessage = "";
    void invoke(SeelenCommand.OpenFile, { path }).catch(reportError);
  }

  function reveal(path: string) {
    void invoke(SeelenCommand.SelectFileOnExplorer, { path }).catch(reportError);
  }

  async function createFolder(intent = focusGuard.begin()) {
    if (busy) return;
    busy = true;
    errorMessage = "";
    try {
      const path = await invoke(SeelenCommand.CreateDesktopFolder, {
        baseName: $t("desktop_shell.new_folder_name"),
      });
      await refresh();
      await startRename(path, intent);
    } catch (error) { reportError(error); }
    finally { busy = false; }
  }

  async function createFile(extension: string, intent: DesktopFocusIntent) {
    const type = newFileTypes.find((type) => type.extension === extension);
    if (busy || !type) return;
    busy = true;
    errorMessage = "";
    try {
      const path = await invoke(SeelenCommand.CreateDesktopFile, { extension, baseName: type.baseName });
      await refresh();
      await startRename(path, intent);
    } catch (error) { reportError(error); }
    finally { busy = false; }
  }

  function cancelPendingFocus() {
    focusGuard.invalidate();
    pendingRenameFocus = null;
    menuFocusIntent = null;
  }

  function focusRenameInput() {
    const pending = pendingRenameFocus;
    if (!pending) return;
    if (!focusGuard.canFocus(pending.intent, true) || renaming !== pending.path) {
      pendingRenameFocus = null;
      return;
    }
    // A menu can still be closing. Wait for natural focus return, never activate
    // this fullscreen native window after file I/O or a delayed menu callback.
    if (!document.hasFocus() || renameInput?.dataset.renamePath !== pending.path) return;
    const entry = items.find((item) => item.path === pending.path);
    if (!entry) return;
    pendingRenameFocus = null;
    renameInput.focus();
    renameInput.setSelectionRange(0, renameSelectionEnd(entry.name, entry.isDirectory));
    renameInput.scrollIntoView({ block: "nearest", inline: "nearest" });
  }

  async function startRename(path: string, intent = focusGuard.begin()) {
    if (!focusGuard.isCurrent(intent)) return;
    const entry = items.find((item) => item.path === path);
    if (!entry) return;
    renameRevision++;
    selected = path;
    // A newly-created file must be reachable even inside a collapsed/scrolling stack.
    const expanded = new Set(collapsedCategories);
    expanded.delete(categoryFor(entry));
    collapsedCategories = expanded;
    renaming = path;
    renameDraft = entry.name;
    const pending = { path, intent };
    pendingRenameFocus = pending;
    await tick();
    if (pendingRenameFocus === pending) focusRenameInput();
  }

  async function commitRename(restoreFocusOnError = true) {
    if (!renaming) return;
    const oldPath = renaming;
    const revision = renameRevision;
    if (renameCommit?.revision === revision) return;
    const intent = focusGuard.capture();
    const entry = items.find((item) => item.path === oldPath);
    if (!entry || renameDraft.trim() === entry.name) {
      renaming = null;
      pendingRenameFocus = null;
      renameRevision++;
      return;
    }

    const commit = { path: oldPath, revision };
    renameCommit = commit;
    try {
      const newPath = await invoke(SeelenCommand.RenameDesktopEntry, {
        path: oldPath,
        newName: renameDraft.trim(),
      });
      if (!disposed && renameRevision === revision && renaming === oldPath) {
        selected = newPath;
        renaming = null;
        pendingRenameFocus = null;
        renameRevision++;
      }
      await refresh();
    } catch (error) {
      reportError(error);
      if (restoreFocusOnError && renameRevision === revision && renaming === oldPath &&
        renameInput?.dataset.renamePath === oldPath &&
        focusGuard.canFocus(intent, document.hasFocus())) {
        renameInput?.focus();
        renameInput?.select();
      }
    } finally {
      if (renameCommit === commit) renameCommit = null;
    }
  }

  function cancelRename() {
    cancelPendingFocus();
    renameRevision++;
    renaming = null;
    renameDraft = "";
  }

  function handleKeyDown(event: KeyboardEvent, entry: DesktopEntry) {
    if (renaming === entry.path) return;
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      open(entry.path);
    } else if (event.key === "F2") {
      event.preventDefault();
      void startRename(entry.path);
    }
  }

  function handleRenameKeyDown(event: KeyboardEvent) {
    event.stopPropagation();
    if (event.key === "Enter") {
      event.preventDefault();
      void commitRename();
    } else if (event.key === "Escape") {
      event.preventDefault();
      cancelRename();
    }
  }

  function itemMenu(entry: DesktopEntry, intent: DesktopFocusIntent): ContextMenu {
    return {
      identifier: ITEM_MENU_ID,
      meta: { path: entry.path, focusOperation: intent.operation },
      items: [
        {
          type: "Item",
          key: "open",
          label: $t(entry.isDirectory ? "desktop_shell.menu.open_folder" : "desktop_shell.menu.open"),
          icon: entry.isDirectory ? "TbFolderOpen" : "TbExternalLink",
          callbackEvent: ITEM_MENU_EVENT,
        },
        {
          type: "Item",
          key: "reveal",
          label: $t("desktop_shell.menu.reveal"),
          icon: "MdOutlineMyLocation",
          callbackEvent: ITEM_MENU_EVENT,
        },
        { type: "Separator" },
        {
          type: "Item",
          key: "rename",
          label: $t("desktop_shell.menu.rename"),
          icon: "RiEditLine",
          callbackEvent: ITEM_MENU_EVENT,
        },
      ],
    };
  }

  function desktopMenu(intent: DesktopFocusIntent): ContextMenu {
    return {
      identifier: DESKTOP_MENU_ID,
      meta: { focusOperation: intent.operation },
      items: [
        {
          type: "Item",
          key: "new_folder",
          label: $t("desktop_shell.menu.new_folder"),
          icon: "TbFolderPlus",
          disabled: busy,
          callbackEvent: DESKTOP_MENU_EVENT,
        },
        {
          type: "Submenu",
          identifier: NEW_FILE_SUBMENU_ID,
          label: $t("desktop_shell.menu.new_file"),
          icon: "TbFilePlus",
          items: newFileTypes.map((type) => ({
            type: "Item" as const,
            key: `new_file:${type.extension}`,
            label: `${type.label} (${type.extension})`,
            icon: "TbFilePlus",
            disabled: busy,
            callbackEvent: DESKTOP_MENU_EVENT,
          })),
        },
        {
          type: "Submenu",
          identifier: ARRANGE_SUBMENU_ID,
          label: $t("desktop_shell.menu.arrange"),
          icon: "TbLayoutGrid",
          items: [
            {
              type: "Item",
              key: "arrange_name",
              label: $t("desktop_shell.arrange.name"),
              icon: "TbSortAscendingLetters",
              callbackEvent: DESKTOP_MENU_EVENT,
              checked: arrangeMode === "name",
            },
            {
              type: "Item",
              key: "arrange_category",
              label: $t("desktop_shell.arrange.category"),
              icon: "TbCategory2",
              callbackEvent: DESKTOP_MENU_EVENT,
              checked: arrangeMode === "category",
            },
          ],
        },
        {
          type: "Item",
          key: "refresh",
          label: $t("desktop_shell.menu.refresh"),
          icon: "TbRefresh",
          callbackEvent: DESKTOP_MENU_EVENT,
        },
        {
          type: "Item", key: "snap_grid", label: $t("desktop_shell.snap_grid"),
          icon: "TbGridDots", checked: snapToGrid, callbackEvent: DESKTOP_MENU_EVENT,
        },
        {
          type: "Item", key: "show_grid", label: $t("desktop_shell.show_grid"),
          icon: "TbGridPattern", checked: showGrid, disabled: arrangeMode !== "category", callbackEvent: DESKTOP_MENU_EVENT,
        },
        {
          type: "Submenu", identifier: ICON_SIZE_SUBMENU_ID, label: $t("desktop_shell.icon_size"),
          items: ICON_SIZES.map((size) => ({ type: "Item" as const, key: `icon_size:${size}`,
            label: $t(`desktop_shell.icon_sizes.${size}`), checked: iconSize === size, callbackEvent: DESKTOP_MENU_EVENT })),
        },
        { type: "Separator" },
        {
          type: "Item",
          key: "open_desktop",
          label: $t("desktop_shell.menu.open_desktop"),
          icon: "TbFolderOpen",
          callbackEvent: DESKTOP_MENU_EVENT,
        },
      ],
    };
  }

  function showItemMenu(event: MouseEvent, entry: DesktopEntry) {
    event.preventDefault();
    event.stopPropagation();
    pendingRenameFocus = null;
    ownedMenus.clear();
    menuFocusIntent = focusGuard.begin();
    selected = entry.path;
    void invoke(SeelenCommand.TriggerContextMenu, { menu: itemMenu(entry, menuFocusIntent), forwardTo: null });
  }

  function showDesktopMenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    pendingRenameFocus = null;
    ownedMenus.clear();
    menuFocusIntent = focusGuard.begin();
    selected = null;
    void invoke(SeelenCommand.TriggerContextMenu, { menu: desktopMenu(menuFocusIntent), forwardTo: null });
  }

  function handleDesktopKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      selected = null;
      cancelRename();
    }
  }

  function setArrangeMode(mode: ArrangeMode) {
    arrangeMode = mode;
    errorMessage = "";
    savePreferences();
  }

  function toggleCategory(category: Category) {
    const next = new Set(collapsedCategories);
    if (next.has(category)) next.delete(category);
    else next.add(category);
    collapsedCategories = next;
    savePreferences();
  }

  function itemMenuAction(payload: ContextMenuCallbackPayload) {
    const path = (payload.meta as { path?: string } | null)?.path;
    if (!path) return;
    if (payload.key === "open") open(path);
    else if (payload.key === "reveal") reveal(path);
    else if (payload.key === "rename") {
      const intent = takeMenuFocusIntent(payload);
      if (intent) void startRename(path, intent);
    }
  }

  function takeMenuFocusIntent(payload: ContextMenuCallbackPayload): DesktopFocusIntent | null {
    const intent = menuFocusIntent;
    const sourceOperation = (payload.meta as { focusOperation?: unknown } | null)?.focusOperation;
    const resumed = intent ? focusGuard.resumeMenu(intent, sourceOperation) : null;
    if (resumed) menuFocusIntent = null;
    return resumed;
  }

  function desktopMenuAction(payload: ContextMenuCallbackPayload) {
    if (payload.key === "new_folder" || payload.key.startsWith("new_file:")) {
      const intent = takeMenuFocusIntent(payload);
      if (!intent) return;
      if (payload.key === "new_folder") void createFolder(intent);
      else void createFile(payload.key.slice("new_file:".length), intent);
    }
    else if (payload.key === "arrange_name") setArrangeMode("name");
    else if (payload.key === "arrange_category") setArrangeMode("category");
    else if (payload.key === "refresh") void refresh();
    else if (payload.key === "snap_grid") toggleSnap();
    else if (payload.key === "show_grid") toggleGrid();
    else if (payload.key.startsWith("icon_size:")) setIconSize(Number(payload.key.slice(10)));
    else if (payload.key === "open_desktop") open("shell:Desktop");
  }

  onMount(() => {
    const regions = trackDesktopRegions(
      (snapshot) => invoke(SeelenCommand.SetDesktopRegions, snapshot),
      (error) => {
        reportError(error);
        // A broken renderer must not take the native desktop away.
        void invoke(SeelenCommand.SetNativeDesktopIconsHidden, { hidden: false }).catch(console.error);
      },
    );
    let unsubscribeFolder: (() => void) | undefined;
    let unsubscribeItemMenu: (() => void) | undefined;
    let unsubscribeDesktopMenu: (() => void) | undefined;
    let unsubscribeFocus: (() => void) | undefined;
    const onBlur = () => { focusGuard.blur(); };
    globalThis.addEventListener("blur", onBlur);
    globalThis.addEventListener("focus", focusRenameInput);

    // Optional templates must never delay desktop readiness or basic text-file creation.
    void invoke(SeelenCommand.GetDesktopNewFileTypes).then((types) => {
      if (!disposed) nativeFileTypes = types;
    }).catch(console.error);

    void (async () => {
      unsubscribeFocus = await subscribe(SeelenEvent.GlobalFocusChanged, ({ payload }) => {
        if (disposed) return;
        if (!isDesktopInteractionWindow(payload, Widget.self.windowId, ownedMenus)) {
          cancelPendingFocus();
          ownedMenus.clear();
        }
      });
      if (disposed) {
        unsubscribeFocus();
        return;
      }
      try {
        const saved = JSON.parse(await invoke(SeelenCommand.ReadFile, {
          filename: PREFERENCES_FILE,
        })) as Partial<DesktopPreferences>;
        if (saved.arrangeMode === "name" || saved.arrangeMode === "category") {
          arrangeMode = saved.arrangeMode;
        }
        stackPositions = Object.fromEntries(categoryOrder.flatMap((category) => {
          const point = saved.positions?.[category];
          return isStackPoint(point) ? [[category, point]] : [];
        }));
        stackSizes = Object.fromEntries(categoryOrder.flatMap((category) => {
          const size = saved.sizes?.[category];
          return isStackSize(size) ? [[category, size]] : [];
        }));
        snapToGrid = saved.snapToGrid !== false;
        showGrid = saved.showGrid === true;
        iconSize = validIconSize(saved.iconSize);
        collapsedCategories = new Set(categoryOrder.filter((category) => Array.isArray(saved.collapsed) && saved.collapsed.includes(category)));
      } catch {
        // First launch has no preferences file.
      }

      await refresh();
      if (disposed) return;

      unsubscribeFolder = await subscribe(
        SeelenEvent.UserFolderChanged,
        ({ payload: { ofFolder } }) => {
          if (ofFolder === FolderType.Desktop) void refresh();
        },
      );
      unsubscribeItemMenu = await Widget.self.webview.listen<ContextMenuCallbackPayload>(
        ITEM_MENU_EVENT,
        ({ payload }) => itemMenuAction(payload),
      );
      unsubscribeDesktopMenu = await Widget.self.webview.listen<ContextMenuCallbackPayload>(
        DESKTOP_MENU_EVENT,
        ({ payload }) => desktopMenuAction(payload),
      );

      if (disposed) {
        unsubscribeFolder?.();
        unsubscribeItemMenu?.();
        unsubscribeDesktopMenu?.();
        unsubscribeFocus?.();
        return;
      }

      ready = true;
      await tick();
      if (disposed) return;
      await regions.flush();
      if (disposed) return;
      await Widget.self.ready();
      if (disposed) return;
      await invoke(SeelenCommand.SetNativeDesktopIconsHidden, { hidden: true });
    })().catch((error) => {
      if (disposed) return;
      reportError(error);
      // Keep Explorer available if loading the replacement desktop fails.
      ready = true;
      void Widget.self.ready().catch(console.error);
    });

    return () => {
      disposed = true;
      regions.dispose();
      focusGuard.dispose();
      pendingRenameFocus = null;
      menuFocusIntent = null;
      globalThis.removeEventListener("blur", onBlur);
      globalThis.removeEventListener("focus", focusRenameInput);
      unsubscribeFolder?.();
      unsubscribeItemMenu?.();
      unsubscribeDesktopMenu?.();
      unsubscribeFocus?.();
      void invoke(SeelenCommand.SetNativeDesktopIconsHidden, { hidden: false }).catch(console.error);
    };
  });
</script>

{#snippet desktopItem(entry: DesktopEntry)}
  {#if renaming === entry.path}
    <div class="desktop-item is-selected" title={entry.path}>
      <FileIcon path={entry.path} class="desktop-item-icon" appearance={entry.isApplication ? "app-glass" : "liquid-glass"} lazy />
      <input
        bind:this={renameInput}
        bind:value={renameDraft}
        data-rename-path={entry.path}
        class="desktop-rename-input"
        aria-label={$t("desktop_shell.rename_label")}
        onclick={(event) => event.stopPropagation()}
        ondblclick={(event) => event.stopPropagation()}
        onkeydown={handleRenameKeyDown}
        onblur={() => void commitRename(false)}
      />
    </div>
  {:else}
    <button
      class="desktop-item"
      class:is-selected={selected === entry.path}
      aria-label={entry.name}
      aria-pressed={selected === entry.path}
      title={entry.path}
      onclick={(event) => {
        event.stopPropagation();
        selected = entry.path;
      }}
      ondblclick={(event) => {
        event.stopPropagation();
        open(entry.path);
      }}
      oncontextmenu={(event) => showItemMenu(event, entry)}
      onkeydown={(event) => handleKeyDown(event, entry)}
    >
      <FileIcon path={entry.path} class="desktop-item-icon" appearance={entry.isApplication ? "app-glass" : "liquid-glass"} lazy />
      <span class="desktop-item-name">{displayName(entry)}</span>
    </button>
  {/if}
{/snippet}

{#if ready}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions (desktop replacement surface intentionally handles background input) -->
  <section
    class="desktop-shell"
    style={`${monitorStyle};--desktop-icon-size:${iconSize}px;--desktop-cell-width:${iconSize + 48}px;--desktop-cell-height:${iconSize + 56}px`}
    role="application"
    tabindex="-1"
    aria-label={$t("desktop_shell.label")}
    onpointerdowncapture={cancelPendingFocus}
    onclick={() => (selected = null)}
    oncontextmenu={showDesktopMenu}
    onkeydown={handleDesktopKeyDown}
  >
    {#if busy}
      <div class="desktop-status" role="status">{$t("desktop_shell.creating")}</div>
    {/if}
    <div class="desktop-controls mac-frosted-surface" role="toolbar" aria-label={$t("desktop_shell.controls_label")}>
      <button
        class:active={arrangeMode === "name"}
        aria-pressed={arrangeMode === "name"}
        title={$t("desktop_shell.arrange.name")}
        onclick={(event) => {
          event.stopPropagation();
          setArrangeMode("name");
        }}
      >
        <Icon name="TbSortAscendingLetters" aria-hidden="true" />
        <span>{$t("desktop_shell.arrange.name_short")}</span>
      </button>
      <button
        class:active={arrangeMode === "category"}
        aria-pressed={arrangeMode === "category"}
        title={$t("desktop_shell.arrange.category")}
        onclick={(event) => {
          event.stopPropagation();
          setArrangeMode("category");
        }}
      >
        <Icon name="TbCategory2" aria-hidden="true" />
        <span>{$t("desktop_shell.arrange.category_short")}</span>
      </button>
      <span class="control-separator"></span>
      <label class="icon-size-control">
        <span>{$t("desktop_shell.icon_size")}</span>
        <select aria-label={$t("desktop_shell.icon_size")} value={iconSize} onchange={(event) => setIconSize(Number(event.currentTarget.value))}>
          {#each ICON_SIZES as size}<option value={size}>{$t(`desktop_shell.icon_sizes.${size}`)}</option>{/each}
        </select>
      </label>
      {#if arrangeMode === "category"}
        <button class:active={showGrid} aria-pressed={showGrid} aria-label={$t("desktop_shell.show_grid")} title={$t("desktop_shell.show_grid_hint")} onclick={(event) => { event.stopPropagation(); toggleGrid(); }}>
          <Icon name="TbGridPattern" aria-hidden="true" />
          <span>{$t("desktop_shell.show_grid")}</span>
        </button>
        <button class:active={snapToGrid} aria-pressed={snapToGrid} aria-label={$t("desktop_shell.snap_grid")} title={$t("desktop_shell.snap_hint")} onclick={(event) => { event.stopPropagation(); toggleSnap(); }}>
          <Icon name="TbGridDots" aria-hidden="true" />
          <span>{$t("desktop_shell.snap_grid")}</span>
        </button>
        <button aria-label={$t("desktop_shell.reset_layout")} title={$t("desktop_shell.reset_layout")} onclick={(event) => { event.stopPropagation(); resetLayout(); }}>
          <Icon name="TbLayoutGrid" aria-hidden="true" />
        </button>
      {/if}
      <button
        disabled={busy}
        aria-label={$t("desktop_shell.menu.new_folder")}
        title={$t("desktop_shell.menu.new_folder")}
        onclick={(event) => {
          event.stopPropagation();
          void createFolder();
        }}
      >
        <Icon name="TbFolderPlus" aria-hidden="true" />
      </button>
      <button
        aria-label={$t("desktop_shell.menu.refresh")}
        title={$t("desktop_shell.menu.refresh")}
        onclick={(event) => {
          event.stopPropagation();
          void refresh();
        }}
      >
        <Icon name="TbRefresh" aria-hidden="true" />
      </button>
      <button aria-label={$t("desktop_shell.controls_label")} title={$t("desktop_shell.controls_label")} onclick={showDesktopMenu}>
        <Icon name="TbDots" aria-hidden="true" />
      </button>
    </div>

    {#if errorMessage}
      <div class="desktop-error" role="alert">{$t("desktop_shell.operation_failed")}: {errorMessage}</div>
    {/if}

    {#if arrangeMode === "category"}
      <div class="category-board" class:show-grid={alignmentGridVisible(showGrid, interacting, snapToGrid)} style:--grid-size={`${STACK_GRID}px`} bind:this={board}>
        {#if boardSize.width > 0 && boardSize.height > 0}
          {#each groups as group, index (group.category)}
            <DesktopStack
              title={$t(`desktop_shell.categories.${group.category}`)}
              count={group.items.length}
              collapsed={collapsedCategories.has(group.category)}
              position={stackPositions[group.category] ?? defaultStackPosition(index, boardSize)}
              dimensions={stackSizes[group.category]}
              snap={snapToGrid}
              bounds={boardSize}
              active={activeCategory === group.category}
              onactivate={() => (activeCategory = group.category)}
              onmove={(point, commit) => moveStack(group.category, point, commit)}
              onresize={(point, size, commit) => resizeStack(group.category, point, size, commit)}
              oninteraction={(value) => (interacting = value)}
              oncollapse={() => toggleCategory(group.category)}
            >
              {#each group.items as entry (entry.path)}
                {@render desktopItem(entry)}
              {/each}
            </DesktopStack>
          {/each}
        {/if}
      </div>
    {:else}
      <div class="desktop-grid">
        {#each sortedItems as entry (entry.path)}
          {@render desktopItem(entry)}
        {/each}
      </div>
    {/if}
  </section>
{/if}

<style>
  .desktop-shell {
    position: fixed;
    z-index: 100;
    box-sizing: border-box;
    padding: 112px 24px 116px;
    overflow: hidden;
    user-select: none;
  }

  .desktop-status {
    position: absolute;
    top: 100px;
    right: 24px;
    padding: 8px 12px;
    border-radius: 8px;
    color: var(--color-persist-white);
    background: rgb(20 40 60 / 0.85);
    z-index: 4;
  }

  .desktop-controls {
    position: absolute;
    z-index: 3;
    top: 46px;
    right: 22px;
    display: flex;
    max-width: calc(100% - 44px);
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    min-height: 46px;
    padding: 4px;
    border: 0;
    border-radius: 17px;
    background: var(--mac-glass-surface, rgb(255 255 255 / 0.1));
    box-shadow: none;
    backdrop-filter: var(--mac-glass-blur, blur(18px));
  }

  .desktop-controls button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-width: 44px;
    height: 38px;
    padding: 0 11px;
    border: 1px solid transparent;
    border-radius: 12px;
    color: white;
    text-shadow: 0 1px 3px rgb(0 0 0 / 0.7);
    background: transparent;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .desktop-controls button:hover,
  .desktop-controls button:focus-visible,
  .desktop-controls button.active {
    border-color: rgb(255 255 255 / 0.3);
    background: rgb(255 255 255 / 0.1);
    box-shadow: none;
  }

  .desktop-controls button:active {
    transform: scale(0.96);
  }

  .desktop-controls :global(.slu-icon) {
    font-size: 18px;
  }

  .control-separator {
    width: 1px;
    height: 24px;
    margin: 0 2px;
    background: rgb(255 255 255 / 0.58);
    box-shadow: none;
  }

  .icon-size-control { display: flex; align-items: center; gap: 6px; padding: 0 6px; font-size: 12px; color: white; text-shadow: 0 1px 3px #0009; }
  .icon-size-control select { font: inherit; min-height: 32px; padding: 0 5px; border: 1px solid rgb(255 255 255 / 0.1); border-radius: 8px; color: white; background: rgb(255 255 255 / 0.1); }
  .icon-size-control option { color: #111; background: #fff; }
  .icon-size-control select:focus-visible { outline: 2px solid #096adc; outline-offset: 2px; }

  .desktop-grid {
    display: grid;
    direction: rtl;
    grid-auto-flow: column;
    grid-template-rows: repeat(auto-fill, var(--desktop-cell-height));
    grid-auto-columns: var(--desktop-cell-width);
    justify-content: start;
    align-content: start;
    width: 100%;
    height: 100%;
    gap: 12px 10px;
    overflow: auto;
    padding: 4px;
    box-sizing: border-box;
  }

  .category-board {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .category-board.show-grid::before {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    background-image: linear-gradient(to right, rgb(255 255 255 / 0.22) 1px, transparent 1px), linear-gradient(to bottom, rgb(255 255 255 / 0.22) 1px, transparent 1px);
    background-size: var(--grid-size) var(--grid-size);
    outline: 1px solid rgb(255 255 255 / 0.3);
  }

  .desktop-item {
    direction: ltr;
    display: grid;
    position: relative;
    box-sizing: border-box;
    grid-template-rows: var(--desktop-icon-size) 36px;
    justify-items: center;
    align-content: start;
    gap: 5px;
    width: min(100%, var(--desktop-cell-width));
    min-width: 0;
    height: var(--desktop-cell-height);
    padding: 6px 6px 8px;
    border: 1px solid transparent;
    border-radius: 16px;
    color: white;
    background: transparent;
    outline: none;
    user-select: none;
    cursor: pointer;
  }

  .desktop-item:hover,
  .desktop-item:focus-visible,
  .desktop-item.is-selected {
    border-color: rgb(255 255 255 / 0.46);
    background: rgb(255 255 255 / 0.1);
    box-shadow: none;
    backdrop-filter: blur(6px);
  }

  .desktop-item:active :global(.desktop-item-icon) {
    transform: scale(0.96);
  }

  :global(.desktop-item-icon) {
    width: var(--desktop-icon-size);
    height: var(--desktop-icon-size);
  }

  .desktop-item-name {
    display: -webkit-box;
    overflow: hidden;
    width: 100%;
    max-width: calc(var(--desktop-cell-width) - 12px);
    min-height: 36px;
    color: white;
    font-size: 12px;
    font-weight: 500;
    line-height: 18px;
    text-align: center;
    text-wrap: balance;
    overflow-wrap: anywhere;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    text-shadow: 0 1px 1px rgb(0 0 0 / 0.9);
  }

  .desktop-item.is-selected .desktop-item-name,
  .desktop-item:focus-visible .desktop-item-name {
    display: block;
    overflow: visible;
    position: relative;
    z-index: 5;
    background: transparent;
    -webkit-line-clamp: unset;
    line-clamp: unset;
  }

  .desktop-item.is-selected, .desktop-item:focus-visible { z-index: 4; }
  .desktop-error {
    position: absolute;
    z-index: 10;
    left: 24px;
    top: 48px;
    max-width: min(520px, 50%);
    padding: 12px 16px;
    border-radius: 12px;
    color: white;
    background: rgb(125 32 32 / 0.95);
    overflow-wrap: anywhere;
  }

  .desktop-rename-input {
    box-sizing: border-box;
    width: 90px;
    min-height: 26px;
    padding: 3px 5px;
    border: 1px solid rgb(65 151 255 / 0.88);
    border-radius: 7px;
    color: rgb(15 35 62);
    background: rgb(255 255 255 / 0.94);
    box-shadow: 0 0 0 2px rgb(255 255 255 / 0.32), 0 5px 14px rgb(12 52 101 / 0.2);
    font: inherit;
    font-size: 12px;
    text-align: center;
    user-select: text;
  }

  @media (prefers-color-scheme: dark) {
    .desktop-controls button {
      color: rgb(240 248 255 / 0.96);
      text-shadow: 0 1px 4px rgb(0 0 0 / 0.5);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .desktop-item,
    .desktop-controls button,
    :global(.desktop-item-icon) {
      transition: none;
    }
  }
</style>
