using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Security.Principal;
using System.Threading;

namespace MacUI.Desktop.Native
{
    internal sealed class DesktopRecovery : IDisposable
    {
        private readonly string directory, token, leasePath;
        private readonly Mutex visibility;
        private readonly List<Dictionary<string, object>> hidden = new List<Dictionary<string, object>>();
        private bool acquired;
        internal static string SessionName { get { return WindowsIdentity.GetCurrent().User.Value + "." + Process.GetCurrentProcess().SessionId; } }
        internal DesktopRecovery(string directory)
        {
            this.directory = directory;
            token = Guid.NewGuid().ToString("N");
            leasePath = Path.Combine(directory, "native-desktop-recovery.json");
            visibility = new Mutex(false, @"Local\macui.desktop.visibility." + SessionName);
        }
        private static void Enter(Mutex mutex)
        { try { mutex.WaitOne(); } catch (AbandonedMutexException) { } }
        private static bool Alive(Dictionary<string, object> lease)
        {
            try
            {
                using (var process = Process.GetProcessById((int)Json.Number(lease, "pid", 0)))
                    return !process.HasExited && process.StartTime.ToUniversalTime().Ticks == Int64.Parse(Json.Text(lease, "started"));
            }
            catch (ArgumentException) { return false; }
        }
        private static void Restore(Dictionary<string, object> lease)
        {
            foreach (var value in Json.Array(lease, "hidden").Cast<Dictionary<string, object>>())
            {
                var hwnd = new IntPtr(Int64.Parse(Json.Text(value, "hwnd")));
                uint pid; Win32.GetWindowThreadProcessId(hwnd, out pid);
                if (Win32.IsWindow(hwnd) && Win32.Class(hwnd) == "SysListView32" && Win32.Class(Win32.GetParent(hwnd)) == "SHELLDLL_DefView" && pid == (uint)Json.Number(value, "pid", 0))
                    Win32.ShowWindowAsync(hwnd, 5);
            }
        }
        internal void Acquire()
        {
            Enter(visibility);
            try
            {
                if (File.Exists(leasePath))
                {
                    var old = Json.Parse(File.ReadAllText(leasePath));
                    if (Alive(old)) throw new InvalidOperationException("A native desktop session is already running");
                    Restore(old);
                }
                SaveLease(); acquired = true;
            }
            finally { visibility.ReleaseMutex(); }
            using (var ready = new EventWaitHandle(false, EventResetMode.ManualReset, @"Local\macui.desktop.watchdog." + token))
            {
                var info = new ProcessStartInfo(Process.GetCurrentProcess().MainModule.FileName) {
                    Arguments = "--watchdog " + Quote(directory) + " " + token,
                    UseShellExecute = false, CreateNoWindow = true, WindowStyle = ProcessWindowStyle.Hidden };
                using (var process = Process.Start(info))
                    if (!ready.WaitOne(TimeSpan.FromSeconds(10))) throw new TimeoutException("Desktop recovery did not start");
            }
            HideExplorerIcons();
        }
        internal static string Quote(string value) { return "\"" + value.Replace("\"", "\\\"").TrimEnd('\\') + "\""; }
        private void SaveLease()
        {
            using (var process = Process.GetCurrentProcess())
                Json.AtomicWrite(leasePath, Json.Write(new { token = token, pid = process.Id, started = process.StartTime.ToUniversalTime().Ticks.ToString(), hidden = hidden }), null);
        }
        internal void HideExplorerIcons()
        {
            if (!acquired) return;
            Enter(visibility);
            try
            {
                uint shellPid; Win32.GetWindowThreadProcessId(Win32.GetShellWindow(), out shellPid);
                var candidates = new List<IntPtr>();
                Win32.EnumWindows(delegate(IntPtr host, IntPtr unused) {
                    uint pid; Win32.GetWindowThreadProcessId(host, out pid);
                    if (pid != shellPid) return true;
                    var view = Win32.FindWindowEx(host, IntPtr.Zero, "SHELLDLL_DefView", null);
                    if (view != IntPtr.Zero)
                    {
                        var icons = Win32.FindWindowEx(view, IntPtr.Zero, "SysListView32", null);
                        if (icons != IntPtr.Zero && Win32.IsWindowVisible(icons)) candidates.Add(icons);
                    }
                    return true;
                }, IntPtr.Zero);
                foreach (var hwnd in candidates)
                {
                    if (!hidden.Any(item => Json.Text(item, "hwnd") == hwnd.ToInt64().ToString()))
                        hidden.Add(new Dictionary<string, object> { { "hwnd", hwnd.ToInt64().ToString() }, { "pid", shellPid } });
                }
                if (candidates.Count > 0) SaveLease();
                foreach (var hwnd in candidates) Win32.ShowWindowAsync(hwnd, 0);
            }
            finally { visibility.ReleaseMutex(); }
        }
        public void Dispose()
        {
            if (acquired)
            {
                Enter(visibility);
                try
                {
                    if (File.Exists(leasePath))
                    {
                        var lease = Json.Parse(File.ReadAllText(leasePath));
                        if (Json.Text(lease, "token") == token) { Restore(lease); File.Delete(leasePath); }
                    }
                    acquired = false;
                }
                finally { visibility.ReleaseMutex(); }
            }
            visibility.Dispose();
        }
        internal static int Watchdog(string directory, string token)
        {
            string path = Path.Combine(Path.GetFullPath(directory), "native-desktop-recovery.json");
            if (!File.Exists(path)) return 0;
            var lease = Json.Parse(File.ReadAllText(path));
            if (Json.Text(lease, "token") != token) return 0;
            Process owner = null;
            try { owner = Process.GetProcessById((int)Json.Number(lease, "pid", 0)); }
            catch (ArgumentException) { }
            using (var ready = EventWaitHandle.OpenExisting(@"Local\macui.desktop.watchdog." + token)) ready.Set();
            if (owner != null)
                using (owner) if (owner.StartTime.ToUniversalTime().Ticks == Int64.Parse(Json.Text(lease, "started"))) owner.WaitForExit();
            using (var mutex = new Mutex(false, @"Local\macui.desktop.visibility." + SessionName))
            {
                Enter(mutex);
                try
                {
                    if (File.Exists(path))
                    {
                        lease = Json.Parse(File.ReadAllText(path));
                        if (Json.Text(lease, "token") == token) { Restore(lease); File.Delete(path); }
                    }
                }
                finally { mutex.ReleaseMutex(); }
            }
            return 0;
        }
    }
}
