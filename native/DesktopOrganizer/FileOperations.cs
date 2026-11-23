using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Windows.Threading;

namespace MacUI.Desktop.Native
{
    internal sealed class StaWorker : IDisposable
    {
        private readonly Thread thread;
        private readonly Dispatcher dispatcher;
        internal StaWorker(string name)
        {
            Dispatcher created = null;
            using (var ready = new ManualResetEventSlim())
            {
                thread = new Thread(delegate() { created = Dispatcher.CurrentDispatcher; ready.Set(); Dispatcher.Run(); });
                thread.IsBackground = true;
                thread.Name = name;
                thread.SetApartmentState(ApartmentState.STA);
                thread.Start();
                ready.Wait();
            }
            dispatcher = created;
        }
        internal Task<T> Run<T>(Func<T> action)
        {
            var completion = new TaskCompletionSource<T>();
            dispatcher.BeginInvoke(new Action(delegate {
                try { completion.SetResult(action()); }
                catch (Exception error) { completion.SetException(error); }
            }));
            return completion.Task;
        }
        public void Dispose() { dispatcher.BeginInvokeShutdown(DispatcherPriority.Background); }
    }

    [ComImport, Guid("947AAB5F-0A5C-4C13-B4D6-4BF7836FC9F8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IFileOperation
    {
        void Advise(IFileOperationProgressSink sink, out uint cookie);
        void Unadvise(uint cookie);
        void SetOperationFlags(uint flags);
        void SetProgressMessage([MarshalAs(UnmanagedType.LPWStr)] string message);
        void SetProgressDialog(IntPtr dialog);
        void SetProperties(IntPtr changes);
        void SetOwnerWindow(IntPtr owner);
        void ApplyPropertiesToItem(IShellItem item);
        void ApplyPropertiesToItems([MarshalAs(UnmanagedType.IUnknown)] object items);
        void RenameItem(IShellItem item, [MarshalAs(UnmanagedType.LPWStr)] string name, IFileOperationProgressSink sink);
        void RenameItems([MarshalAs(UnmanagedType.IUnknown)] object items, [MarshalAs(UnmanagedType.LPWStr)] string name);
        void MoveItem(IShellItem item, IShellItem destination, [MarshalAs(UnmanagedType.LPWStr)] string name, IFileOperationProgressSink sink);
        void MoveItems([MarshalAs(UnmanagedType.IUnknown)] object items, IShellItem destination);
        void CopyItem(IShellItem item, IShellItem destination, [MarshalAs(UnmanagedType.LPWStr)] string name, IFileOperationProgressSink sink);
        void CopyItems([MarshalAs(UnmanagedType.IUnknown)] object items, IShellItem destination);
        void DeleteItem(IShellItem item, IFileOperationProgressSink sink);
        void DeleteItems([MarshalAs(UnmanagedType.IUnknown)] object items);
        void NewItem(IShellItem destination, uint attributes, [MarshalAs(UnmanagedType.LPWStr)] string name, [MarshalAs(UnmanagedType.LPWStr)] string template, IFileOperationProgressSink sink);
        [PreserveSig] int PerformOperations();
        void GetAnyOperationsAborted([MarshalAs(UnmanagedType.Bool)] out bool aborted);
    }

    [ComVisible(true), Guid("04B0F1A7-9490-44BC-96E1-4296A31252E2"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IFileOperationProgressSink
    {
        [PreserveSig] int StartOperations();
        [PreserveSig] int FinishOperations(int result);
        [PreserveSig] int PreRenameItem(uint flags, IShellItem item, [MarshalAs(UnmanagedType.LPWStr)] string name);
        [PreserveSig] int PostRenameItem(uint flags, IShellItem item, [MarshalAs(UnmanagedType.LPWStr)] string name, int result, IShellItem renamed);
        [PreserveSig] int PreMoveItem(uint flags, IShellItem item, IShellItem destination, [MarshalAs(UnmanagedType.LPWStr)] string name);
        [PreserveSig] int PostMoveItem(uint flags, IShellItem item, IShellItem destination, [MarshalAs(UnmanagedType.LPWStr)] string name, int result, IShellItem moved);
        [PreserveSig] int PreCopyItem(uint flags, IShellItem item, IShellItem destination, [MarshalAs(UnmanagedType.LPWStr)] string name);
        [PreserveSig] int PostCopyItem(uint flags, IShellItem item, IShellItem destination, [MarshalAs(UnmanagedType.LPWStr)] string name, int result, IShellItem copied);
        [PreserveSig] int PreDeleteItem(uint flags, IShellItem item);
        [PreserveSig] int PostDeleteItem(uint flags, IShellItem item, int result, IShellItem recycled);
        [PreserveSig] int PreNewItem(uint flags, IShellItem destination, [MarshalAs(UnmanagedType.LPWStr)] string name);
        [PreserveSig] int PostNewItem(uint flags, IShellItem destination, [MarshalAs(UnmanagedType.LPWStr)] string name, [MarshalAs(UnmanagedType.LPWStr)] string template, uint attributes, int result, IShellItem created);
        [PreserveSig] int UpdateProgress(uint total, uint completed);
        [PreserveSig] int ResetTimer();
        [PreserveSig] int PauseTimer();
        [PreserveSig] int ResumeTimer();
    }

    internal sealed class TransferResult
    {
        internal readonly Dictionary<string, string> Paths = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        internal bool Cancelled;
        internal int Error;
        internal void ThrowIfFailed() { if (Error < 0) Marshal.ThrowExceptionForHR(Error); }
    }

    [ComVisible(true), ClassInterface(ClassInterfaceType.None)]
    internal sealed class OperationSink : IFileOperationProgressSink
    {
        private readonly string source;
        private readonly TransferResult outcome;
        internal OperationSink(string source, TransferResult outcome) { this.source = source; this.outcome = outcome; }
        private int Capture(IShellItem item, int result, IShellItem destination)
        {
            try
            {
                if (item != null && !String.Equals(Win32.DisplayName(item, 0x80058000), source, StringComparison.OrdinalIgnoreCase)) return 0;
                if (result == unchecked((int)0x80270000) || result == unchecked((int)0x800704C7)) outcome.Cancelled = true;
                else if (result < 0) outcome.Error = result;
                else if (destination != null) outcome.Paths[source] = Win32.DisplayName(destination, 0x80058000);
                return 0;
            }
            catch (COMException error) { outcome.Error = error.ErrorCode; return error.ErrorCode; }
        }
        public int StartOperations() { return 0; }
        public int FinishOperations(int result) { return 0; }
        public int PreRenameItem(uint f, IShellItem i, string n) { return 0; }
        public int PostRenameItem(uint f, IShellItem i, string n, int r, IShellItem d) { return Capture(i, r, d); }
        public int PreMoveItem(uint f, IShellItem i, IShellItem d, string n) { return 0; }
        public int PostMoveItem(uint f, IShellItem i, IShellItem d, string n, int r, IShellItem result) { return Capture(i, r, result); }
        public int PreCopyItem(uint f, IShellItem i, IShellItem d, string n) { return 0; }
        public int PostCopyItem(uint f, IShellItem i, IShellItem d, string n, int r, IShellItem result) { return Capture(i, r, result); }
        public int PreDeleteItem(uint f, IShellItem i) { return 0; }
        public int PostDeleteItem(uint f, IShellItem i, int r, IShellItem d) { return Capture(i, r, d); }
        public int PreNewItem(uint f, IShellItem d, string n) { return 0; }
        public int PostNewItem(uint f, IShellItem d, string n, string t, uint a, int r, IShellItem created) { return Capture(null, r, created); }
        public int UpdateProgress(uint t, uint c) { return 0; }
        public int ResetTimer() { return 0; }
        public int PauseTimer() { return 0; }
        public int ResumeTimer() { return 0; }
    }

    internal enum TransferMode { Default, Copy, Move }
    internal sealed class FileOperations : IDisposable
    {
        private readonly StaWorker worker = new StaWorker("macui.desktop.files");
        [DllImport("kernel32", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool GetVolumePathName(string path, StringBuilder volume, uint length);
        internal static bool CopyByDefault(string source, string destination)
        {
            var first = new StringBuilder(1024); var second = new StringBuilder(1024);
            if (!GetVolumePathName(source, first, (uint)first.Capacity) || !GetVolumePathName(destination, second, (uint)second.Capacity))
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            return !String.Equals(first.ToString(), second.ToString(), StringComparison.OrdinalIgnoreCase);
        }
        private static IFileOperation Create(IntPtr owner, bool permanent, bool duplicate)
        {
            var operation = (IFileOperation)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("3AD05575-8857-4850-9277-11B85BDB8E09")));
            try
            {
                operation.SetOwnerWindow(owner);
                operation.SetOperationFlags(0x4000u | (permanent ? 0u : 0x40u | 0x20000000u | 0x80000u) | (duplicate ? 0x8u | 0x200000u : 0u));
                return operation;
            }
            catch { Win32.Release(operation); throw; }
        }
        private static void Perform(IFileOperation operation, TransferResult outcome)
        {
            int result = operation.PerformOperations();
            if (result == unchecked((int)0x80270000) || result == unchecked((int)0x800704C7)) outcome.Cancelled = true;
            else if (result < 0) outcome.Error = result;
            bool cancelled; operation.GetAnyOperationsAborted(out cancelled);
            outcome.Cancelled |= cancelled;
        }
        internal Task<TransferResult> Transfer(IntPtr owner, string[] paths, string destination, TransferMode mode)
        {
            return worker.Run(delegate {
                if (!Directory.Exists(destination)) throw new DirectoryNotFoundException(destination);
                var outcome = new TransferResult();
                var resources = new List<object>();
                var sinks = new List<OperationSink>();
                bool duplicate = paths.Any(p => String.Equals(Path.GetDirectoryName(p), destination, StringComparison.OrdinalIgnoreCase));
                var operation = Create(owner, false, duplicate);
                try
                {
                    var folder = Win32.Item(destination); resources.Add(folder);
                    foreach (string path in paths.Distinct(StringComparer.OrdinalIgnoreCase))
                    {
                        bool copy = mode == TransferMode.Copy || (mode == TransferMode.Default && CopyByDefault(path, destination));
                        if (!copy && String.Equals(Path.GetDirectoryName(path), destination, StringComparison.OrdinalIgnoreCase)) { outcome.Paths[path] = path; continue; }
                        var item = Win32.Item(path); resources.Add(item);
                        var sink = new OperationSink(path, outcome); sinks.Add(sink);
                        if (copy) operation.CopyItem(item, folder, null, sink);
                        else operation.MoveItem(item, folder, null, sink);
                    }
                    Perform(operation, outcome);
                    GC.KeepAlive(sinks);
                    return outcome;
                }
                finally { foreach (var resource in resources) Win32.Release(resource); Win32.Release(operation); }
            });
        }
        internal Task<TransferResult> CreateFromTemplate(IntPtr owner, string destination, string name, ShellTemplate template)
        {
            return worker.Run(delegate {
                string temporary = null;
                var outcome = new TransferResult();
                var operation = Create(owner, false, false); IShellItem source = null, folder = null;
                try
                {
                    string templatePath = template.FileName;
                    if (templatePath == null)
                    {
                        temporary = Path.Combine(Path.GetTempPath(), "macui-template-" + Guid.NewGuid().ToString("N") + template.Extension);
                        using (var file = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write)) file.Write(template.Data, 0, template.Data.Length);
                        templatePath = temporary;
                    }
                    source = Win32.Item(templatePath); folder = Win32.Item(destination);
                    var sink = new OperationSink(templatePath, outcome);
                    operation.CopyItem(source, folder, name, sink); Perform(operation, outcome); GC.KeepAlive(sink); return outcome;
                }
                finally { Win32.Release(source); Win32.Release(folder); Win32.Release(operation); if (temporary != null) File.Delete(temporary); }
            });
        }
        internal Task<TransferResult> Change(IntPtr owner, string[] paths, string action, string name = null, bool permanent = false)
        {
            return worker.Run(delegate {
                var outcome = new TransferResult();
                var operation = Create(owner, permanent, false);
                var resources = new List<object>(); var sinks = new List<OperationSink>();
                try
                {
                    foreach (string path in paths)
                    {
                        var item = Win32.Item(path); resources.Add(item);
                        var sink = new OperationSink(path, outcome); sinks.Add(sink);
                        if (action == "rename") operation.RenameItem(item, name, sink);
                        else if (action == "folder") operation.NewItem(item, 0x10, name, null, sink);
                        else if (action == "file") operation.NewItem(item, 0x80, name, null, sink);
                        else if (action == "delete") operation.DeleteItem(item, sink);
                        else throw new ArgumentException("Unknown file action", "action");
                    }
                    Perform(operation, outcome); GC.KeepAlive(sinks); return outcome;
                }
                finally { foreach (var resource in resources) Win32.Release(resource); Win32.Release(operation); }
            });
        }
        public void Dispose() { worker.Dispose(); }
    }
}
