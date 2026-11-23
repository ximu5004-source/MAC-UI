using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Interop;
using System.Windows.Threading;

namespace MacUI.Desktop.Native
{
    [ComImport, Guid("6D5140C1-7436-11CE-8034-00AA006009FA"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IShellServiceProvider
    { void QueryService(ref Guid service, ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out object value); }
    [ComImport, Guid("000214E2-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IShellBrowser
    {
        void GetWindow(out IntPtr hwnd);
        void ContextSensitiveHelp(bool enter);
        void InsertMenusSB(IntPtr menu, IntPtr widths);
        void SetMenuSB(IntPtr menu, IntPtr oleMenu, IntPtr active);
        void RemoveMenusSB(IntPtr menu);
        void SetStatusTextSB([MarshalAs(UnmanagedType.LPWStr)] string text);
        void EnableModelessSB(bool enable);
        void TranslateAcceleratorSB(IntPtr message, ushort id);
        void BrowseObject(IntPtr pidl, uint flags);
        void GetViewStateStream(uint mode, out IntPtr stream);
        void GetControlWindow(uint id, out IntPtr hwnd);
        void SendControlMsg(uint id, uint message, IntPtr wparam, IntPtr lparam, out IntPtr result);
        void QueryActiveShellView(out IShellView view);
    }
    [ComImport, Guid("000214E3-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IShellView
    {
        void GetWindow(out IntPtr hwnd);
        void ContextSensitiveHelp(bool enter);
        void TranslateAccelerator(IntPtr message);
        void EnableModeless(bool enable);
        void UIActivate(uint state);
        void Refresh();
        void CreateViewWindow(IntPtr previous, IntPtr settings, IntPtr browser, IntPtr bounds, out IntPtr hwnd);
        void DestroyViewWindow();
        void GetCurrentInfo(IntPtr settings);
        void AddPropertySheetPages(uint reserved, IntPtr callback, IntPtr data);
        void SaveViewState();
        void SelectItem(IntPtr pidl, uint flags);
        void GetItemObject(uint scope, ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out object value);
    }
    [StructLayout(LayoutKind.Sequential)]
    internal struct FolderSettings { internal uint ViewMode, Flags; }
    [ComImport, Guid("DFD3B6B5-C10C-4BE9-85F6-A66969F402F6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IExplorerBrowser
    {
        void Initialize(IntPtr parent, ref Win32.RECT rect, ref FolderSettings settings);
        void Destroy();
        void SetRect(IntPtr deferred, Win32.RECT rect);
        void SetPropertyBag([MarshalAs(UnmanagedType.LPWStr)] string name);
        void SetEmptyText([MarshalAs(UnmanagedType.LPWStr)] string text);
        void SetFolderSettings(ref FolderSettings settings);
        void Advise(IntPtr events, out uint cookie);
        void Unadvise(uint cookie);
        void SetOptions(uint flags);
        void GetOptions(out uint flags);
        void BrowseToIDList(IntPtr pidl, uint flags);
        void BrowseToObject([MarshalAs(UnmanagedType.IUnknown)] object value, uint flags);
        void FillFromObject([MarshalAs(UnmanagedType.IUnknown)] object value, uint flags);
        void RemoveAll();
        [PreserveSig] int GetCurrentView(ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out object view);
    }
    [ComImport, Guid("CDE725B0-CCC9-4519-917E-325D72FAB4CE"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IFolderView
    {
        void GetCurrentViewMode(out uint mode);
        void SetCurrentViewMode(uint mode);
        void GetFolder(ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out object folder);
    }
    [ComImport, Guid("96E5AE6D-6AE1-4B1C-900C-C6480EAA8828"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IResultsFolder
    {
        void AddItem(IShellItem item);
        void AddIDList(IntPtr absolute, out IntPtr child);
        void RemoveItem(IShellItem item);
        void RemoveIDList(IntPtr absolute);
        void RemoveAll();
    }
    internal sealed class ShellSelection : IDisposable
    {
        private readonly List<IntPtr> children = new List<IntPtr>();
        private HwndSource host;
        private IExplorerBrowser browser;
        private IShellItemArray items;
        private object view, folder;
        private bool initialized;
        internal IContextMenu Menu { get; private set; }
        internal static ShellSelection Background(IntPtr owner)
        {
            var selection = new ShellSelection();
            object shellWindows = null, desktop = null, shellBrowser = null;
            string operation = "ShellWindows.Create";
            try
            {
                shellWindows = Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("9BA05972-F6A8-11CF-A442-00A0C90A8F39")));
                object location = 0, root = null; int hwnd;
                operation = "ShellWindows.FindWindowSW(CSIDL_DESKTOP)";
                desktop = ((dynamic)shellWindows).FindWindowSW(ref location, ref root, 8, out hwnd, 1);
                if (desktop == null) throw new InvalidOperationException("Windows desktop view is unavailable");
                var service = new Guid("4C96BE40-915C-11CF-99D3-00AA004AE837"); var browserId = typeof(IShellBrowser).GUID;
                operation = "Desktop.QueryService(IShellBrowser)";
                ((IShellServiceProvider)desktop).QueryService(ref service, ref browserId, out shellBrowser);
                operation = "Desktop.QueryActiveShellView";
                IShellView desktopView; ((IShellBrowser)shellBrowser).QueryActiveShellView(out desktopView); selection.view = desktopView;
                Guid iid = typeof(IContextMenu).GUID; object context;
                operation = "Desktop.GetItemObject(SVGIO_BACKGROUND)";
                desktopView.GetItemObject(0, ref iid, out context); selection.Menu = (IContextMenu)context;
                return selection;
            }
            catch (Exception error) { error.Data["nativeOperation"] = operation; selection.Dispose(); throw; }
            finally { Win32.Release(shellBrowser); Win32.Release(desktop); Win32.Release(shellWindows); }
        }
        internal static ShellSelection Create(IntPtr owner, string[] paths)
        {
            var selection = new ShellSelection();
            var absolute = new List<IntPtr>();
            try
            {
                foreach (string path in paths)
                {
                    IntPtr id; uint attributes;
                    Win32.Check(Win32.SHParseDisplayName(path, IntPtr.Zero, out id, 0, out attributes)); absolute.Add(id);
                }
                if (paths.Length == 1)
                {
                    Win32.Check(Win32.SHCreateShellItemArrayFromIDLists((uint)absolute.Count, absolute.ToArray(), out selection.items));
                    Guid handler = new Guid("3981E225-F559-11D3-8E3A-00C04F6837D5"), iid = typeof(IContextMenu).GUID;
                    object menu; selection.items.BindToHandler(IntPtr.Zero, ref handler, ref iid, out menu); selection.Menu = (IContextMenu)menu;
                }
                else selection.CreateResults(owner, absolute);
                return selection;
            }
            catch { selection.Dispose(); throw; }
            finally { foreach (IntPtr id in absolute) Marshal.FreeCoTaskMem(id); }
        }
        private void CreateResults(IntPtr owner, List<IntPtr> absolute)
        {
            host = new HwndSource(new HwndSourceParameters("macui.desktop.selection") { ParentWindow = owner, WindowStyle = 0x40000000, Width = 1, Height = 1 });
            browser = (IExplorerBrowser)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("71F96385-DDD6-48D3-A0C1-AE06E8B055FB")));
            var rect = new Win32.RECT { Right = 1, Bottom = 1 }; var settings = new FolderSettings { ViewMode = 4, Flags = 0x1 | 0x10000 | 0x10000000 };
            browser.Initialize(host.Handle, ref rect, ref settings); initialized = true;
            browser.SetOptions(0x1 | 0x8 | 0x40 | 0x80); browser.FillFromObject(null, 0x200);
            Guid viewId = typeof(IFolderView).GUID; int result = browser.GetCurrentView(ref viewId, out view);
            if (result < 0)
            {
                var frame = new DispatcherFrame(); var elapsed = Stopwatch.StartNew();
                var timer = new DispatcherTimer(DispatcherPriority.Background) { Interval = TimeSpan.FromMilliseconds(10) };
                timer.Tick += delegate { result = browser.GetCurrentView(ref viewId, out view); if (result >= 0 || elapsed.ElapsedMilliseconds >= 1500) frame.Continue = false; };
                timer.Start(); try { Dispatcher.PushFrame(frame); } finally { timer.Stop(); }
                Win32.Check(result);
            }
            Guid folderId = typeof(IShellFolder).GUID; ((IFolderView)view).GetFolder(ref folderId, out folder);
            var results = (IResultsFolder)folder;
            foreach (IntPtr id in absolute) { IntPtr child; results.AddIDList(id, out child); children.Add(child); }
            Guid menuId = typeof(IContextMenu).GUID; object context;
            ((IShellFolder)folder).GetUIObjectOf(owner, (uint)children.Count, children.ToArray(), ref menuId, IntPtr.Zero, out context);
            Menu = (IContextMenu)context;
        }
        public void Dispose()
        {
            Win32.Release(Menu); Menu = null;
            foreach (IntPtr child in children) Marshal.FreeCoTaskMem(child); children.Clear();
            Win32.Release(folder); Win32.Release(view); Win32.Release(items);
            if (initialized) browser.Destroy(); Win32.Release(browser);
            if (host != null) host.Dispose();
        }
    }
}
