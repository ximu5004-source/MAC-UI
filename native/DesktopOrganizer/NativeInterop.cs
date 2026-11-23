using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;

namespace MacUI.Desktop.Native
{
    internal static class Win32
    {
        [StructLayout(LayoutKind.Sequential)] internal struct RECT { public int Left, Top, Right, Bottom; }
        [StructLayout(LayoutKind.Sequential)] internal struct POINT { public int X, Y; }
        [StructLayout(LayoutKind.Sequential)] internal struct SIZE { public int X, Y; }
        [StructLayout(LayoutKind.Sequential)] internal struct WINDOWPOS { public IntPtr Hwnd, After; public int X, Y, Cx, Cy; public uint Flags; }
        [StructLayout(LayoutKind.Sequential)] internal struct MARGINS { public int Left, Right, Top, Bottom; }
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] internal struct MONITORINFO
        { public int Size; public RECT Monitor, Work; public uint Flags; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string Device; }
        internal delegate bool EnumWindow(IntPtr hwnd, IntPtr parameter);
        internal delegate bool EnumMonitor(IntPtr monitor, IntPtr dc, ref RECT rect, IntPtr parameter);
        [DllImport("user32")] internal static extern bool EnumWindows(EnumWindow callback, IntPtr parameter);
        [DllImport("user32")] internal static extern bool EnumChildWindows(IntPtr parent, EnumWindow callback, IntPtr parameter);
        [DllImport("user32")] internal static extern bool EnumDisplayMonitors(IntPtr dc, IntPtr clip, EnumMonitor callback, IntPtr parameter);
        [DllImport("user32", CharSet = CharSet.Unicode)] internal static extern bool GetMonitorInfo(IntPtr monitor, ref MONITORINFO info);
        [DllImport("shcore")] internal static extern int GetDpiForMonitor(IntPtr monitor, int type, out uint x, out uint y);
        [DllImport("user32", CharSet = CharSet.Unicode)] internal static extern int GetClassName(IntPtr hwnd, StringBuilder text, int count);
        [DllImport("user32", CharSet = CharSet.Unicode)] internal static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string className, string title);
        [DllImport("user32")] internal static extern IntPtr GetShellWindow();
        [DllImport("user32")] internal static extern IntPtr GetParent(IntPtr hwnd);
        [DllImport("user32")] internal static extern IntPtr GetTopWindow(IntPtr parent);
        [DllImport("user32")] internal static extern IntPtr GetWindow(IntPtr hwnd, uint command);
        [DllImport("user32")] internal static extern bool IsWindow(IntPtr hwnd);
        [DllImport("user32")] internal static extern bool IsWindowVisible(IntPtr hwnd);
        [DllImport("user32")] internal static extern bool IsIconic(IntPtr hwnd);
        [DllImport("user32")] internal static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
        [DllImport("user32")] internal static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
        [DllImport("user32", SetLastError = true)] internal static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int width, int height, uint flags);
        [DllImport("user32")] internal static extern bool ShowWindowAsync(IntPtr hwnd, int command);
        [DllImport("user32", EntryPoint = "GetWindowLongPtrW")] internal static extern IntPtr GetWindowLongPtr(IntPtr hwnd, int index);
        [DllImport("user32", EntryPoint = "SetWindowLongPtrW")] internal static extern IntPtr SetWindowLongPtr(IntPtr hwnd, int index, IntPtr value);
        [DllImport("user32")] internal static extern bool SetForegroundWindow(IntPtr hwnd);
        [DllImport("user32")] internal static extern bool GetCursorPos(out POINT point);
        [DllImport("user32")] internal static extern uint GetClipboardSequenceNumber();
        [DllImport("user32")] internal static extern bool PostMessage(IntPtr hwnd, uint message, IntPtr wparam, IntPtr lparam);
        [DllImport("dwmapi")] internal static extern int DwmGetWindowAttribute(IntPtr hwnd, int attribute, out int value, int size);
        [DllImport("dwmapi")] internal static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int size);
        [DllImport("dwmapi")] internal static extern int DwmExtendFrameIntoClientArea(IntPtr hwnd, ref MARGINS margins);
        [DllImport("gdi32")] internal static extern bool DeleteObject(IntPtr handle);
        [DllImport("shell32", CharSet = CharSet.Unicode)] internal static extern int SHCreateItemFromParsingName(string path, IntPtr bindContext, ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out object item);
        [DllImport("shell32", CharSet = CharSet.Unicode)] internal static extern int SHParseDisplayName(string path, IntPtr bindContext, out IntPtr pidl, uint mask, out uint attributes);
        [DllImport("shell32")] internal static extern int SHCreateShellItemArrayFromIDLists(uint count, IntPtr[] pidls, out IShellItemArray items);
        [DllImport("shell32")] internal static extern int SHGetDesktopFolder(out IShellFolder folder);
        [DllImport("shlwapi", CharSet = CharSet.Unicode)] internal static extern int StrCmpLogicalW(string first, string second);
        internal static void Check(int result) { if (result < 0) Marshal.ThrowExceptionForHR(result); }
        internal static string Class(IntPtr hwnd) { var text = new StringBuilder(256); GetClassName(hwnd, text, text.Capacity); return text.ToString(); }
        internal static bool Namespace(string path) { return path.StartsWith("::", StringComparison.Ordinal) || path.StartsWith("shell:", StringComparison.OrdinalIgnoreCase); }
        internal static IShellItem Item(string path)
        { Guid iid = typeof(IShellItem).GUID; object item; Check(SHCreateItemFromParsingName(path, IntPtr.Zero, ref iid, out item)); return (IShellItem)item; }
        internal static string DisplayName(IShellItem item, uint kind)
        { IntPtr text; item.GetDisplayName(kind, out text); try { return Marshal.PtrToStringUni(text); } finally { Marshal.FreeCoTaskMem(text); } }
        internal static void Release(object value) { if (value != null && Marshal.IsComObject(value)) Marshal.ReleaseComObject(value); }
    }

    [ComImport, Guid("43826D1E-E718-42EE-BC55-A1E261C37BFE"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IShellItem
    {
        void BindToHandler(IntPtr context, ref Guid handler, ref Guid iid, out IntPtr result);
        void GetParent(out IShellItem parent);
        void GetDisplayName(uint kind, out IntPtr name);
        void GetAttributes(uint mask, out uint attributes);
        void Compare(IShellItem other, uint hint, out int order);
    }
    [ComImport, Guid("BCC18B79-BA16-442F-80C4-8A59C30C463B"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IShellItemImageFactory { void GetImage(Win32.SIZE size, uint flags, out IntPtr bitmap); }
    [ComImport, Guid("B63EA76D-1F85-456F-A19C-48159EFA858B"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IShellItemArray
    {
        void BindToHandler(IntPtr context, ref Guid handler, ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out object result);
        void GetPropertyStore(int flags, ref Guid iid, out IntPtr result);
        void GetPropertyDescriptionList(IntPtr key, ref Guid iid, out IntPtr result);
        void GetAttributes(uint flags, uint mask, out uint attributes);
        void GetCount(out uint count);
        void GetItemAt(uint index, out IShellItem item);
        void EnumItems(out IntPtr enumerator);
    }
    [ComImport, Guid("000214E6-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IShellFolder
    {
        void ParseDisplayName(IntPtr hwnd, IntPtr context, [MarshalAs(UnmanagedType.LPWStr)] string name, out uint eaten, out IntPtr pidl, ref uint attributes);
        void EnumObjects(IntPtr hwnd, uint flags, out IEnumIDList list);
        void BindToObject(IntPtr pidl, IntPtr context, ref Guid iid, out IntPtr result);
        void BindToStorage(IntPtr pidl, IntPtr context, ref Guid iid, out IntPtr result);
        [PreserveSig] int CompareIDs(IntPtr hint, IntPtr first, IntPtr second);
        void CreateViewObject(IntPtr hwnd, ref Guid iid, out IntPtr result);
        void GetAttributesOf(uint count, [MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 0)] IntPtr[] pidls, ref uint attributes);
        void GetUIObjectOf(IntPtr hwnd, uint count, [MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 1)] IntPtr[] pidls, ref Guid iid, IntPtr reserved, [MarshalAs(UnmanagedType.Interface)] out object result);
        void GetDisplayNameOf(IntPtr pidl, uint flags, IntPtr name);
        void SetNameOf(IntPtr hwnd, IntPtr pidl, [MarshalAs(UnmanagedType.LPWStr)] string name, uint flags, out IntPtr renamed);
    }
    [ComImport, Guid("000214F2-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IEnumIDList
    {
        [PreserveSig] int Next(uint count, out IntPtr pidl, out uint fetched);
        void Skip(uint count); void Reset(); void Clone(out IEnumIDList other);
    }
}
