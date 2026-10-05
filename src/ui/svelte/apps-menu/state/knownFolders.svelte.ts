import { invoke, SeelenCommand, SeelenEvent, subscribe } from "@seelen-ui/lib";
import { FolderType, type StartMenuItem } from "@seelen-ui/lib/types";
import { lazyRune } from "libs/ui/svelte/utils";

const desktop = lazyRune(() => invoke(SeelenCommand.GetUserFolderContent, { folderType: FolderType.Desktop }));
const downloads = lazyRune(() => invoke(SeelenCommand.GetUserFolderContent, { folderType: FolderType.Downloads }));
const documents = lazyRune(() => invoke(SeelenCommand.GetUserFolderContent, { folderType: FolderType.Documents }));
const music = lazyRune(() => invoke(SeelenCommand.GetUserFolderContent, { folderType: FolderType.Music }));
const pictures = lazyRune(() => invoke(SeelenCommand.GetUserFolderContent, { folderType: FolderType.Pictures }));
const videos = lazyRune(() => invoke(SeelenCommand.GetUserFolderContent, { folderType: FolderType.Videos }));

subscribe(SeelenEvent.UserFolderChanged, ({ payload: { ofFolder, content } }) => {
  switch (ofFolder) {
    case FolderType.Desktop:
      desktop.value = content;
      break;
    case FolderType.Downloads:
      downloads.value = content;
      break;
    case FolderType.Documents:
      documents.value = content;
      break;
    case FolderType.Music:
      music.value = content;
      break;
    case FolderType.Pictures:
      pictures.value = content;
      break;
    case FolderType.Videos:
      videos.value = content;
      break;
  }
});

await Promise.all([
  desktop.init(),
  downloads.init(),
  documents.init(),
  music.init(),
  pictures.init(),
  videos.init(),
]);

function pathAsItem(path: string): StartMenuItem {
  return {
    path,
    umid: null,
    display_name: path.split(/[\\/]/g).pop() || "",
    target: null,
    toast_activator: null,
    start_menu_folder: [],
  };
}

const _foldersAsStartMenuItems = $derived.by(() => {
  return [
    desktop.value.map(pathAsItem),
    downloads.value.map(pathAsItem),
    documents.value.map(pathAsItem),
    music.value.map(pathAsItem),
    pictures.value.map(pathAsItem),
    videos.value.map(pathAsItem),
  ].flat();
});

export const foldersAsStartMenuItems = {
  get value() {
    return _foldersAsStartMenuItems;
  },
};

// =======================================================
// ======================For Debug========================
// =======================================================

const extensionCounts: Record<string, number> = {};
let index = 0;
function process(deadline: IdleDeadline) {
  while (index < _foldersAsStartMenuItems.length && deadline.timeRemaining() > 0) {
    const item = _foldersAsStartMenuItems[index++];
    const extension = item?.path.split(".").pop();
    if (extension) {
      extensionCounts[extension] ??= 0;
      extensionCounts[extension]++;
    }
  }

  if (index < _foldersAsStartMenuItems.length) {
    requestIdleCallback(process);
    return;
  }

  invoke(SeelenCommand.WriteFile, {
    filename: "index.log",
    content: JSON.stringify({
      totalFiles: _foldersAsStartMenuItems.length,
      extensionCounts,
    }),
  });
}

requestIdleCallback(process);
