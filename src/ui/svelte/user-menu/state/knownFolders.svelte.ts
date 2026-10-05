import { invoke, SeelenCommand, SeelenEvent, subscribe } from "@seelen-ui/lib";
import { FolderType } from "@seelen-ui/lib/types";
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

function pathAsItem(path: string) {
  return {
    path,
    displayName: path.split(/[\\/]/g).pop() || "",
  };
}

function predicate(path: string): boolean {
  let lowercased = path.toLowerCase();
  return !lowercased.endsWith(".ini") && !lowercased.endsWith(".tmp");
}

const _knownFolders: Record<FolderType, FolderData> = $derived.by(() => {
  return {
    [FolderType.Recent]: {
      icon: "MdOutlineHistory",
      content: [],
    },
    [FolderType.Desktop]: {
      icon: "HiOutlineDesktopComputer",
      content: desktop.value.filter(predicate).map(pathAsItem),
    },
    [FolderType.Downloads]: {
      icon: "PiDownloadSimpleBold",
      content: downloads.value.filter(predicate).map(pathAsItem),
    },
    [FolderType.Documents]: {
      icon: "IoDocumentsOutline",
      content: documents.value.filter(predicate).map(pathAsItem),
    },
    [FolderType.Music]: {
      icon: "BsFileEarmarkMusic",
      content: music.value.filter(predicate).map(pathAsItem),
    },
    [FolderType.Pictures]: {
      icon: "IoImageOutline",
      content: pictures.value.filter(predicate).map(pathAsItem),
    },
    [FolderType.Videos]: {
      icon: "PiVideo",
      content: videos.value.filter(predicate).map(pathAsItem),
    },
  };
});

export interface FolderData {
  icon: string;
  content: { path: string; displayName: string }[];
}

export const knownFolders = {
  get value() {
    return _knownFolders;
  },
};
