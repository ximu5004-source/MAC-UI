using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Windows;

namespace MacUI.Desktop.Native
{
    internal sealed class DesktopGroup
    {
        internal string Id, Title, Kind, Folder, Error;
        internal List<DesktopEntry> Items = new List<DesktopEntry>();
        internal bool Query { get { return Kind == "recent" || Kind == "frequent"; } }
    }
    internal sealed class DesktopModel
    {
        internal readonly LayoutStore Store;
        internal readonly Localizer Text;
        internal CatalogSnapshot Catalog = new CatalogSnapshot();
        internal Dictionary<string, object> Data { get { return Store.Data; } }
        internal Dictionary<string, object> Organization { get { return Json.Map(Data, "organization"); } }
        internal List<Dictionary<string, object>> Boxes { get { return Json.Array(Organization, "boxes").Cast<Dictionary<string, object>>().ToList(); } }
        internal int IconSize { get { return (int)Math.Max(16, Math.Min(256, Json.Number(Data, "iconSize", 64))); } }
        internal bool Locked { get { return Json.Flag(Data, "layoutLocked"); } }
        internal int GlassTransparency { get { return (int)Json.Number(Data, "glassTransparency", 88); } }
        internal int GlassBlur { get { return (int)Json.Number(Data, "glassBlur", 35); } }
        internal bool Categorized { get { return Json.Text(Data, "arrangeMode", "name") == "category"; } }
        internal event Action Changed;
        internal DesktopModel(LayoutStore store, Localizer text) { Store = store; Text = text; }
        internal void Notify() { if (Changed != null) Changed(); }
        internal void Save() { Store.Save(); Notify(); }
        internal string[] MappedFolders { get { return Boxes.Where(b => Json.Text(b, "folder") != "").Select(b => Json.Text(b, "folder")).ToArray(); } }
        internal string T(string key) { return Text["desktop_shell." + key]; }
        internal void ReplaceCatalog(CatalogSnapshot snapshot) { Catalog = snapshot; Notify(); }
        private static readonly Dictionary<string, string[]> Categories = new Dictionary<string, string[]> {
            {"images", "avif bmp gif heic jpeg jpg png svg webp".Split(' ')}, {"documents", "csv doc docx md pdf ppt pptx rtf txt xls xlsx".Split(' ')},
            {"videos", "avi m4v mkv mov mp4 webm wmv".Split(' ')}, {"audio", "aac flac m4a mp3 ogg wav wma".Split(' ')},
            {"archives", "7z gz rar tar zip".Split(' ')}, {"apps", "appref-ms bat cmd exe lnk msi".Split(' ')} };
        internal string Scope(DesktopEntry entry)
        {
            var assignments = Json.Map(Organization, "assignments");
            string assigned = Json.Text(assignments, entry.Key);
            if (assigned != "" && (!assigned.StartsWith("box:", StringComparison.Ordinal) || Boxes.Any(b => Json.Text(b, "id") == assigned && Json.Text(b, "kind") != "recent" && Json.Text(b, "kind") != "frequent"))) return assigned;
            string scope = "other";
            var matched = Boxes.FirstOrDefault(b => Json.Array(b, "extensions").Select(Convert.ToString).Contains(entry.Extension));
            if (matched != null) scope = Json.Text(matched, "id");
            else if (entry.IsNamespace) scope = "desktop";
            else if (Json.Text(Organization, "strategy") == "deskgo") scope = entry.IsDirectory ? "deskgo-folders" : entry.Extension == "lnk" ? "desktop" : "deskgo-files";
            else if (entry.IsDirectory) scope = "folders";
            else foreach (var category in Categories) if (category.Value.Contains(entry.Extension)) { scope = category.Key; break; }
            return Json.Array(Organization, "dismissedCategories").Contains(scope) ? "desktop" : scope;
        }
        internal List<DesktopGroup> Groups()
        {
            var result = new List<DesktopGroup>();
            if (!Categorized) result.Add(new DesktopGroup { Id = "all", Kind = "manual", Items = Sort(Catalog.Desktop, "all") });
            else
            {
                foreach (var group in Catalog.Desktop.GroupBy(Scope))
                    result.Add(new DesktopGroup { Id = group.Key, Title = T("categories." + group.Key), Kind = "manual", Items = Sort(group.ToList(), group.Key) });
            }
            var available = Catalog.Desktop.Concat(Catalog.Folders.Values.SelectMany(v => v)).GroupBy(e => e.Key).Select(g => g.First()).ToList();
            foreach (var box in Boxes)
            {
                string id = Json.Text(box, "id"), kind = Json.Text(box, "kind", "manual"), folder = Json.Text(box, "folder");
                if (!Categorized) continue;
                var group = result.FirstOrDefault(g => g.Id == id);
                if (group == null) { group = new DesktopGroup { Id = id }; result.Add(group); }
                group.Title = Json.Text(box, "title"); group.Kind = kind; group.Folder = folder;
                if (kind == "recent" || kind == "frequent") group.Items = available.Where(e => Json.Number(Activity(e.Key), "opens", 0) > 0 || (kind == "frequent" && Json.Flag(Activity(e.Key), "pinned"))).ToList();
                else if (folder != "")
                {
                    List<DesktopEntry> contents;
                    if (Catalog.Folders.TryGetValue(folder, out contents)) group.Items = group.Items.Concat(contents).GroupBy(e => e.Key).Select(g => g.First()).ToList();
                    string error; if (Catalog.Errors.TryGetValue(folder, out error)) group.Error = error;
                }
                group.Items = Sort(group.Items, id);
                if (group.Query) group.Items = group.Items.Take(100).ToList();
            }
            if (Categorized && !result.Any(g => g.Id == "desktop")) result.Add(new DesktopGroup { Id = "desktop", Kind = "manual" });
            return result;
        }
        internal Dictionary<string, object> Activity(string key)
        {
            object value; var activity = Json.Map(Data, "activity");
            return activity.TryGetValue(key, out value) ? (Dictionary<string, object>)value : new Dictionary<string, object>();
        }
        internal void Opened(DesktopEntry entry)
        {
            var activity = Activity(entry.Key); activity["opens"] = Json.Number(activity, "opens", 0) + 1;
            activity["lastOpened"] = (DateTime.UtcNow - new DateTime(1970, 1, 1)).TotalMilliseconds;
            Json.Map(Data, "activity")[entry.Key] = activity; Store.Save();
        }
        internal string SortMode(string id)
        {
            string mode = Json.Text(Json.Map(Data, "sortModes"), id);
            if (mode != "") return mode;
            if (Json.Array(Json.Map(Data, "iconOrders"), id).Count > 0) return "manual";
            var box = Boxes.FirstOrDefault(b => Json.Text(b, "id") == id);
            string kind = box == null ? "" : Json.Text(box, "kind");
            return kind == "recent" || kind == "frequent" ? kind : id == "deskgo-folders" ? "modified" : id == "deskgo-files" ? "time-grouped" : "name";
        }
        internal List<DesktopEntry> Sort(IEnumerable<DesktopEntry> entries, string id)
        {
            var result = entries.ToList(); var order = Json.Array(Json.Map(Data, "iconOrders"), id).Select(p => Json.Key(Convert.ToString(p))).ToList();
            string mode = SortMode(id), direction = Json.Text(Json.Map(Data, "sortDirections"), id);
            bool descending = mode == "modified" || mode == "time-grouped" || mode == "size";
            int reverse = direction == "" || (direction == "descending") == descending ? 1 : -1;
            result.Sort(delegate(DesktopEntry a, DesktopEntry b) {
                int comparison = 0;
                if (mode == "manual")
                {
                    int first = order.IndexOf(a.Key), second = order.IndexOf(b.Key);
                    comparison = (first < 0 ? int.MaxValue : first).CompareTo(second < 0 ? int.MaxValue : second);
                }
                else if (mode == "recent" || mode == "frequent")
                {
                    var first = Activity(a.Key); var second = Activity(b.Key);
                    if (mode == "frequent")
                    {
                        comparison = Json.Flag(second, "pinned").CompareTo(Json.Flag(first, "pinned"));
                        if (comparison == 0) comparison = Json.Number(second, "opens", 0).CompareTo(Json.Number(first, "opens", 0));
                    }
                    if (comparison == 0) comparison = Json.Number(second, "lastOpened", 0).CompareTo(Json.Number(first, "lastOpened", 0));
                }
                else
                {
                    if (mode != "time-grouped" && a.IsDirectory != b.IsDirectory) return b.IsDirectory.CompareTo(a.IsDirectory);
                    if (mode == "type") comparison = StringComparer.CurrentCultureIgnoreCase.Compare(a.Extension, b.Extension);
                    else if (mode == "modified" || mode == "time-grouped") comparison = b.Modified.CompareTo(a.Modified);
                    else if (mode == "size") comparison = b.Size.CompareTo(a.Size);
                }
                if (comparison == 0) comparison = Win32.StrCmpLogicalW(a.Name, b.Name);
                return comparison * reverse;
            });
            return result;
        }
        internal void Reorder(string scope, IList<DesktopEntry> ordered, string[] moved, string before)
        {
            var keys = new HashSet<string>(moved.Select(Json.Key));
            if (before != null && keys.Contains(Json.Key(before))) return;
            var result = ordered.Select(e => e.Path).Where(p => !keys.Contains(Json.Key(p))).ToList();
            int at = result.FindIndex(p => Json.Key(p) == Json.Key(before ?? ""));
            if (at < 0) at = result.Count;
            result.InsertRange(at, moved);
            Json.Map(Data, "iconOrders")[scope] = result;
            Json.Map(Data, "sortModes")[scope] = "manual";
            if (scope != "all") foreach (string path in moved) Json.Map(Organization, "assignments")[Json.Key(path)] = scope;
            Save();
        }
        internal void Assign(string[] paths, string scope)
        {
            foreach (var path in paths) Json.Map(Organization, "assignments")[Json.Key(path)] = scope;
            Save();
        }
        internal void Automatic(string[] paths)
        {
            foreach (var path in paths) Json.Map(Organization, "assignments").Remove(Json.Key(path));
            Save();
        }
        internal void RenamePath(string oldPath, string newPath)
        {
            string key = Json.Key(oldPath), replacement = Json.Key(newPath); object value;
            foreach (var map in new[] { Json.Map(Organization, "assignments"), Json.Map(Data, "activity") })
                if (map.TryGetValue(key, out value)) { map.Remove(key); map[replacement] = value; }
            var orders = Json.Map(Data, "iconOrders");
            foreach (string scope in orders.Keys.ToArray()) orders[scope] = Json.Array(orders, scope).Select(p => Json.Key(Convert.ToString(p)) == key ? newPath : Convert.ToString(p)).ToArray();
            foreach (var scope in Json.Map(Data, "looseIconPositions").Values.Cast<Dictionary<string, object>>())
                if (scope.TryGetValue(key, out value)) { scope.Remove(key); scope[replacement] = value; }
        }
        internal string AddBox(string title, string kind, string folder, string monitor = null)
        {
            if (Boxes.Count >= 64) throw new InvalidOperationException(T("boxes.box_limit"));
            string id = "box:" + Guid.NewGuid().ToString("N"); var boxes = Boxes;
            boxes.Add(new Dictionary<string, object> { { "id", id }, { "title", title }, { "kind", kind }, { "folder", folder }, { "extensions", new string[0] } });
            Organization["boxes"] = boxes; Data["arrangeMode"] = "category";
            if (monitor != null) Json.Map(Organization, "monitors")[id] = monitor;
            Save(); return id;
        }
        internal void Dissolve(DesktopGroup group)
        {
            if (Locked) return;
            foreach (var item in group.Items.Where(e => e.IsDesktop)) Json.Map(Organization, "assignments")[item.Key] = "desktop";
            if (group.Id.StartsWith("box:", StringComparison.Ordinal)) Organization["boxes"] = Boxes.Where(b => Json.Text(b, "id") != group.Id).ToArray();
            else { var dismissed = Json.Array(Organization, "dismissedCategories"); if (!dismissed.Contains(group.Id)) dismissed.Add(group.Id); Organization["dismissedCategories"] = dismissed; }
            foreach (var name in new[] { "positions", "sizes", "iconOrders", "sortModes", "sortDirections" }) Json.Map(Data, name).Remove(group.Id);
            Json.Map(Organization, "monitors").Remove(group.Id);
            Save();
        }
        internal object Controls()
        {
            var spacing = Json.Map(Data, "iconSpacing");
            return new { arrangeMode = Categorized ? "category" : "name", iconSize = IconSize, snapToGrid = Json.Flag(Data, "snapToGrid", true), showGrid = Json.Flag(Data, "showGrid"), layoutLocked = Locked, glassTransparency = GlassTransparency, glassBlur = GlassBlur,
                snapBoxes = Json.Flag(Data, "snapBoxes", true), horizontalSpacing = Json.Number(spacing, "horizontal", 8), verticalSpacing = Json.Number(spacing, "vertical", 10) };
        }
        internal void Patch(Dictionary<string, object> patch)
        {
            foreach (var value in patch)
            {
                if (value.Key == "arrangeMode" && (Equals(value.Value, "name") || Equals(value.Value, "category"))) continue;
                if (value.Key == "iconSize" && value.Value is int && (int)value.Value >= 16 && (int)value.Value <= 256) continue;
                if ((value.Key == "glassTransparency" || value.Key == "glassBlur") && value.Value is int && (int)value.Value >= 0 && (int)value.Value <= 100) continue;
                if ((value.Key == "horizontalSpacing" || value.Key == "verticalSpacing") && value.Value is int && (int)value.Value >= 0 && (int)value.Value <= 64) continue;
                if (new[] { "snapToGrid", "showGrid", "layoutLocked", "snapBoxes" }.Contains(value.Key) && value.Value is bool) continue;
                throw new ArgumentException("Invalid desktop setting: " + value.Key);
            }
            foreach (var value in patch)
            {
                if (value.Key == "horizontalSpacing") Json.Map(Data, "iconSpacing")["horizontal"] = value.Value;
                else if (value.Key == "verticalSpacing") Json.Map(Data, "iconSpacing")["vertical"] = value.Value;
                else Data[value.Key] = value.Value;
            }
            Save();
        }
        internal Point Position(string scope, DesktopMonitor monitor, Point defaultValue)
        {
            object value;
            var native = Json.Map(Json.Map(Json.Map(Data, "nativeDesktop"), "layouts"), monitor.Id);
            if (Json.Map(native, "positions").TryGetValue(scope, out value))
            { var nativePoint = (Dictionary<string, object>)value; return new Point(Json.Number(nativePoint, "x", defaultValue.X), Json.Number(nativePoint, "y", defaultValue.Y)); }
            var map = Geometry("positions", monitor.Id);
            if (!map.TryGetValue(scope, out value)) return defaultValue;
            var point = (Dictionary<string, object>)value;
            return new Point((Json.Number(point, "x", 0) + 24) / monitor.Scale, (Json.Number(point, "y", 0) + 112 - (monitor.Work.Top - monitor.Bounds.Top)) / monitor.Scale);
        }
        internal Size Size(string scope, DesktopMonitor monitor, Size defaultValue)
        {
            object value;
            var native = Json.Map(Json.Map(Json.Map(Data, "nativeDesktop"), "layouts"), monitor.Id);
            if (Json.Map(native, "sizes").TryGetValue(scope, out value))
            { var dimensions = (Dictionary<string, object>)value; return new Size(Math.Max(180, Json.Number(dimensions, "width", defaultValue.Width)), Math.Max(120, Json.Number(dimensions, "height", defaultValue.Height))); }
            var map = Geometry("sizes", monitor.Id);
            if (!map.TryGetValue(scope, out value)) return defaultValue;
            var size = (Dictionary<string, object>)value;
            return new Size(Math.Max(180, Json.Number(size, "width", defaultValue.Width) / monitor.Scale), Math.Max(120, Json.Number(size, "height", defaultValue.Height) / monitor.Scale));
        }
        private Dictionary<string, object> Geometry(string kind, string monitor)
        {
            var layouts = Json.Map(Organization, "layouts"); object value;
            return layouts.TryGetValue(monitor, out value) && ((Dictionary<string, object>)value).ContainsKey(kind)
                ? Json.Map((Dictionary<string, object>)value, kind) : Json.Map(Data, kind);
        }
        internal void SaveGeometry(string scope, string monitor, Point point, Size size)
        {
            var layout = Json.Map(Json.Map(Json.Map(Data, "nativeDesktop"), "layouts"), monitor);
            Json.Map(layout, "positions")[scope] = new Dictionary<string, object> { { "x", point.X }, { "y", point.Y } };
            Json.Map(layout, "sizes")[scope] = new Dictionary<string, object> { { "width", size.Width }, { "height", size.Height } };
            Json.Map(Organization, "monitors")[scope] = monitor;
            Store.Save();
        }
        internal void MoveLoose(string scope, string[] paths, Point point, string monitor)
        {
            var positions = Json.Map(Json.Map(Data, "looseIconPositions"), scope);
            var cell = DesktopLayout.Cell(this);
            if (Json.Flag(Data, "snapToGrid", true)) point = DesktopLayout.SnapIcon(point, cell);
            int index = 0;
            foreach (string path in paths)
            {
                positions[Json.Key(path)] = new Dictionary<string, object> { { "x", point.X + index++ * cell.Width }, { "y", point.Y }, { "monitor", monitor }, { "coordinateUnit", "dip" } };
                if (scope != "all") Json.Map(Organization, "assignments")[Json.Key(path)] = "desktop";
            }
            Save();
        }
    }
}
