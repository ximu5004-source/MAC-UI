using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Threading;
using MacUI.Desktop.Native;

internal static class RecoveryTests
{
    internal static int Inspect()
    {
        uint shell; Win32.GetWindowThreadProcessId(Win32.GetShellWindow(), out shell);
        Win32.EnumWindows(delegate(IntPtr host, IntPtr unused) {
            uint process; Win32.GetWindowThreadProcessId(host, out process);
            if (process != shell || (Win32.Class(host) != "WorkerW" && Win32.Class(host) != "Progman")) return true;
            Console.WriteLine("Host " + host + " " + Win32.Class(host) + " visible=" + Win32.IsWindowVisible(host));
            Win32.EnumChildWindows(host, delegate(IntPtr child, IntPtr ignored) {
                Console.WriteLine("Child " + child + " " + Win32.Class(child) + " parent=" + Win32.Class(Win32.GetParent(child)) + " visible=" + Win32.IsWindowVisible(child));
                return true;
            }, IntPtr.Zero);
            return true;
        }, IntPtr.Zero);
        return 0;
    }
    private static Dictionary<IntPtr, bool> ExplorerIcons()
    {
        uint shell;
        Win32.GetWindowThreadProcessId(Win32.GetShellWindow(), out shell);
        var result = new Dictionary<IntPtr, bool>();
        Win32.EnumWindows(delegate(IntPtr host, IntPtr unused) {
            uint process;
            Win32.GetWindowThreadProcessId(host, out process);
            if (process != shell) return true;
            var view = Win32.FindWindowEx(host, IntPtr.Zero, "SHELLDLL_DefView", null);
            if (view == IntPtr.Zero) return true;
            var icons = Win32.FindWindowEx(view, IntPtr.Zero, "SysListView32", null);
            if (icons != IntPtr.Zero) result[icons] = Win32.IsWindowVisible(icons);
            return true;
        }, IntPtr.Zero);
        return result;
    }

    internal static int Run(string executable)
    {
        string root = Path.Combine(Path.GetTempPath(), "macui-desktop-recovery-test-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        string files = Path.Combine(root, "files"); Directory.CreateDirectory(files);
        File.WriteAllText(Path.Combine(files, "test.txt"), "MAC UI recovery test");
        var before = ExplorerIcons();
        if (before.Count == 0) throw new InvalidOperationException("Explorer desktop was not found.");
        try
        {
            foreach (var item in before) Win32.ShowWindowAsync(item.Key, 5);
            if (!SpinWait.SpinUntil(() => before.Keys.All(Win32.IsWindowVisible), 5000)) throw new Exception("Explorer icons could not be shown for the recovery test");
            foreach (bool crash in new[] { false, true })
            {
                string config = Path.Combine(root, crash ? "crash" : "normal");
                var info = new ProcessStartInfo(Path.GetFullPath(executable)) {
                    UseShellExecute = false, CreateNoWindow = true, WindowStyle = ProcessWindowStyle.Hidden,
                    RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true,
                    Arguments = "--desktop --root " + DesktopRecovery.Quote(files) + " --data " + DesktopRecovery.Quote(config) + " --parent " + Process.GetCurrentProcess().Id + " --locale zh-CN"
                };
                using (var process = Process.Start(info))
                {
                    try
                    {
                        while (true)
                        {
                            var pending = process.StandardOutput.ReadLineAsync();
                            if (!pending.Wait(TimeSpan.FromSeconds(20))) throw new TimeoutException("Native desktop did not report ready");
                            if (pending.Result == null) throw new InvalidOperationException(process.StandardError.ReadToEnd());
                            var response = Json.Parse(pending.Result);
                            if (Json.Text(response, "type") == "error") throw new InvalidOperationException(pending.Result);
                            if (Json.Text(response, "type") == "ready") break;
                        }
                        if (!SpinWait.SpinUntil(() => before.Keys.All(hwnd => !Win32.IsWindowVisible(hwnd)), 5000)) throw new Exception("Explorer icons were not hidden");
                        if (crash) process.Kill();
                        else { process.StandardInput.WriteLine("{\"type\":\"stop\"}"); process.StandardInput.Flush(); }
                        if (!process.WaitForExit(15000)) throw new TimeoutException("Native desktop did not exit");
                        if (!crash && process.ExitCode != 0) throw new Exception("Normal exit returned an error");
                        string lease = Path.Combine(config, "native-desktop-recovery.json");
                        if (!SpinWait.SpinUntil(() => !File.Exists(lease) && before.Keys.All(hwnd => Win32.IsWindow(hwnd) && Win32.IsWindowVisible(hwnd)), 10000)) throw new Exception("Explorer visibility was not restored");
                        Console.WriteLine("PASS: native desktop " + (crash ? "crash" : "normal exit") + " restored Explorer icons and removed its recovery record");
                    }
                    finally
                    {
                        if (!process.HasExited) { process.Kill(); process.WaitForExit(); }
                    }
                }
            }
        }
        finally
        {
            foreach (var item in before)
                if (Win32.IsWindow(item.Key) && Win32.Class(item.Key) == "SysListView32" && Win32.Class(Win32.GetParent(item.Key)) == "SHELLDLL_DefView")
                    Win32.ShowWindowAsync(item.Key, item.Value ? 5 : 0);
        }
        Directory.Delete(root, true);
        return 0;
    }
}
