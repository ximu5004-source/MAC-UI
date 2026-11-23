using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using MacUI.Desktop.Native;

internal static class NativeTests
{
    [DllImport("user32")] private static extern IntPtr CreatePopupMenu();
    [DllImport("user32")] private static extern bool DestroyMenu(IntPtr menu);
    [DllImport("user32")] private static extern int GetMenuItemCount(IntPtr menu);
    [DllImport("user32", CharSet = CharSet.Unicode)] private static extern int GetMenuString(IntPtr menu, uint item, System.Text.StringBuilder text, int count, uint flags);
    private static int assertions;
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); assertions++; }
    private static void CheckShellMenu(string[] paths)
    {
        Console.WriteLine("Checking Shell selection: " + paths.Length);
        System.Windows.Forms.Application.OleRequired();
        using (var owner = new System.Windows.Interop.HwndSource(new System.Windows.Interop.HwndSourceParameters("macui.desktop.tests") { WindowStyle = unchecked((int)0x80000000), Width = 1, Height = 1 }))
        using (var selection = ShellSelection.Create(owner.Handle, paths))
            Check(selection.Menu != null, "Windows context menu supports the selected Shell items");
    }
    [STAThread] private static int Main(string[] arguments)
    {
        if (arguments.Length == 2 && arguments[0] == "--visual") return VisualTests.Run(arguments[1]);
        if (arguments.Length == 1 && arguments[0] == "--glass")
        {
            try
            {
                var window = new System.Windows.Window { Width = 320, Height = 240, WindowStyle = System.Windows.WindowStyle.None, Background = System.Windows.Media.Brushes.Transparent };
                window.Show();
                try { using (var glass = new NativeGlass(window)) { glass.SetBlur(0); glass.SetBlur(100); window.UpdateLayout(); } }
                finally { window.Close(); }
                Console.WriteLine("PASS: native Composition backdrop and Gaussian blur initialized"); return 0;
            }
            catch (Exception error) { Console.Error.WriteLine(error); return 1; }
        }
        if (arguments.Length == 1 && arguments[0] == "--inspect-explorer") return RecoveryTests.Inspect();
        if (arguments.Length == 2 && arguments[0] == "--recovery")
        {
            try { return RecoveryTests.Run(arguments[1]); }
            catch (Exception error) { Console.Error.WriteLine(error); return 1; }
        }
        string directory = Path.Combine(Path.GetTempPath(), "macui-native-tests-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        try
        {
            var config = Path.Combine(directory, "config"); Directory.CreateDirectory(config);
            string original = "{\"schemaVersion\":3,\"arrangeMode\":\"category\",\"customFutureSetting\":{\"value\":7},\"organization\":{\"strategy\":\"deskgo\",\"boxes\":[],\"assignments\":{}}}";
            File.WriteAllText(Path.Combine(config, "desktop-shell.json"), original);
            var model = new DesktopModel(new LayoutStore(config), new Localizer(arguments[0], "zh-CN"));
            Check(File.ReadAllText(Path.Combine(config, "desktop-shell.before-native-csharp.json")) == original, "Migration preserves the original configuration");
            var source = Path.Combine(directory, "source"); var destination = Path.Combine(directory, "destination"); Directory.CreateDirectory(source); Directory.CreateDirectory(destination);
            var first = Path.Combine(source, "中文 $ 文件.txt"); var second = Path.Combine(source, "second.txt"); File.WriteAllText(first, "first"); File.WriteAllText(second, "second");
            var firstEntry = DesktopEntry.File(first, true); var secondEntry = DesktopEntry.File(second, true);
            using (var owner = new System.Windows.Interop.HwndSource(new System.Windows.Interop.HwndSourceParameters("macui.desktop.tests") { Width = 1, Height = 1 }))
            using (var background = ShellSelection.Background(owner.Handle))
            {
                var nativeMenu = CreatePopupMenu();
                try
                {
                    ShellMenu.Populate(background.Menu, nativeMenu, true, false);
                    bool refresh = false;
                    var labels = new List<string>();
                    for (int index = 0; index < GetMenuItemCount(nativeMenu); index++)
                    {
                        var label = new System.Text.StringBuilder(256); GetMenuString(nativeMenu, (uint)index, label, 256, 0x400);
                        string caption = label.ToString().Replace("&", "");
                        labels.Add(caption);
                        refresh |= caption.StartsWith("刷新", StringComparison.Ordinal) || caption.StartsWith("Refresh", StringComparison.OrdinalIgnoreCase);
                    }
                    Check(refresh, "Explorer desktop background menu includes the native Refresh command");
                    Check(labels.Any(s => s.StartsWith("查看") || s.StartsWith("View")) && labels.Any(s => s.StartsWith("排序方式") || s.StartsWith("Sort by")), "Native View and Sort submenus remain available");
                }
                finally { DestroyMenu(nativeMenu); }
            }
            var cell = DesktopLayout.Cell(model);
            var snapped = DesktopLayout.SnapIcon(new System.Windows.Point(123, 176), cell);
            Check((snapped.X - 12) % cell.Width == 0 && (snapped.Y - 58) % cell.Height == 0, "Loose icons snap to the same grid as their layout");
            var reserved = new DesktopMonitor { Scale = 2, Bounds = new Win32.RECT { Top = 100, Bottom = 1300 }, Work = new Win32.RECT { Top = 100, Bottom = 1300 } };
            DesktopLayout.ReserveToolbar(reserved, new Dictionary<string, object> { { "enabled", true }, { "itemSize", 16 }, { "padding", 8 }, { "position", "Top" } });
            Check(reserved.Work.Top == 164, "Desktop toolbar respects scaled MAC UI top bar bounds");
            var fullScreen = new DesktopMonitor { Scale = 1, Work = new Win32.RECT { Right = 1920, Bottom = 1080 } };
            var below = DesktopLayout.PlaceBox(new System.Windows.Point(700, 760), new System.Windows.Size(280, 300), fullScreen, new[] { new System.Windows.Rect(700, 100, 280, 600) }, true);
            Check(below.Y >= 740 && below.Y + 300 <= fullScreen.Height, "Available space below a tall box remains usable");
            var adjacent = DesktopLayout.PlaceBox(new System.Windows.Point(700, 650), new System.Windows.Size(280, 300), fullScreen, new[] { new System.Windows.Rect(700, 100, 280, 600) }, true);
            Check(adjacent.X == 700 && adjacent.Y == 712, "Overlapping drops use the nearest free adjacent position");
            var neighbour = new System.Windows.Rect(701, 100, 280, 400);
            var edge = DesktopLayout.AlignBox(new System.Windows.Point(711, 520), new System.Windows.Size(280, 240), fullScreen, new[] { neighbour });
            Check(edge.X == 701 && edge.Y == 512, "Dragging aligns nearby left edges with a consistent vertical gap");
            var rightEdge = DesktopLayout.AlignBox(new System.Windows.Point(630, 520), new System.Windows.Size(360, 240), fullScreen, new[] { neighbour });
            Check(rightEdge.X == 621 && rightEdge.Y == 512, "Different-width boxes align their right edges");
            var beyondSnap = DesktopLayout.AlignBox(new System.Windows.Point(714, 525), new System.Windows.Size(280, 240), fullScreen, new[] { neighbour });
            Check(beyondSnap.X == 714 && beyondSnap.Y == 525, "Dragging beyond the 12-DIP snap radius preserves continuous movement");
            var overlap = DesktopLayout.AlignBox(new System.Windows.Point(550, 450), new System.Windows.Size(100, 100), fullScreen, new[] { new System.Windows.Rect(500, 400, 200, 200) });
            Check(overlap.X == 550 && overlap.Y == 450, "Manual dragging may place a box over another box");
            var folded = DesktopLayout.PlaceBox(new System.Windows.Point(701, 512), new System.Windows.Size(280, 48), fullScreen, new[] { neighbour }, true);
            Check(folded.X == 701 && folded.Y == 512, "Dropping preserves an exact edge match for a collapsed box");
            var foldedBelow = DesktopLayout.AlignBox(new System.Windows.Point(701, 500), new System.Windows.Size(280, 48), fullScreen, new[] { neighbour });
            Check(foldedBelow.X == 701 && foldedBelow.Y == 512, "Collapsed 48-DIP boxes can be dragged below a neighboring box");
            var free = DesktopLayout.PlaceBox(new System.Windows.Point(721, 525), new System.Windows.Size(280, 240), fullScreen, new[] { neighbour }, false, false);
            Check(free.X == 721 && free.Y == 525, "Turning off snapping preserves freely positioned boxes");
            var lowResolution = new DesktopMonitor { Scale = 1, Bounds = new Win32.RECT { Left = 0, Top = 0, Right = 640, Bottom = 480 }, Work = new Win32.RECT { Left = 0, Top = 80, Right = 640, Bottom = 480 } };
            var clampedLarge = DesktopLayout.ClampBox(new System.Windows.Point(100, 100), new System.Windows.Size(600, 350), lowResolution);
            Check(clampedLarge.X == 40 && clampedLarge.Y == 50, "Large boxes clamp to the low-resolution work-area edges");
            var tooLarge = DesktopLayout.ClampBox(new System.Windows.Point(120, 120), new System.Windows.Size(700, 500), lowResolution);
            Check(tooLarge.X == 0 && tooLarge.Y == 0, "Boxes larger than the work area remain at its zero origin");
            var dpiMonitor = new DesktopMonitor { Scale = 1, Work = new Win32.RECT { Left = -1600, Top = 80, Right = 0, Bottom = 980 }, Bounds = new Win32.RECT { Left = -1600, Top = 0, Right = 0, Bottom = 1000 } };
            foreach (double scale in new[] { 1.0, 1.25, 1.5, 2.0 })
            {
                dpiMonitor.Scale = scale;
                var expected = new System.Windows.Point(123.5, 234.25);
                var offset = new System.Windows.Point(17.5, 29.25);
                var cursor = new System.Windows.Point(dpiMonitor.Work.Left + (expected.X + offset.X) * scale, dpiMonitor.Work.Top + (expected.Y + offset.Y) * scale);
                var drag = DesktopLayout.DragPosition(cursor, offset, dpiMonitor);
                Check(Math.Abs(drag.X - expected.X) < 0.001 && Math.Abs(drag.Y - expected.Y) < 0.001, "Box dragging preserves DIP offsets at " + (scale * 100) + "% DPI");
            }
            var leftDisplay = new DesktopMonitor { Id = "left", Scale = 1.25, Bounds = new Win32.RECT { Left = -1600, Top = 0, Right = 0, Bottom = 900 }, Work = new Win32.RECT { Left = -1600, Top = 60, Right = 0, Bottom = 900 } };
            var rightDisplay = new DesktopMonitor { Id = "right", Scale = 1.5, Bounds = new Win32.RECT { Left = 0, Top = -200, Right = 1600, Bottom = 1000 }, Work = new Win32.RECT { Left = 0, Top = -150, Right = 1600, Bottom = 1000 } };
            Check(DesktopLayout.MonitorAt(new System.Windows.Point(-20, 40), new[] { leftDisplay, rightDisplay }) == leftDisplay, "Negative screen coordinates select the monitor whose bounds contain the cursor");
            Check(DesktopLayout.MonitorAt(new System.Windows.Point(20, 40), new[] { leftDisplay, rightDisplay }) == rightDisplay, "Cross-monitor dragging selects the destination monitor by screen bounds");
            model.Patch(new Dictionary<string, object> { { "horizontalSpacing", 16 }, { "verticalSpacing", 20 }, { "snapBoxes", false } });
            var spacingReload = new DesktopModel(new LayoutStore(config), model.Text);
            Check(DesktopLayout.Cell(spacingReload).Width == model.IconSize + 46 && !Json.Flag(spacingReload.Data, "snapBoxes"), "Unified settings persist icon spacing and edge snapping");
            model.Patch(new Dictionary<string, object> { { "glassTransparency", 94 }, { "glassBlur", 68 } });
            var materialReload = new DesktopModel(new LayoutStore(config), model.Text);
            Check(materialReload.GlassTransparency == 94 && materialReload.GlassBlur == 68, "Native glass settings persist across restarts");
            var tile = new IconTile(firstEntry, 48, false);
            Check(!tile.UpdateEntry(DesktopEntry.File(first, true), 48) && tile.IconRevision == 1, "Unchanged native icons preserve their image revision");
            Check(DesktopSession.TransferEffect(new[] { first }, destination, 0, System.Windows.DragDropEffects.Copy) == System.Windows.DragDropEffects.Copy, "Copy-only drag sources cannot be moved");
            Check(DesktopSession.TransferEffect(new[] { first }, destination, System.Windows.DragDropKeyStates.ShiftKey, System.Windows.DragDropEffects.Copy) == System.Windows.DragDropEffects.None, "Explicit move respects source capabilities");
            model.Catalog.Desktop.AddRange(new[] { firstEntry, secondEntry });
            Check(model.Scope(firstEntry) == "deskgo-files", "Desktop grouping");
            string box = model.AddBox("工作", "manual", "");
            model.Assign(new[] { first, second }, box);
            model.Reorder(box, model.Catalog.Desktop, new[] { second }, first);
            Check(model.Groups().First(g => g.Id == box).Items[0].Path == second, "Manual order is persisted");
            model.Dissolve(model.Groups().First(g => g.Id == box));
            Check(File.ReadAllText(first) == "first" && File.Exists(second), "Dissolving never changes files");
            Check(model.Scope(firstEntry) == "desktop" && model.Scope(secondEntry) == "desktop", "Dissolved files become loose icons");
            model.Automatic(new[] { first });
            Check(model.Scope(firstEntry) == "deskgo-files" && model.Scope(secondEntry) == "desktop", "Automatic classification changes only the selected files");
            model.Patch(new Dictionary<string, object> { { "iconSize", 80 }, { "layoutLocked", true } });
            var monitor = new DesktopMonitor { Id = "test-monitor", Scale = 2, Work = new Win32.RECT { Top = 40 }, Bounds = new Win32.RECT() };
            Json.Map(model.Data, "positions")["images"] = new Dictionary<string, object> { { "x", 88 }, { "y", 160 } };
            var converted = model.Position("images", monitor, new System.Windows.Point());
            Check(converted.X == 56 && converted.Y == 116, "Physical layout coordinates migrate using monitor DPI and work area");
            model.SaveGeometry("images", monitor.Id, new System.Windows.Point(70, 90), new System.Windows.Size(240, 320));
            Check(Json.Number((Dictionary<string, object>)Json.Map(model.Data, "positions")["images"], "x", 0) == 88, "Native geometry preserves the original layout coordinates");
            Check(model.Position("images", monitor, new System.Windows.Point()).X == 70, "Native monitor layout uses DIP coordinates");
            var reloaded = new LayoutStore(config);
            Check(Json.Number((Dictionary<string, object>)reloaded.Data["customFutureSetting"], "value", 0) == 7, "Unknown configuration fields survive saves");
            Check(Json.Number(reloaded.Data, "iconSize", 0) == 80, "Settings round trip");
            bool invalid = false;
            try { model.Patch(new Dictionary<string, object> { { "iconSize", 999 } }); } catch (ArgumentException) { invalid = true; }
            Check(invalid && model.IconSize == 80, "Invalid settings do not mutate configuration");
            model.Store.Snapshot(); model.Patch(new Dictionary<string, object> { { "iconSize", 48 } }); model.Store.Restore(true);
            Check(model.IconSize == 80, "Layout snapshot restores settings");
            using (var operations = new FileOperations())
            {
                var longFolder = Path.Combine(source, new string('a', 100), new string('b', 100));
                Directory.CreateDirectory(longFolder);
                var longFile = Path.Combine(longFolder, "长路径文件.txt");
                File.WriteAllText(longFile, "Long path content");
                Check(longFile.Length > 260 && ShellCatalog.ReadFolder(longFolder, false).Single().Path == longFile, "Native enumeration supports long file paths");
                var longCopy = operations.Transfer(IntPtr.Zero, new[] { longFile }, destination, TransferMode.Copy).GetAwaiter().GetResult(); longCopy.ThrowIfFailed();
                Check(File.ReadAllText(longCopy.Paths[longFile]) == "Long path content", "Windows copy supports long source paths");
                var copied = operations.Transfer(IntPtr.Zero, new[] { first }, destination, TransferMode.Copy).GetAwaiter().GetResult(); copied.ThrowIfFailed();
                Check(!copied.Cancelled && copied.Paths.Count == 1, "Windows copy reports an actual destination");
                Check(File.ReadAllText(copied.Paths[first]) == "first" && File.Exists(first), "Windows copy preserves source");
                CheckShellMenu(new[] { first });
                CheckShellMenu(new[] { first, copied.Paths[first] });
                CheckShellMenu(new[] { first, "::{645FF040-5081-101B-9F08-00AA002F954E}" });
                var moved = operations.Transfer(IntPtr.Zero, new[] { second }, destination, TransferMode.Move).GetAwaiter().GetResult(); moved.ThrowIfFailed();
                Check(!File.Exists(second) && File.ReadAllText(moved.Paths[second]) == "second", "Windows move transfers the file");
                var renamed = operations.Change(IntPtr.Zero, new[] { moved.Paths[second] }, "rename", "renamed.txt").GetAwaiter().GetResult(); renamed.ThrowIfFailed();
                Check(renamed.Paths.Values.Single() == Path.Combine(destination, "renamed.txt"), "Windows rename reports actual destination");
                var duplicate = operations.Transfer(IntPtr.Zero, new[] { first }, source, TransferMode.Copy).GetAwaiter().GetResult(); duplicate.ThrowIfFailed();
                Check(duplicate.Paths[first] != first && File.ReadAllText(duplicate.Paths[first]) == "first", "Same-folder duplicate uses Windows collision naming");
                var folder = operations.Change(IntPtr.Zero, new[] { destination }, "folder", "新文件夹").GetAwaiter().GetResult(); folder.ThrowIfFailed();
                Check(Directory.Exists(folder.Paths.Values.Single()), "Windows folder creation");
                var template = new ShellTemplate { Extension = ".txt", Label = "Test document", Data = System.Text.Encoding.UTF8.GetBytes("Template content") };
                var created = operations.CreateFromTemplate(IntPtr.Zero, destination, "template.txt", template).GetAwaiter().GetResult(); created.ThrowIfFailed();
                Check(File.ReadAllText(created.Paths.Values.Single()) == "Template content", "Windows file creation preserves template content");
            }
            using (var catalog = new ShellCatalog(source))
            {
                var snapshot = catalog.Read(new[] { destination }).GetAwaiter().GetResult();
                Check(snapshot.Desktop.Any(e => e.Path == first), "Native directory enumeration");
                Check(snapshot.Folders[destination].Any(e => e.Name == "renamed.txt"), "Mapped folder enumeration");
                var icon = catalog.Icon(firstEntry, 64).GetAwaiter().GetResult();
                Check(icon != null && icon.IsFrozen, "Native Shell icon extraction on STA worker");
            }
            var monitors = DesktopMonitor.Read();
            Check(monitors.Count > 0 && monitors.Any(m => m.Primary), "Native monitor discovery");
            Check(monitors.All(m => !String.IsNullOrWhiteSpace(m.Id) && m.Scale > 0), "Stable monitor identity and DPI");
            Check(ShellTemplates.Read().All(template => template.Extension.StartsWith(".") && (template.Data != null || File.Exists(template.FileName))), "Windows registered file templates resolve to usable sources");
            Console.WriteLine("PASS: " + assertions + " assertions; real Windows Shell file operations completed."); return 0;
        }
        catch (Exception error) { Console.Error.WriteLine(error.GetType().FullName + ": " + error.Message + "\n" + error.StackTrace); return 1; }
        finally { Directory.Delete(directory, true); }
    }
}
