using System;
using System.Collections.Generic;
using System.Linq;
using System.Windows;

namespace MacUI.Desktop.Native
{
    internal static class DesktopLayout
    {
        internal const double BoxGrid = 20;
        internal static Size Cell(DesktopModel model)
        {
            var spacing = Json.Map(model.Data, "iconSpacing");
            return new Size(model.IconSize + 30 + Json.Number(spacing, "horizontal", 8), model.IconSize + 52 + Json.Number(spacing, "vertical", 10));
        }
        internal static Point SnapIcon(Point point, Size cell)
        { return new Point(12 + Math.Max(0, Math.Round((point.X - 12) / cell.Width)) * cell.Width, 58 + Math.Max(0, Math.Round((point.Y - 58) / cell.Height)) * cell.Height); }
        internal static void ReserveToolbar(DesktopMonitor monitor, Dictionary<string, object> toolbar)
        {
            if (!Json.Flag(toolbar, "enabled")) return;
            int height = (int)Math.Ceiling((Json.Number(toolbar, "itemSize", 16) + 2 * Json.Number(toolbar, "padding", 8) + 2 * Json.Number(toolbar, "margin", 0)) * monitor.Scale);
            if (Json.Text(toolbar, "position", "Top") == "Top") monitor.Work.Top = Math.Max(monitor.Work.Top, monitor.Bounds.Top + height);
            else monitor.Work.Bottom = Math.Min(monitor.Work.Bottom, monitor.Bounds.Bottom - height);
        }
        internal static Point ClampBox(Point point, Size size, DesktopMonitor monitor)
        { return new Point(Math.Max(0, Math.Min(monitor.Width - size.Width, point.X)), Math.Max(0, Math.Min(monitor.Height - size.Height, point.Y))); }
        internal static DesktopMonitor MonitorAt(Point cursor, IEnumerable<DesktopMonitor> monitors)
        {
            return monitors.OrderBy(m => {
                double x = Math.Max(m.Bounds.Left - cursor.X, Math.Max(0, cursor.X - m.Bounds.Right));
                double y = Math.Max(m.Bounds.Top - cursor.Y, Math.Max(0, cursor.Y - m.Bounds.Bottom));
                return x * x + y * y;
            }).First();
        }
        internal static Point DragPosition(Point cursor, Point offset, DesktopMonitor monitor)
        { return new Point((cursor.X - monitor.Work.Left) / monitor.Scale - offset.X, (cursor.Y - monitor.Work.Top) / monitor.Scale - offset.Y); }
        internal static Point AlignBox(Point desired, Size size, DesktopMonitor monitor, IEnumerable<Rect> occupied, bool snapGrid = false)
        {
            var point = snapGrid ? new Point(Math.Round(desired.X / BoxGrid) * BoxGrid, Math.Round(desired.Y / BoxGrid) * BoxGrid) : desired;
            point = ClampBox(point, size, monitor);
            var neighbours = occupied.Where(r => r.Right + 36 >= point.X && r.Left - 36 <= point.X + size.Width
                && r.Bottom + 36 >= point.Y && r.Top - 36 <= point.Y + size.Height).ToArray();
            var horizontal = neighbours.SelectMany(r => new[] { r.Left, r.Right - size.Width, r.Right + 12, r.Left - size.Width - 12 })
                .Concat(new[] { 0.0, monitor.Width - size.Width }).OrderBy(x => Math.Abs(x - desired.X));
            var vertical = neighbours.SelectMany(r => new[] { r.Top, r.Bottom - size.Height, r.Bottom + 12, r.Top - size.Height - 12 })
                .Concat(new[] { 0.0, monitor.Height - size.Height }).OrderBy(y => Math.Abs(y - desired.Y));
            double edgeX = horizontal.First(), edgeY = vertical.First();
            if (Math.Abs(edgeX - desired.X) <= 12) point.X = edgeX;
            if (Math.Abs(edgeY - desired.Y) <= 12) point.Y = edgeY;
            return ClampBox(point, size, monitor);
        }
        internal static Point PlaceBox(Point desired, Size size, DesktopMonitor monitor, IEnumerable<Rect> occupied, bool snap, bool snapEdges = true)
        {
            var point = desired;
            if (snap) point = new Point(Math.Round(point.X / BoxGrid) * BoxGrid, Math.Round(point.Y / BoxGrid) * BoxGrid);
            point = ClampBox(point, size, monitor);
            var rectangles = occupied.ToArray();
            if (snapEdges)
            {
                point = AlignBox(desired, size, monitor, rectangles, snap);
            }
            if (!rectangles.Any(r => r.IntersectsWith(new Rect(point, size)))) return point;
            var xs = rectangles.SelectMany(r => new[] { r.Left - size.Width - 12, r.Right + 12, r.Left }).Concat(new[] { point.X, 0, monitor.Width - size.Width });
            var ys = rectangles.SelectMany(r => new[] { r.Top - size.Height - 12, r.Bottom + 12, r.Top }).Concat(new[] { point.Y, 0, monitor.Height - size.Height });
            var candidates = (from x in xs from y in ys select new Point(x, y))
                .Where(p => p == ClampBox(p, size, monitor) && !rectangles.Any(r => r.IntersectsWith(new Rect(p, size))))
                .OrderBy(p => (p - desired).LengthSquared).ToArray();
            return candidates.Length > 0 ? candidates[0] : point;
        }
    }
}
