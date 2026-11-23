using Microsoft.Win32;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;

namespace MacUI.Desktop.Native
{
    internal sealed class NativeDrag
    {
        internal readonly string Token = Guid.NewGuid().ToString("N");
        internal DesktopEntry[] Entries;
        internal string Scope;
        internal Point Offset;
    }
    internal sealed partial class DesktopSession : IDisposable
    {
        internal const string DragFormat = "MACUI.Desktop.Selection";
        internal readonly DesktopModel Model;
        internal readonly ShellCatalog Catalog;
        internal readonly FileOperations Files = new FileOperations();
        internal readonly HashSet<string> Selected = new HashSet<string>();
        internal NativeDrag Dragging;
        internal DesktopWindow MovingBox;
        internal bool Contrast, Restacking, Stopping, ClosingWindows;
        internal Dictionary<string, object> Toolbar = new Dictionary<string, object>();
        internal readonly bool Preview;
        internal event Action<string, object> Send;
        private readonly Dispatcher dispatcher;
        private readonly List<DesktopWindow> windows = new List<DesktopWindow>();
        private readonly List<FileSystemWatcher> watchers = new List<FileSystemWatcher>();
        private readonly DispatcherTimer refreshTimer, guardTimer, displayTimer;
        private readonly DesktopRecovery recovery;
        private List<DesktopMonitor> monitors;
        private bool refreshRunning, refreshAgain, renderPending, operationRunning, stopRequested;
        private string anchor;
        internal DesktopSession(DesktopModel model, ShellCatalog catalog, bool preview)
        {
            Model = model; Catalog = catalog; Preview = preview; dispatcher = Dispatcher.CurrentDispatcher;
            refreshTimer = new DispatcherTimer(TimeSpan.FromMilliseconds(180), DispatcherPriority.Background, delegate { refreshTimer.Stop(); Refresh(); }, dispatcher); refreshTimer.Stop();
            displayTimer = new DispatcherTimer(TimeSpan.FromMilliseconds(150), DispatcherPriority.Background, delegate { displayTimer.Stop(); UpdateDisplays(); }, dispatcher); displayTimer.Stop();
            guardTimer = new DispatcherTimer(TimeSpan.FromSeconds(1.5), DispatcherPriority.Background, delegate { if (!Stopping) { if (recovery != null) recovery.HideExplorerIcons(); Restack(); } }, dispatcher); guardTimer.Stop();
            if (!preview) recovery = new DesktopRecovery(model.Store.DirectoryPath);
            Model.Changed += ModelChanged;
            SystemEvents.DisplaySettingsChanged += DisplaysChanged;
        }
        internal async Task Start()
        {
            monitors = DesktopMonitor.Read();
            foreach (var monitor in monitors) DesktopLayout.ReserveToolbar(monitor, Toolbar);
            Model.Catalog = await Catalog.Read(Model.MappedFolders);
            if (recovery != null) recovery.Acquire();
            Render(); WatchFolders(); guardTimer.Start();
            if (Send != null) Send("ready", new { pid = System.Diagnostics.Process.GetCurrentProcess().Id, windows = windows.Select(w => w.Handle.ToInt64()).ToArray(), controls = Model.Controls() });
        }
        private void ModelChanged()
        {
            if (Stopping) return;
            if (Dragging != null || MovingBox != null) renderPending = true; else Render();
            if (Send != null) Send("changed", Model.Controls());
        }
        internal void ScheduleDisplays() { displayTimer.Stop(); displayTimer.Start(); }
        private void DisplaysChanged(object sender, EventArgs e) { dispatcher.BeginInvoke(new Action(ScheduleDisplays)); }
        private void UpdateDisplays()
        {
            if (Dragging != null || MovingBox != null) { ScheduleDisplays(); return; }
            monitors = DesktopMonitor.Read();
            foreach (var monitor in monitors) DesktopLayout.ReserveToolbar(monitor, Toolbar);
            Render();
        }
        private void Render()
        {
            if (Stopping || monitors == null) return;
            var groups = Model.Groups(); var wanted = new HashSet<DesktopWindow>();
            foreach (var monitor in monitors)
            {
                var loose = groups.First(g => g.Id == (Model.Categorized ? "desktop" : "all"));
                var positions = Json.Map(Json.Map(Model.Data, "looseIconPositions"), loose.Id);
                var displayGroup = new DesktopGroup { Id = loose.Id, Kind = "manual", Items = loose.Items.Where(e => {
                    object saved; string device = positions.TryGetValue(e.Key, out saved) ? Json.Text((Dictionary<string, object>)saved, "monitor") : "";
                    return device == monitor.Id || (!monitors.Any(m => m.Id == device) && monitor.Primary);
                }).ToList() };
                var window = windows.FirstOrDefault(w => w.Loose && w.Monitor.Id == monitor.Id);
                if (window == null) { window = new DesktopWindow(this, monitor, displayGroup, true); windows.Add(window); window.Show(); }
                window.Monitor = monitor; window.Place(new Point(), new Size(monitor.Width, monitor.Height)); window.Update(displayGroup); wanted.Add(window);
            }
            int index = 0;
            foreach (var group in groups.Where(g => g.Id != "desktop" && g.Id != "all"))
            {
                string device = Json.Text(Json.Map(Model.Organization, "monitors"), group.Id);
                var monitor = monitors.FirstOrDefault(m => m.Id == device) ?? monitors[0];
                var window = windows.FirstOrDefault(w => !w.Loose && w.Group.Id == group.Id);
                if (window == null) { window = new DesktopWindow(this, monitor, group, false); windows.Add(window); window.Show(); }
                window.Monitor = monitor;
                var size = Model.Size(group.Id, monitor, new Size(290, 330));
                size.Width = Math.Min(monitor.Width, size.Width); size.Height = Math.Min(monitor.Height, size.Height);
                if (Json.Array(Model.Data, "collapsed").Contains(group.Id)) size.Height = 48;
                int rows = Math.Max(1, (int)((monitor.Height - 65) / 344));
                var location = Model.Position(group.Id, monitor, new Point(monitor.Width - 306 - (index / rows) * 304, 64 + (index % rows) * 344));
                location = DesktopLayout.ClampBox(location, size, monitor);
                window.Place(location, size); window.Update(group); wanted.Add(window); index++;
            }
            ClosingWindows = true;
            try { foreach (var window in windows.Where(w => !wanted.Contains(w)).ToArray()) { window.Close(); windows.Remove(window); } }
            finally { ClosingWindows = false; }
            Restack();
        }
        internal void Restack()
        {
            if (Stopping) return;
            var owned = new HashSet<IntPtr>(windows.Select(w => w.Handle));
            owned.UnionWith(windows.Select(w => w.ToolbarHandle).Where(h => h != IntPtr.Zero));
            IntPtr lastApp = IntPtr.Zero; uint shellPid; Win32.GetWindowThreadProcessId(Win32.GetShellWindow(), out shellPid);
            for (IntPtr hwnd = Win32.GetTopWindow(IntPtr.Zero); hwnd != IntPtr.Zero; hwnd = Win32.GetWindow(hwnd, 2))
            {
                if (owned.Contains(hwnd) || NativeGlass.IsHost(hwnd) || !Win32.IsWindowVisible(hwnd) || Win32.IsIconic(hwnd)) continue;
                uint pid; Win32.GetWindowThreadProcessId(hwnd, out pid);
                string name = Win32.Class(hwnd);
                if (pid == shellPid && (name == "Progman" || name == "WorkerW")) break;
                int cloaked; if (Win32.DwmGetWindowAttribute(hwnd, 14, out cloaked, 4) >= 0 && cloaked != 0) continue;
                if ((Win32.GetWindowLongPtr(hwnd, -20).ToInt64() & 0x8) == 0) lastApp = hwnd;
            }
            Restacking = true;
            try
            {
                foreach (var window in windows.OrderByDescending(w => w.Loose))
                    Win32.SetWindowPos(window.Handle, lastApp, 0, 0, 0, 0, 0x1 | 0x2 | 0x10 | 0x200);
                foreach (var toolbar in windows.Select(w => w.ToolbarHandle).Where(h => h != IntPtr.Zero))
                    Win32.SetWindowPos(toolbar, lastApp, 0, 0, 0, 0, 0x1 | 0x2 | 0x10 | 0x200);
            }
            finally { Restacking = false; }
        }
        private void WatchFolders()
        {
            foreach (var watcher in watchers) watcher.Dispose(); watchers.Clear();
            foreach (var directory in new[] { Catalog.DesktopPath, Catalog.PublicPath }.Concat(Model.MappedFolders).Where(p => p != null).Distinct(StringComparer.OrdinalIgnoreCase))
            {
                if (!Directory.Exists(directory)) continue;
                var watcher = new FileSystemWatcher(directory) { NotifyFilter = NotifyFilters.FileName | NotifyFilters.DirectoryName | NotifyFilters.LastWrite | NotifyFilters.Size };
                FileSystemEventHandler changed = delegate { ScheduleRefresh(); };
                watcher.Changed += changed; watcher.Created += changed; watcher.Deleted += changed; watcher.Renamed += delegate { ScheduleRefresh(); };
                watcher.Error += delegate(object sender, ErrorEventArgs e) { dispatcher.BeginInvoke(new Action(delegate { Error(e.GetException()); ScheduleRefresh(); })); };
                watcher.EnableRaisingEvents = true; watchers.Add(watcher);
            }
        }
        private void ScheduleRefresh()
        { dispatcher.BeginInvoke(new Action(delegate { if (Stopping) return; refreshTimer.Stop(); refreshTimer.Start(); })); }
        internal async void Refresh()
        {
            if (Stopping) return;
            if (refreshRunning || Dragging != null) { refreshAgain = true; return; }
            refreshRunning = true;
            try { Model.ReplaceCatalog(await Catalog.Read(Model.MappedFolders)); WatchFolders(); }
            catch (Exception error) { Error(error); }
            finally { refreshRunning = false; if (refreshAgain && Dragging == null) { refreshAgain = false; ScheduleRefresh(); } }
        }
        internal async void LoadIcon(IconTile tile)
        {
            var entry = tile.Entry;
            int revision = tile.IconRevision;
            try { var image = await Catalog.Icon(entry, (int)Math.Ceiling(Model.IconSize * monitors.Max(m => m.Scale))); if (!Stopping && tile.IconRevision == revision) tile.SetIcon(image); }
            catch (COMException error) { Log("icon", error); }
            catch (FileNotFoundException error) { Log("icon", error); }
        }
        internal void Select(DesktopWindow window, IconTile tile, bool control, bool shift)
        {
            if (shift && anchor != null)
            {
                int first = window.Tiles.FindIndex(t => t.Entry.Key == anchor), last = window.Tiles.IndexOf(tile);
                if (!control) Selected.Clear();
                if (first >= 0) foreach (var selected in window.Tiles.Skip(Math.Min(first, last)).Take(Math.Abs(last - first) + 1)) Selected.Add(selected.Entry.Key);
                else Selected.Add(tile.Entry.Key);
            }
            else if (control) { if (!Selected.Remove(tile.Entry.Key)) Selected.Add(tile.Entry.Key); anchor = tile.Entry.Key; }
            else { if (!Selected.Contains(tile.Entry.Key)) { Selected.Clear(); Selected.Add(tile.Entry.Key); } anchor = tile.Entry.Key; }
            RefreshSelection();
        }
        internal void RefreshSelection() { foreach (var window in windows) window.RefreshSelection(); }
        internal DesktopMonitor MonitorAt(Point cursor) { return DesktopLayout.MonitorAt(cursor, monitors); }
        internal void FinishBoxMove()
        {
            MovingBox = null;
            if (stopRequested) RequestStop();
            else if (renderPending) { renderPending = false; Render(); }
            Restack();
        }
        internal Point PlaceBox(DesktopWindow window, Point point, Size size, bool moving = false)
        {
            var occupied = windows.Where(other => !other.Loose && other != window && other.Monitor.Id == window.Monitor.Id).Select(other => {
                Win32.RECT rect; Win32.GetWindowRect(other.Handle, out rect);
                return new Rect((rect.Left - window.Monitor.Work.Left) / window.Monitor.Scale, (rect.Top - window.Monitor.Work.Top) / window.Monitor.Scale,
                    (rect.Right - rect.Left) / window.Monitor.Scale, (rect.Bottom - rect.Top) / window.Monitor.Scale);
            });
            bool snapEdges = Json.Flag(Model.Data, "snapBoxes", true) && (Keyboard.Modifiers & ModifierKeys.Shift) == 0;
            if (moving) return snapEdges ? DesktopLayout.AlignBox(point, size, window.Monitor, occupied) : DesktopLayout.ClampBox(point, size, window.Monitor);
            return DesktopLayout.PlaceBox(point, size, window.Monitor, occupied, Json.Flag(Model.Data, "snapToGrid", true), snapEdges);
        }
        private DesktopEntry[] Selection()
        { return Model.Catalog.Desktop.Concat(Model.Catalog.Folders.Values.SelectMany(items => items)).Where(e => Selected.Contains(e.Key)).GroupBy(e => e.Key).Select(g => g.First()).ToArray(); }
        internal void StartDrag(DesktopWindow window, DesktopEntry entry, Point offset)
        {
            if (operationRunning || Dragging != null) return;
            if (!Selected.Contains(entry.Key)) { Selected.Clear(); Selected.Add(entry.Key); }
            var entries = Selection(); if (entries.Length == 0) return;
            Dragging = new NativeDrag { Entries = entries, Scope = window.Group.Id, Offset = offset };
            var data = new DataObject(); data.SetData(DragFormat, Dragging.Token, false);
            if (entries.All(item => !item.IsNamespace)) data.SetData(DataFormats.FileDrop, entries.Select(item => item.Path).ToArray(), true);
            try
            {
                ShellDragImage.Start(window.Handle, data, window.Tiles.First(tile => tile.Entry.Key == entry.Key), offset, window.Monitor.Scale);
            }
            catch (COMException error) { Error(error); }
            finally
            {
                Dragging = null; Mouse.Capture(null);
                if (stopRequested) RequestStop();
                else
                {
                    if (renderPending) { renderPending = false; Render(); }
                    if (refreshAgain) { refreshAgain = false; ScheduleRefresh(); }
                }
            }
        }
        private bool OwnDrag(IDataObject data)
        { return Dragging != null && data.GetDataPresent(DragFormat, false) && Equals(data.GetData(DragFormat, false), Dragging.Token); }
        private bool VisualDrop(DesktopWindow window, DesktopEntry target, IDataObject data, DragDropKeyStates keys)
        {
            if (!OwnDrag(data) || (keys & (DragDropKeyStates.ControlKey | DragDropKeyStates.ShiftKey)) != 0 || (target != null && target.IsDirectory)) return false;
            if (Dragging.Entries.All(e => e.IsDesktop) && String.IsNullOrEmpty(window.Group.Folder)) return true;
            return Dragging.Scope == window.Group.Id && Dragging.Entries.All(e => window.Group.Items.Any(i => i.Key == e.Key));
        }
        internal DragDropEffects DropEffect(DesktopWindow window, DesktopEntry target, DragEventArgs args)
        {
            if (operationRunning || window.Group.Query || (args.KeyStates & DragDropKeyStates.AltKey) != 0) return DragDropEffects.None;
            if (VisualDrop(window, target, args.Data, args.KeyStates)) return args.AllowedEffects & DragDropEffects.Move;
            if (!args.Data.GetDataPresent(DataFormats.FileDrop)) return DragDropEffects.None;
            var paths = args.Data.GetData(DataFormats.FileDrop) as string[]; if (paths == null || paths.Length == 0) return DragDropEffects.None;
            string destination = Destination(window, target);
            if (paths.Any(p => String.Equals(p, destination, StringComparison.OrdinalIgnoreCase) || destination.StartsWith(p.TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase))) return DragDropEffects.None;
            return TransferEffect(paths, destination, args.KeyStates, args.AllowedEffects);
        }
        internal static DragDropEffects TransferEffect(string[] paths, string destination, DragDropKeyStates keys, DragDropEffects allowed)
        {
            bool control = (keys & DragDropKeyStates.ControlKey) != 0, shift = (keys & DragDropKeyStates.ShiftKey) != 0;
            if ((control && shift) || (keys & DragDropKeyStates.AltKey) != 0) return DragDropEffects.None;
            var effect = control ? DragDropEffects.Copy : shift ? DragDropEffects.Move : paths.Any(path => FileOperations.CopyByDefault(path, destination)) ? DragDropEffects.Copy : DragDropEffects.Move;
            if ((allowed & effect) != 0) return effect;
            if (control || shift) return DragDropEffects.None;
            return (allowed & DragDropEffects.Copy) != 0 ? DragDropEffects.Copy : allowed & DragDropEffects.Move;
        }
        private string Destination(DesktopWindow window, DesktopEntry target)
        { return target != null && target.IsDirectory ? target.Path : String.IsNullOrEmpty(window.Group.Folder) ? Catalog.DesktopPath : window.Group.Folder; }
        internal void Drop(DesktopWindow window, DesktopEntry target, IDataObject data, DragDropKeyStates keys, DragDropEffects effect, Point point)
        {
            if (VisualDrop(window, target, data, keys))
            {
                string[] paths = Dragging.Entries.Select(e => e.Path).ToArray();
                if (window.Loose) Model.MoveLoose(window.Group.Id, paths, point - (Vector)Dragging.Offset, window.Monitor.Id);
                else Model.Reorder(window.Group.Id, window.Group.Items, paths, target == null ? null : target.Path);
                return;
            }
            var files = (string[])data.GetData(DataFormats.FileDrop);
            var mode = effect == DragDropEffects.Copy ? TransferMode.Copy : TransferMode.Move;
            Transfer(window, files, Destination(window, target), mode, target == null || !target.IsDirectory ? window.Group.Id : null, point);
        }
        private async void Transfer(DesktopWindow window, string[] paths, string destination, TransferMode mode, string scope, Point point, uint clipboardSequence = 0)
        {
            if (operationRunning) return;
            operationRunning = true;
            try
            {
                var result = await Files.Transfer(window.Handle, paths, destination, mode);
                foreach (var path in result.Paths) if (mode != TransferMode.Copy && !File.Exists(path.Key) && !Directory.Exists(path.Key)) Model.RenamePath(path.Key, path.Value);
                if (scope != null && destination.Equals(Catalog.DesktopPath, StringComparison.OrdinalIgnoreCase))
                { if (window.Loose) Model.MoveLoose(scope, result.Paths.Values.ToArray(), point, window.Monitor.Id); else Model.Assign(result.Paths.Values.ToArray(), scope); }
                else Model.Store.Save();
                result.ThrowIfFailed();
                if (clipboardSequence != 0 && !result.Cancelled && result.Paths.Count == paths.Distinct(StringComparer.OrdinalIgnoreCase).Count() && clipboardSequence == Win32.GetClipboardSequenceNumber()) Clipboard.Clear();
            }
            catch (Exception error) { Error(error); }
            finally { OperationFinished(); }
        }
        internal void Open(DesktopWindow window, DesktopEntry entry)
        { try { ShellMenu.Open(window.Handle, entry.Path); Model.Opened(entry); } catch (Exception error) { Error(error); } }
        internal void Copy(bool cut)
        {
            var entries = Selection(); if (entries.Length == 0) return;
            if (entries.Any(e => e.IsNamespace)) { Error(new InvalidOperationException(Model.T("boxes.namespace_readonly"))); return; }
            try
            {
                var data = new DataObject(); data.SetData(DataFormats.FileDrop, entries.Select(e => e.Path).ToArray(), true);
                data.SetData("Preferred DropEffect", new MemoryStream(BitConverter.GetBytes(cut ? 2 : 1)));
                Clipboard.SetDataObject(data, true);
            }
            catch (ExternalException error) { Error(error); }
        }
        internal void Paste(DesktopWindow window)
        {
            if (operationRunning || window.Group.Query) return;
            try
            {
                uint sequence = Win32.GetClipboardSequenceNumber();
                var data = Clipboard.GetDataObject(); if (data == null || !data.GetDataPresent(DataFormats.FileDrop)) return;
                var paths = (string[])data.GetData(DataFormats.FileDrop); var stream = data.GetData("Preferred DropEffect") as MemoryStream;
                byte[] effect = stream == null ? null : stream.ToArray(); bool cut = effect != null && effect.Length >= 4 && BitConverter.ToInt32(effect, 0) == 2;
                Transfer(window, paths, Destination(window, null), cut ? TransferMode.Move : TransferMode.Copy, window.Group.Id, new Point(16, 65), cut ? sequence : 0);
            }
            catch (ExternalException error) { Error(error); }
        }
        internal async void Delete(DesktopWindow window, bool permanent)
        {
            var entries = Selection(); if (operationRunning || entries.Length == 0) return;
            if (entries.Any(e => e.IsNamespace)) { Error(new InvalidOperationException(Model.T("boxes.namespace_readonly"))); return; }
            operationRunning = true;
            try { var result = await Files.Change(window.Handle, entries.Select(e => e.Path).ToArray(), "delete", null, permanent); result.ThrowIfFailed(); }
            catch (Exception error) { Error(error); }
            finally { OperationFinished(); }
        }
        internal async void Rename(DesktopWindow window)
        {
            var entries = Selection(); if (operationRunning || entries.Length != 1 || entries[0].IsNamespace) return;
            string name = Dialogs.Text(window, Model.T("rename_label"), entries[0].Name, Model.Text); if (name == null || name == entries[0].Name) return;
            if ((entries[0].Extension == "lnk" || entries[0].Extension == "url") && !name.EndsWith("." + entries[0].Extension, StringComparison.OrdinalIgnoreCase)) name += "." + entries[0].Extension;
            operationRunning = true;
            try { var result = await Files.Change(window.Handle, new[] { entries[0].Path }, "rename", name); foreach (var path in result.Paths) Model.RenamePath(path.Key, path.Value); Model.Save(); result.ThrowIfFailed(); }
            catch (Exception error) { Error(error); }
            finally { OperationFinished(); }
        }
        internal async void NewFile(DesktopWindow window, bool folder, ShellTemplate template = null)
        {
            string suggested = template == null ? Model.T(folder ? "new_folder_name" : "new_text_name") + (folder ? "" : ".txt") : template.Label + template.Extension;
            string name = Dialogs.Text(window, Model.T(folder ? "menu.new_folder" : "menu.new_file"), suggested, Model.Text);
            if (name == null || operationRunning) return;
            operationRunning = true;
            try
            {
                var result = template == null ? await Files.Change(window.Handle, new[] { Destination(window, null) }, folder ? "folder" : "file", name)
                    : await Files.CreateFromTemplate(window.Handle, Destination(window, null), name, template);
                if (String.IsNullOrEmpty(window.Group.Folder) && window.Group.Id != "all") Model.Assign(result.Paths.Values.ToArray(), window.Group.Id);
                result.ThrowIfFailed();
            }
            catch (Exception error) { Error(error); }
            finally { OperationFinished(); }
        }
        internal void Organize()
        {
            Model.Data["arrangeMode"] = "category"; Model.Organization["strategy"] = "deskgo"; Model.Organization["dismissedCategories"] = new string[0];
            Model.Save();
        }
        internal void Error(Exception error)
        { Log("operation", error); if (!Stopping) MessageBox.Show(error.Message, Model.T("operation_failed"), MessageBoxButton.OK, MessageBoxImage.Error); }
        private void Log(string action, Exception error)
        { if (Send != null) Send("error", new { action = action, operation = error.Data["nativeOperation"], hresult = "0x" + error.HResult.ToString("X8"), system = Environment.OSVersion.VersionString, error = error.ToString() }); }
        private void OperationFinished()
        { operationRunning = false; if (stopRequested) RequestStop(); else Refresh(); }
        internal void RequestStop()
        {
            if (Stopping) return;
            stopRequested = true;
            if (operationRunning || Dragging != null || MovingBox != null) return;
            Stopping = true;
            Application.Current.Shutdown();
        }
        public void Dispose()
        {
            Stopping = true; SystemEvents.DisplaySettingsChanged -= DisplaysChanged; Model.Changed -= ModelChanged;
            guardTimer.Stop(); refreshTimer.Stop(); displayTimer.Stop(); foreach (var watcher in watchers) watcher.Dispose();
            foreach (var window in windows.ToArray()) window.Close(); windows.Clear();
            if (recovery != null) recovery.Dispose(); Catalog.Dispose(); Files.Dispose();
        }
    }
}
