using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows;
using System.Windows.Input;
using System.Windows.Interop;

namespace MacUI.Desktop.Native
{
    internal sealed class NativeMenuItem
    {
        internal string Title;
        internal Action Action;
        internal bool Enabled = true, Checked;
        internal NativeMenuItem[] Children;
        internal NativeMenuItem(string title, Action action = null) { Title = title; Action = action; }
    }
    [ComImport, Guid("000214E4-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IContextMenu
    {
        [PreserveSig] int QueryContextMenu(IntPtr menu, uint index, uint first, uint last, uint flags);
        void InvokeCommand(ref ShellMenu.INVOKE info);
        [PreserveSig] int GetCommandString(UIntPtr id, uint flags, IntPtr reserved, [MarshalAs(UnmanagedType.LPArray)] byte[] text, uint size);
    }
    [ComImport, Guid("000214F4-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IContextMenu2
    {
        [PreserveSig] int QueryContextMenu(IntPtr menu, uint index, uint first, uint last, uint flags);
        void InvokeCommand(ref ShellMenu.INVOKE info);
        void GetCommandString(UIntPtr id, uint flags, IntPtr reserved, IntPtr text, uint size);
        [PreserveSig] int HandleMenuMsg(uint message, IntPtr wparam, IntPtr lparam);
    }
    [ComImport, Guid("BCFCE0A0-EC17-11D0-8D10-00A0C90F2719"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IContextMenu3
    {
        [PreserveSig] int QueryContextMenu(IntPtr menu, uint index, uint first, uint last, uint flags);
        void InvokeCommand(ref ShellMenu.INVOKE info);
        void GetCommandString(UIntPtr id, uint flags, IntPtr reserved, IntPtr text, uint size);
        [PreserveSig] int HandleMenuMsg(uint message, IntPtr wparam, IntPtr lparam);
        [PreserveSig] int HandleMenuMsg2(uint message, IntPtr wparam, IntPtr lparam, out IntPtr result);
    }
    internal static class ShellMenu
    {
        [StructLayout(LayoutKind.Sequential)] internal struct INVOKE
        {
            internal int Size; internal uint Mask; internal IntPtr Owner, Verb, Parameters, Directory;
            internal int Show, HotKey; internal IntPtr Icon, Title, VerbW, ParametersW, DirectoryW, TitleW; internal Win32.POINT Point;
        }
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] private struct EXECUTE
        {
            internal int Size; internal uint Mask; internal IntPtr Owner;
            [MarshalAs(UnmanagedType.LPWStr)] internal string Verb, File, Parameters, Directory;
            internal int Show; internal IntPtr Instance, IdList;
            [MarshalAs(UnmanagedType.LPWStr)] internal string Class;
            internal IntPtr ClassKey; internal uint HotKey; internal IntPtr Icon, Process;
        }
        [DllImport("shell32", CharSet = CharSet.Unicode, SetLastError = true)] private static extern bool ShellExecuteEx(ref EXECUTE info);
        [DllImport("user32")] private static extern IntPtr CreatePopupMenu();
        [DllImport("user32")] private static extern bool DestroyMenu(IntPtr menu);
        [DllImport("user32")] private static extern uint TrackPopupMenuEx(IntPtr menu, uint flags, int x, int y, IntPtr owner, IntPtr parameters);
        [DllImport("user32", CharSet = CharSet.Unicode, SetLastError = true)] private static extern bool AppendMenu(IntPtr menu, uint flags, UIntPtr id, string text);
        private static void Append(IntPtr menu, NativeMenuItem[] items, Dictionary<uint, Action> actions, ref uint id)
        {
            foreach (var item in items)
            {
                if (item.Title == null) { AppendMenu(menu, 0x800, UIntPtr.Zero, null); continue; }
                uint flags = item.Enabled ? 0u : 3u; if (item.Checked) flags |= 8;
                if (item.Children != null)
                {
                    var child = CreatePopupMenu(); Append(child, item.Children, actions, ref id);
                    if (!AppendMenu(menu, flags | 0x10, new UIntPtr((ulong)child.ToInt64()), item.Title)) { DestroyMenu(child); throw new Win32Exception(Marshal.GetLastWin32Error()); }
                }
                else { actions.Add(id, item.Action); if (!AppendMenu(menu, flags, new UIntPtr(id++), item.Title)) throw new Win32Exception(Marshal.GetLastWin32Error()); }
            }
        }
        internal static void Custom(Window window, NativeMenuItem[] items)
        {
            IntPtr owner = new WindowInteropHelper(window).Handle, menu = CreatePopupMenu();
            Action action = null;
            try
            {
                var actions = new Dictionary<uint, Action>(); uint id = 0x8000; Append(menu, items, actions, ref id);
                Win32.POINT point; Win32.GetCursorPos(out point); Win32.SetForegroundWindow(owner);
                uint selected = TrackPopupMenuEx(menu, 0x100 | 0x2, point.X, point.Y, owner, IntPtr.Zero);
                actions.TryGetValue(selected, out action);
            }
            finally { DestroyMenu(menu); Win32.PostMessage(owner, 0, IntPtr.Zero, IntPtr.Zero); }
            if (action != null) action();
        }
        internal static void Open(IntPtr owner, string path, string verb = "open")
        {
            IntPtr pidl; uint attributes;
            Win32.Check(Win32.SHParseDisplayName(path, IntPtr.Zero, out pidl, 0, out attributes));
            try
            {
                var info = new EXECUTE { Size = Marshal.SizeOf(typeof(EXECUTE)), Owner = owner, Verb = verb, IdList = pidl, Mask = 0xC | 0x100, Show = 1 };
                if (!ShellExecuteEx(ref info)) throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            finally { Marshal.FreeCoTaskMem(pidl); }
        }
        internal static bool Show(Window window, string[] paths)
        {
            if (paths.Length == 0) return false;
            return ShowContext(window, paths, null, null);
        }
        internal static void Background(Window window, string title, NativeMenuItem[] items)
        { ShowContext(window, null, title, items); }
        internal static void Populate(IContextMenu context, IntPtr menu, bool background, bool extended)
        {
            uint flags = background ? 0u : 0x10u | 0x80u;
            if (extended) flags |= 0x100;
            Win32.Check(context.QueryContextMenu(menu, 0, 1, 0x7FFF, flags));
        }
        private static bool ShowContext(Window window, string[] paths, string title, NativeMenuItem[] items)
        {
            IntPtr owner = new WindowInteropHelper(window).Handle;
            ShellSelection selection = null; IntPtr menu = IntPtr.Zero;
            HwndSource source = HwndSource.FromHwnd(owner); HwndSourceHook hook = null;
            string operation = "ShellSelection.Create";
            try
            {
                selection = paths == null ? ShellSelection.Background(owner) : ShellSelection.Create(owner, paths);
                var context = selection.Menu; var second = context as IContextMenu2; var third = context as IContextMenu3;
                hook = delegate(IntPtr hwnd, int message, IntPtr wparam, IntPtr lparam, ref bool handled) {
                    if (message == 0x117 || message == 0x2B || message == 0x2C || message == 0x120)
                    {
                        IntPtr result;
                        if (third != null && third.HandleMenuMsg2((uint)message, wparam, lparam, out result) >= 0) { handled = true; return result; }
                        if (second != null && message != 0x120 && second.HandleMenuMsg((uint)message, wparam, lparam) >= 0) handled = true;
                    }
                    return IntPtr.Zero;
                };
                source.AddHook(hook);
                menu = CreatePopupMenu();
                if (menu == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
                operation = "IContextMenu.QueryContextMenu";
                Populate(context, menu, paths == null, (Keyboard.Modifiers & ModifierKeys.Shift) != 0);
                var actions = new Dictionary<uint, Action>();
                if (items != null)
                {
                    uint id = 0x8000;
                    Append(menu, new[] { new NativeMenuItem(null), new NativeMenuItem(title) { Children = items } }, actions, ref id);
                }
                Win32.POINT point; Win32.GetCursorPos(out point);
                Win32.SetForegroundWindow(owner);
                uint selected = TrackPopupMenuEx(menu, 0x100 | 0x2, point.X, point.Y, owner, IntPtr.Zero);
                if (selected == 0) return false;
                Action action;
                if (actions.TryGetValue(selected, out action)) { if (action != null) window.Dispatcher.BeginInvoke(action); return false; }
                byte[] verb = new byte[512];
                int verbResult = context.GetCommandString(new UIntPtr(selected - 1), 4, IntPtr.Zero, verb, 256);
                string name = verbResult >= 0 ? Encoding.Unicode.GetString(verb).TrimEnd('\0') : "";
                if (name.Equals("rename", StringComparison.OrdinalIgnoreCase)) return true;
                var command = new INVOKE { Size = Marshal.SizeOf(typeof(INVOKE)), Mask = 0x4000 | 0x20000000, Owner = owner,
                    Verb = new IntPtr(selected - 1), VerbW = new IntPtr(selected - 1), Show = 1, Point = point };
                operation = "IContextMenu.InvokeCommand";
                context.InvokeCommand(ref command);
                return false;
            }
            catch (Exception error)
            {
                if (!error.Data.Contains("nativeOperation")) error.Data["nativeOperation"] = operation;
                throw;
            }
            finally
            {
                if (hook != null) source.RemoveHook(hook);
                if (menu != IntPtr.Zero) DestroyMenu(menu);
                if (selection != null) selection.Dispose();
                Win32.PostMessage(owner, 0, IntPtr.Zero, IntPtr.Zero);
            }
        }
    }
}
