using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;

namespace MacUI.Desktop.Native
{
    internal static class Dialogs
    {
        internal static Window Create(Window owner, string title, Panel content, Localizer text, Action accept)
        {
            var dialog = new Window { Owner = owner, Title = title, Width = 440, SizeToContent = SizeToContent.Height, MinHeight = 170,
                WindowStartupLocation = WindowStartupLocation.CenterOwner, ResizeMode = ResizeMode.NoResize, ShowInTaskbar = false,
                WindowStyle = WindowStyle.None, AllowsTransparency = true, Background = System.Windows.Media.Brushes.Transparent, Foreground = Glass.Ink, FontFamily = owner.FontFamily, FontSize = 13 };
            GlassControls.Apply(dialog);
            NativeGlass frost = null;
            dialog.SourceInitialized += delegate { frost = new NativeGlass(dialog); };
            dialog.Closed += delegate { if (frost != null) frost.Dispose(); };
            var layout = new StackPanel { Margin = new Thickness(22) };
            var heading = Glass.Label(title, 20); heading.FontWeight = FontWeights.SemiBold; heading.Margin = new Thickness(0, 0, 0, 18); layout.Children.Add(heading);
            heading.MouseLeftButtonDown += delegate { dialog.DragMove(); };
            layout.Children.Add(content);
            var buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Margin = new Thickness(0, 18, 0, 0) };
            var cancel = Glass.Button(text["desktop_shell.boxes.cancel"], delegate { dialog.DialogResult = false; }); cancel.IsCancel = true;
            var save = Glass.Button(text["desktop_shell.boxes.save"], delegate {
                try { accept(); dialog.DialogResult = true; }
                catch (ArgumentException error) { MessageBox.Show(dialog, error.Message, title, MessageBoxButton.OK, MessageBoxImage.Warning); }
            }); save.IsDefault = true;
            buttons.Children.Add(cancel); buttons.Children.Add(save); layout.Children.Add(buttons); dialog.Content = GlassControls.Frame(layout, false); return dialog;
        }
        internal static TextBox Field(Panel panel, string label, string value)
        {
            var caption = Glass.Label(label); caption.Margin = new Thickness(0, 12, 0, 6); panel.Children.Add(caption);
            var field = new TextBox { Text = value, Padding = new Thickness(10, 7, 10, 7), BorderThickness = new Thickness(0), Background = Glass.Brush(210, 255, 255, 255) };
            System.Windows.Automation.AutomationProperties.SetName(field, label); panel.Children.Add(field); return field;
        }
        internal static string Text(Window owner, string title, string value, Localizer text)
        {
            var panel = new StackPanel(); var field = Field(panel, text["desktop_shell.boxes.name"], value); string result = null;
            var dialog = Create(owner, title, panel, text, delegate {
                string name = field.Text.Trim();
                if (name.Length == 0 || name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0 || name == "." || name == "..") throw new ArgumentException(text["desktop_shell.boxes.invalid_box"]);
                result = name;
            });
            dialog.Loaded += delegate { field.Focus(); field.SelectAll(); };
            return dialog.ShowDialog() == true ? result : null;
        }
    }
    internal sealed partial class DesktopSession
    {
        internal void FileMenu(DesktopWindow window, IconTile tile)
        {
            var menu = Glass.Menu(this); var entries = Selection(); bool files = entries.All(e => !e.IsNamespace);
            menu.Items.Add(Glass.Item(Model.T("menu.open"), delegate { Open(window, tile.Entry); }));
            menu.Items.Add(Glass.Item(Model.T("files.copy"), delegate { Copy(false); }, files));
            menu.Items.Add(Glass.Item(Model.T("files.cut"), delegate { Copy(true); }, files));
            menu.Items.Add(Glass.Item(Model.T("menu.rename"), delegate { Rename(window); }, files && entries.Length == 1));
            menu.Items.Add(Glass.Item(Model.T("files.delete"), delegate { Delete(window, false); }, files));
            menu.Items.Add(new Separator());
            var move = new MenuItem { Header = Model.T("boxes.move_to"), IsEnabled = entries.All(e => e.IsDesktop) };
            move.Items.Add(Glass.Item(Model.T("boxes.automatic"), delegate { Model.Automatic(entries.Select(e => e.Path).ToArray()); }));
            move.Items.Add(Glass.Item(Model.T("categories.desktop"), delegate { Model.Assign(entries.Select(e => e.Path).ToArray(), "desktop"); }));
            foreach (var group in Model.Groups().Where(g => !g.Query && g.Id != "all" && g.Id != "desktop" && String.IsNullOrEmpty(g.Folder)))
            { var destination = group; move.Items.Add(Glass.Item(destination.Title, delegate { Model.Assign(entries.Select(e => e.Path).ToArray(), destination.Id); })); }
            menu.Items.Add(move);
            bool pinned = Json.Flag(Model.Activity(tile.Entry.Key), "pinned");
            menu.Items.Add(Glass.Item(Model.T(pinned ? "activity.unpin" : "activity.pin"), delegate {
                foreach (var entry in entries) { var activity = Model.Activity(entry.Key); activity["pinned"] = !pinned; Json.Map(Model.Data, "activity")[entry.Key] = activity; }
                Model.Save();
            }));
            menu.Items.Add(new Separator());
            menu.Items.Add(Glass.Item(Model.Text["native.windows_menu"], delegate {
                try { if (ShellMenu.Show(window, entries.Select(e => e.Path).ToArray())) Rename(window); Refresh(); }
                catch (Exception error) { Error(error); }
            }));
            menu.Items.Add(Glass.Item(Model.T("files.properties"), delegate {
                try { ShellMenu.Open(window.Handle, tile.Entry.Path, "properties"); } catch (Exception error) { Error(error); }
            }));
            menu.PlacementTarget = tile; menu.IsOpen = true;
        }
        internal void GroupMenu(DesktopWindow window)
        {
            if (window.Loose)
            {
                try { ShellMenu.Background(window, Model.Text["native.desktop_menu"], DesktopCommands(window)); Refresh(); }
                catch (Exception error) { Error(error); }
                return;
            }
            var menu = Glass.Menu(this);
            menu.Items.Add(Glass.Item(Model.T("files.paste"), delegate { Paste(window); }, !window.Group.Query));
            menu.Items.Add(Glass.Item(Model.T("menu.new_folder"), delegate { NewFile(window, true); }, !window.Group.Query));
            var create = new MenuItem { Header = Model.T("menu.new_file"), IsEnabled = !window.Group.Query };
            create.Items.Add(Glass.Item(Model.T("file_types.text"), delegate { NewFile(window, false); }));
            menu.Items.Add(create); LoadTemplates(create, window);
            menu.Items.Add(new Separator());
            var sort = new MenuItem { Header = Model.T("sort.label") };
            foreach (var mode in new[] { "name", "type", "modified", "size", "time-grouped", "manual" })
            {
                string selected = mode;
                var item = Glass.Item(Model.T("sort." + mode), delegate { Json.Map(Model.Data, "sortModes")[window.Group.Id] = selected; Model.Save(); });
                item.IsCheckable = true; item.IsChecked = Model.SortMode(window.Group.Id) == mode; sort.Items.Add(item);
            }
            menu.Items.Add(sort);
            if (!window.Loose)
            {
                if (window.Group.Id.StartsWith("box:", StringComparison.Ordinal))
                    menu.Items.Add(Glass.Item(Model.Text["native.map_folder"], delegate { MapFolder(window); }));
                menu.Items.Add(Glass.Item(Model.T("boxes.edit"), delegate { EditBox(window, window.Group); }, window.Group.Id.StartsWith("box:", StringComparison.Ordinal)));
                menu.Items.Add(Glass.Item(Model.Text["native.dissolve"], delegate { Model.Dissolve(window.Group); }, !Model.Locked));
                if (!String.IsNullOrEmpty(window.Group.Folder)) menu.Items.Add(Glass.Item(Model.T("menu.open_folder"), delegate {
                    try { ShellMenu.Open(window.Handle, window.Group.Folder); } catch (Exception error) { Error(error); }
                }));
            }
            menu.Items.Add(Glass.Item(Model.T("boxes.create"), delegate { CreateBox(window); }));
            menu.Items.Add(Glass.Item(Model.T("activity.recent"), delegate { Model.AddBox(Model.T("activity.recent"), "recent", ""); }));
            menu.Items.Add(Glass.Item(Model.T("activity.frequent"), delegate { Model.AddBox(Model.T("activity.frequent"), "frequent", ""); }));
            menu.Items.Add(new Separator());
            menu.Items.Add(Glass.Item(Model.T("organize.now"), Organize));
            menu.Items.Add(Glass.Item(Model.T("arrange.name"), delegate { Model.Patch(new Dictionary<string, object> { { "arrangeMode", "name" } }); }));
            menu.Items.Add(Glass.Item(Model.T("controls_label"), OpenSettings, !Preview));
            menu.Items.Add(Glass.Item(Model.T("menu.refresh"), Refresh));
            menu.Items.Add(Glass.Item(Model.T("backup.create"), delegate { Model.Store.Snapshot(); }));
            menu.Items.Add(Glass.Item(Model.T("backup.restore_manual"), delegate { try { Model.Store.Restore(true); Model.Notify(); } catch (Exception error) { Error(error); } }, File.Exists(Path.Combine(Model.Store.DirectoryPath, "desktop-shell.manual-backup.json"))));
            menu.Items.Add(Glass.Item(Model.T("backup.restore_previous"), delegate { try { Model.Store.Restore(false); Model.Notify(); } catch (Exception error) { Error(error); } }, File.Exists(Path.Combine(Model.Store.DirectoryPath, "desktop-shell.backup.json"))));
            if (Preview) menu.Items.Add(Glass.Item(Model.Text["native.close_preview"], RequestStop));
            menu.PlacementTarget = window; menu.IsOpen = true;
        }
        internal NativeMenuItem[] DesktopCommands(DesktopWindow window)
        {
            return new[] {
                new NativeMenuItem(Model.T("boxes.create"), delegate { CreateBox(window); }),
                new NativeMenuItem(Model.T("organize.now"), Organize),
                new NativeMenuItem(Model.T("snap_grid"), delegate { Model.Patch(new Dictionary<string, object> { { "snapToGrid", !Json.Flag(Model.Data, "snapToGrid", true) } }); }) { Checked = Json.Flag(Model.Data, "snapToGrid", true) },
                new NativeMenuItem(Model.T("show_grid"), delegate { Model.Patch(new Dictionary<string, object> { { "showGrid", !Json.Flag(Model.Data, "showGrid") } }); }) { Checked = Json.Flag(Model.Data, "showGrid") },
                new NativeMenuItem(Model.T("organize.lock"), delegate { Model.Patch(new Dictionary<string, object> { { "layoutLocked", !Model.Locked } }); }) { Checked = Model.Locked },
                new NativeMenuItem(null),
                new NativeMenuItem(Model.T("controls_label"), OpenSettings) { Enabled = !Preview },
                new NativeMenuItem(Model.T("backup.create"), delegate { Model.Store.Snapshot(); }),
                new NativeMenuItem(Model.T("backup.restore_manual"), delegate { Model.Store.Restore(true); Model.Notify(); }) { Enabled = File.Exists(Path.Combine(Model.Store.DirectoryPath, "desktop-shell.manual-backup.json")) }
            };
        }
        internal void CreateBox(DesktopWindow owner)
        {
            if (!Model.Categorized)
                foreach (var entry in Model.Catalog.Desktop) Json.Map(Model.Organization, "assignments")[entry.Key] = "desktop";
            string title = Model.T("boxes.new_name"), candidate = title; int suffix = 2;
            while (Model.Boxes.Any(box => Json.Text(box, "title") == candidate)) candidate = title + " " + suffix++;
            string id = Model.AddBox(candidate, "manual", "", owner.Monitor.Id);
            var created = windows.First(window => window.Group.Id == id);
            Win32.RECT rect; Win32.GetWindowRect(created.Handle, out rect);
            var size = new Size(created.Width, created.Height);
            var point = PlaceBox(created, new Point((rect.Left - created.Monitor.Work.Left) / created.Monitor.Scale, (rect.Top - created.Monitor.Work.Top) / created.Monitor.Scale), size);
            created.Place(point, size); Model.SaveGeometry(id, created.Monitor.Id, point, size);
            created.Activate();
        }
        private void MapFolder(DesktopWindow owner)
        {
            using (var picker = new System.Windows.Forms.FolderBrowserDialog { Description = Model.T("boxes.folder"), SelectedPath = owner.Group.Folder ?? "" })
            {
                if (picker.ShowDialog(new NativeOwner(owner.Handle)) != System.Windows.Forms.DialogResult.OK) return;
                var box = Model.Boxes.First(value => Json.Text(value, "id") == owner.Group.Id);
                box["folder"] = picker.SelectedPath; box["kind"] = "folder";
                Model.Store.Save(); Refresh();
            }
        }
        private async void LoadTemplates(MenuItem menu, DesktopWindow window)
        {
            try
            {
                var templates = await System.Threading.Tasks.Task.Run(() => ShellTemplates.Read());
                foreach (var template in templates.Where(item => !item.Extension.Equals(".txt", StringComparison.OrdinalIgnoreCase)))
                { var selected = template; menu.Items.Add(Glass.Item(selected.Label, delegate { NewFile(window, false, selected); })); }
            }
            catch (Exception error) { Log("templates", error); }
        }
        internal void EditBox(DesktopWindow owner, DesktopGroup group)
        {
            var existing = group == null ? null : Model.Boxes.First(b => Json.Text(b, "id") == group.Id);
            var panel = new StackPanel();
            var name = Dialogs.Field(panel, Model.T("boxes.name"), existing == null ? Model.T("boxes.new_name") : Json.Text(existing, "title"));
            var extensions = Dialogs.Field(panel, Model.T("boxes.extensions"), existing == null ? "" : String.Join(", ", Json.Array(existing, "extensions")));
            var folder = Dialogs.Field(panel, Model.T("boxes.folder"), existing == null ? "" : Json.Text(existing, "folder"));
            panel.Children.Add(Glass.Button(Model.T("boxes.choose_folder"), delegate {
                using (var picker = new System.Windows.Forms.FolderBrowserDialog { SelectedPath = folder.Text, Description = Model.T("boxes.folder") })
                    if (picker.ShowDialog(new NativeOwner(owner.Handle)) == System.Windows.Forms.DialogResult.OK) folder.Text = picker.SelectedPath;
            }));
            var monitor = new ComboBox { ItemsSource = monitors.Select(m => m.Id).ToArray(), SelectedItem = owner.Monitor.Id, Margin = new Thickness(0, 10, 0, 0) };
            panel.Children.Add(Glass.Label(Model.T("boxes.monitor"))); panel.Children.Add(monitor);
            var dialog = Dialogs.Create(owner, Model.T(group == null ? "boxes.create" : "boxes.edit"), panel, Model.Text, delegate {
                if (name.Text.Trim().Length == 0) throw new ArgumentException(Model.T("boxes.invalid_box"));
                string path = folder.Text.Trim();
                if (path != "" && !Directory.Exists(path)) throw new ArgumentException(Model.T("boxes.mapping_unavailable"));
                var values = extensions.Text.Split(new[] { ',', ';', ' ', '，' }, StringSplitOptions.RemoveEmptyEntries).Select(v => v.TrimStart('.').ToLowerInvariant()).Distinct().ToArray();
                string id = group == null ? Model.AddBox(name.Text.Trim(), path == "" ? "manual" : "folder", path) : group.Id;
                var box = Model.Boxes.First(b => Json.Text(b, "id") == id);
                box["title"] = name.Text.Trim(); box["extensions"] = values; box["folder"] = path;
                if (group == null || !group.Query) box["kind"] = path == "" ? "manual" : "folder";
                Json.Map(Model.Organization, "monitors")[id] = Convert.ToString(monitor.SelectedItem);
                Model.Save(); Refresh();
            });
            dialog.ShowDialog(); Restack();
        }
        private sealed class NativeOwner : System.Windows.Forms.IWin32Window
        { public IntPtr Handle { get; private set; } internal NativeOwner(IntPtr handle) { Handle = handle; } }
        internal void OpenSettings() { if (Send != null) Send("open-settings", new { }); }
    }
}
