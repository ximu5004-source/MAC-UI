using Microsoft.Win32;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace MacUI.Desktop.Native
{
    internal sealed class DesktopEntry
    {
        internal string Path, Name, Extension;
        internal bool IsDirectory, IsDesktop, IsNamespace;
        internal long Size, Modified;
        internal string Key { get { return Json.Key(Path); } }
        internal static DesktopEntry File(string path, bool desktop)
        {
            var info = new FileInfo(path);
            var attributes = info.Attributes;
            bool folder = (attributes & FileAttributes.Directory) != 0;
            bool shortcut = info.Extension.Equals(".lnk", StringComparison.OrdinalIgnoreCase) || info.Extension.Equals(".url", StringComparison.OrdinalIgnoreCase);
            return new DesktopEntry { Path = info.FullName, Name = shortcut ? System.IO.Path.GetFileNameWithoutExtension(info.Name) : info.Name, Extension = folder ? "" : info.Extension.TrimStart('.').ToLowerInvariant(),
                IsDirectory = folder, IsDesktop = desktop, Size = folder ? 0 : info.Length, Modified = (long)(info.LastWriteTimeUtc - new DateTime(1970, 1, 1)).TotalMilliseconds };
        }
    }

    internal sealed class CatalogSnapshot
    {
        internal readonly List<DesktopEntry> Desktop = new List<DesktopEntry>();
        internal readonly Dictionary<string, List<DesktopEntry>> Folders = new Dictionary<string, List<DesktopEntry>>(StringComparer.OrdinalIgnoreCase);
        internal readonly Dictionary<string, string> Errors = new Dictionary<string, string>();
    }

    internal sealed class ShellCatalog : IDisposable
    {
        private readonly StaWorker worker = new StaWorker("macui.desktop.catalog");
        private readonly StaWorker icons = new StaWorker("macui.desktop.icons");
        private readonly Dictionary<string, ImageSource> cache = new Dictionary<string, ImageSource>();
        internal readonly string DesktopPath, PublicPath;
        internal ShellCatalog(string testRoot)
        {
            DesktopPath = testRoot ?? Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
            PublicPath = testRoot == null ? Environment.GetFolderPath(Environment.SpecialFolder.CommonDesktopDirectory) : null;
        }
        internal Task<CatalogSnapshot> Read(string[] folders)
        {
            return worker.Run(delegate {
                var snapshot = new CatalogSnapshot();
                snapshot.Desktop.AddRange(ReadFolder(DesktopPath, true));
                if (PublicPath != null && Directory.Exists(PublicPath))
                    snapshot.Desktop.AddRange(ReadFolder(PublicPath, true).Where(p => !snapshot.Desktop.Any(existing => existing.Name.Equals(p.Name, StringComparison.OrdinalIgnoreCase))));
                if (PublicPath != null) snapshot.Desktop.AddRange(NamespaceItems());
                foreach (var folder in folders.Distinct(StringComparer.OrdinalIgnoreCase))
                {
                    try { snapshot.Folders[folder] = ReadFolder(folder, false); }
                    catch (IOException error) { snapshot.Errors[folder] = error.Message; }
                    catch (UnauthorizedAccessException error) { snapshot.Errors[folder] = error.Message; }
                }
                return snapshot;
            });
        }
        internal static List<DesktopEntry> ReadFolder(string folder, bool desktop)
        {
            var result = new List<DesktopEntry>();
            foreach (var path in Directory.EnumerateFileSystemEntries(folder))
            {
                try
                {
                    if ((File.GetAttributes(path) & (FileAttributes.Hidden | FileAttributes.System)) == 0) result.Add(DesktopEntry.File(path, desktop));
                }
                catch (FileNotFoundException) { }
                catch (DirectoryNotFoundException) { }
            }
            return result;
        }
        [DllImport("shell32")] private static extern int SHCreateItemWithParent(IntPtr parentId, IShellFolder parent, IntPtr childId, ref Guid iid, out IShellItem item);
        private static bool HiddenNamespace(string path)
        {
            int start = path.IndexOf('{'), end = path.IndexOf('}');
            if (start < 0 || end <= start) return false;
            string clsid = path.Substring(start, end - start + 1);
            const string root = @"Software\Microsoft\Windows\CurrentVersion\";
            foreach (RegistryKey hive in new[] { Registry.CurrentUser, Registry.LocalMachine })
                using (var policy = hive.OpenSubKey(root + @"Policies\NonEnum"))
                    if (policy != null && Convert.ToInt32(policy.GetValue(clsid, 0)) != 0) return true;
            foreach (RegistryKey hive in new[] { Registry.CurrentUser, Registry.LocalMachine })
                using (var hidden = hive.OpenSubKey(root + @"Explorer\HideDesktopIcons\NewStartPanel"))
                {
                    object value = hidden == null ? null : hidden.GetValue(clsid);
                    if (value != null) return Convert.ToInt32(value) != 0;
                }
            return false;
        }
        private static List<DesktopEntry> NamespaceItems()
        {
            var result = new List<DesktopEntry>(); IShellFolder folder = null; IEnumIDList list = null;
            try
            {
                Win32.Check(Win32.SHGetDesktopFolder(out folder));
                folder.EnumObjects(IntPtr.Zero, 0x20 | 0x40, out list);
                if (list == null) return result;
                IntPtr pidl; uint fetched;
                while (list.Next(1, out pidl, out fetched) == 0 && fetched == 1)
                {
                    IShellItem item = null;
                    try
                    {
                        Guid iid = typeof(IShellItem).GUID;
                        Win32.Check(SHCreateItemWithParent(IntPtr.Zero, folder, pidl, ref iid, out item));
                        uint attrs; item.GetAttributes(0x40000000 | 0x80000 | 0x100000, out attrs);
                        if ((attrs & (0x40000000 | 0x80000 | 0x100000)) != 0) continue;
                        string path = Win32.DisplayName(item, 0x80028000);
                        if (!HiddenNamespace(path)) result.Add(new DesktopEntry { Path = path, Name = Win32.DisplayName(item, 0), Extension = "", IsNamespace = true, IsDesktop = true });
                    }
                    finally { Win32.Release(item); Marshal.FreeCoTaskMem(pidl); }
                }
                return result;
            }
            finally { Win32.Release(list); Win32.Release(folder); }
        }
        internal Task<ImageSource> Icon(DesktopEntry entry, int size)
        {
            string key = entry.Key + "|" + entry.Modified + "|" + size;
            return icons.Run<ImageSource>(delegate {
                ImageSource image;
                if (cache.TryGetValue(key, out image)) return image;
                IShellItem item = null; IntPtr bitmap = IntPtr.Zero;
                try
                {
                    item = Win32.Item(entry.Path);
                    ((IShellItemImageFactory)item).GetImage(new Win32.SIZE { X = size, Y = size }, 0x4 | 0x1 | 0x100, out bitmap);
                    image = Imaging.CreateBitmapSourceFromHBitmap(bitmap, IntPtr.Zero, Int32Rect.Empty, BitmapSizeOptions.FromEmptyOptions());
                    image.Freeze();
                    if (cache.Count >= 1024) cache.Clear();
                    cache[key] = image;
                    return image;
                }
                finally { if (bitmap != IntPtr.Zero) Win32.DeleteObject(bitmap); Win32.Release(item); }
            });
        }
        public void Dispose() { worker.Dispose(); icons.Dispose(); }
    }
}
