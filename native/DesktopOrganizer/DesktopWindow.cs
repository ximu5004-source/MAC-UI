using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Shapes;

namespace MacUI.Desktop.Native
{
    internal sealed class DesktopMonitor
    {
        internal string Id;
        internal Win32.RECT Work, Bounds;
        internal double Scale;
        internal bool Primary;
        internal double Width { get { return (Work.Right - Work.Left) / Scale; } }
        internal double Height { get { return (Work.Bottom - Work.Top) / Scale; } }
        internal static List<DesktopMonitor> Read()
        {
            var result = new List<DesktopMonitor>();
            var identities = MonitorIdentity.Read();
            Win32.EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, delegate(IntPtr monitor, IntPtr dc, ref Win32.RECT rectangle, IntPtr data) {
                var info = new Win32.MONITORINFO { Size = Marshal.SizeOf(typeof(Win32.MONITORINFO)) };
                if (!Win32.GetMonitorInfo(monitor, ref info)) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
                uint x, y; Win32.Check(Win32.GetDpiForMonitor(monitor, 0, out x, out y));
                string identity;
                result.Add(new DesktopMonitor { Id = identities.TryGetValue(info.Device, out identity) ? identity : info.Device, Work = info.Work, Bounds = info.Monitor, Scale = x / 96.0, Primary = (info.Flags & 1) != 0 });
                return true;
            }, IntPtr.Zero);
            return result.OrderByDescending(m => m.Primary).ToList();
        }
    }

    internal static class Glass
    {
        internal static SolidColorBrush Brush(byte a, byte r, byte g, byte b) { var brush = new SolidColorBrush(Color.FromArgb(a, r, g, b)); brush.Freeze(); return brush; }
        internal static readonly Brush Ink = Brush(255, 26, 41, 60), Accent = Brush(255, 22, 92, 214);
        internal static Button Button(string text, Action action, bool compact = false)
        {
            var button = new Button { Content = text, Padding = compact ? new Thickness(8, 3, 8, 3) : new Thickness(14, 7, 14, 7),
                Margin = new Thickness(3), Background = Brush(90, 255, 255, 255), Foreground = Ink, BorderThickness = new Thickness(0), Cursor = Cursors.Hand };
            button.FocusVisualStyle = GlassControls.ButtonFocus(compact);
            if (compact) { button.Width = button.Height = 30; button.Padding = new Thickness(0); button.VerticalAlignment = VerticalAlignment.Center; }
            var template = new ControlTemplate(typeof(Button));
            var border = new FrameworkElementFactory(typeof(Border)); border.SetValue(Border.CornerRadiusProperty, new CornerRadius(compact ? 15 : 16));
            border.SetBinding(Border.BackgroundProperty, new System.Windows.Data.Binding("Background") { RelativeSource = new System.Windows.Data.RelativeSource(System.Windows.Data.RelativeSourceMode.TemplatedParent) });
            var presenter = new FrameworkElementFactory(typeof(ContentPresenter));
            presenter.SetValue(FrameworkElement.HorizontalAlignmentProperty, HorizontalAlignment.Center); presenter.SetValue(FrameworkElement.VerticalAlignmentProperty, VerticalAlignment.Center);
            presenter.SetBinding(FrameworkElement.MarginProperty, new System.Windows.Data.Binding("Padding") { RelativeSource = new System.Windows.Data.RelativeSource(System.Windows.Data.RelativeSourceMode.TemplatedParent) });
            border.AppendChild(presenter); template.VisualTree = border; button.Template = template;
            button.Click += delegate { action(); }; return button;
        }
        internal static FrameworkElement MoreIcon()
        {
            var canvas = new Canvas { Width = 16, Height = 16, IsHitTestVisible = false };
            foreach (double left in new[] { 1.0, 6.5, 12.0 })
            { var dot = new Ellipse { Width = 3, Height = 3, Fill = Ink }; Canvas.SetLeft(dot, left); Canvas.SetTop(dot, 6.5); canvas.Children.Add(dot); }
            return canvas;
        }
        internal static FrameworkElement CollapseIcon(bool folded)
        {
            var icon = new System.Windows.Shapes.Path { Width = 12, Height = 8, Stretch = Stretch.Fill, Fill = Ink, IsHitTestVisible = false,
                Data = Geometry.Parse("M 1.5,8 Q 0,8 0.9,6.7 L 5,1.1 Q 6,-0.3 7,1.1 L 11.1,6.7 Q 12,8 10.5,8 Z") };
            if (folded) { icon.RenderTransformOrigin = new Point(0.5, 0.5); icon.RenderTransform = new RotateTransform(180); }
            return icon;
        }
        internal static TextBlock Label(string text, double size = 12)
        { return new TextBlock { Text = text, FontSize = size, Foreground = Ink, TextWrapping = TextWrapping.Wrap, VerticalAlignment = VerticalAlignment.Center }; }
        internal static MenuItem Item(string text, Action action, bool enabled = true)
        { var item = new MenuItem { Header = text, IsEnabled = enabled, Padding = new Thickness(12, 7, 12, 7) }; item.Click += delegate { action(); }; return item; }
        internal static ContextMenu Menu(DesktopSession session) { return new GlassContextMenu(session); }
    }

    internal sealed class IconTile : Border
    {
        internal DesktopEntry Entry;
        internal int IconRevision = 1;
        private readonly Image image;
        private readonly TextBlock label;
        protected override System.Windows.Automation.Peers.AutomationPeer OnCreateAutomationPeer() { return new IconTilePeer(this); }
        private sealed class IconTilePeer : System.Windows.Automation.Peers.FrameworkElementAutomationPeer
        {
            internal IconTilePeer(IconTile owner) : base(owner) { }
            protected override string GetClassNameCore() { return "DesktopIcon"; }
            protected override System.Windows.Automation.Peers.AutomationControlType GetAutomationControlTypeCore() { return System.Windows.Automation.Peers.AutomationControlType.ListItem; }
            protected override string GetNameCore() { return ((IconTile)Owner).Entry.Name; }
        }
        internal IconTile(DesktopEntry entry, int size, bool loose)
        {
            Entry = entry; CornerRadius = new CornerRadius(10); BorderThickness = new Thickness(1); Background = Brushes.Transparent;
            Padding = new Thickness(5); Focusable = true; Cursor = Cursors.Arrow; ToolTip = entry.Name;
            AutomationProperties.SetName(this, entry.Name);
            AutomationProperties.SetAutomationId(this, "file:" + entry.Path);
            var panel = new StackPanel();
            image = new Image { Width = size, Height = size, Stretch = Stretch.Uniform, IsHitTestVisible = false };
            panel.Children.Add(image);
            label = Glass.Label(entry.Name, 12); label.TextAlignment = TextAlignment.Center; label.Height = 34; label.TextTrimming = TextTrimming.CharacterEllipsis;
            label.Margin = new Thickness(0, 6, 0, 0); label.IsHitTestVisible = false;
            if (loose) { label.Foreground = Brushes.White; label.Effect = new System.Windows.Media.Effects.DropShadowEffect { BlurRadius = 3, ShadowDepth = 1, Opacity = 0.9 }; }
            panel.Children.Add(label); Child = panel; ToolTip = entry.Name;
        }
        internal void SetIcon(ImageSource source) { image.Source = source; }
        internal bool UpdateEntry(DesktopEntry entry, int size)
        {
            bool changed = Entry.Modified != entry.Modified || image.Width != size;
            Entry = entry; label.Text = entry.Name; ToolTip = entry.Name;
            image.Width = image.Height = size;
            if (changed) IconRevision++;
            return changed;
        }
        internal void Select(bool selected)
        { Background = selected ? Glass.Brush(75, 91, 166, 255) : Brushes.Transparent; BorderBrush = selected ? Glass.Brush(170, 155, 204, 255) : Brushes.Transparent; }
    }

    internal sealed class DesktopWindow : Window
    {
        internal readonly DesktopSession Session;
        internal DesktopMonitor Monitor;
        internal DesktopGroup Group;
        internal readonly bool Loose;
        internal readonly Canvas Icons = new Canvas { Background = Brushes.Transparent, VerticalAlignment = VerticalAlignment.Top, HorizontalAlignment = HorizontalAlignment.Stretch };
        internal readonly List<IconTile> Tiles = new List<IconTile>();
        private readonly Grid root = new Grid();
        private readonly ScrollViewer scroll;
        private readonly Rectangle marquee = new Rectangle { Stroke = Glass.Accent, Fill = Glass.Brush(48, 52, 130, 235), StrokeThickness = 1, Visibility = Visibility.Collapsed, IsHitTestVisible = false };
        private readonly Canvas overlay = new Canvas { IsHitTestVisible = false };
        private readonly TextBlock title;
        private readonly Button collapse;
        private Point pressedPoint, marqueePoint;
        private Point boxDragOffset;
        private Size boxDragSize;
        private double boxExpandedHeight;
        private IconTile pressed;
        private bool selecting;
        private HashSet<string> initialSelection;
        private HwndSource source;
        private NativeGlass frost;
        private DesktopToolbar toolbar;
        private Border frame;
        private TextBlock emptyHint;
        private readonly ShellDragImage dragImage = new ShellDragImage();
        internal IntPtr Handle { get { return new WindowInteropHelper(this).Handle; } }
        internal IntPtr ToolbarHandle { get { return toolbar == null ? IntPtr.Zero : new WindowInteropHelper(toolbar).Handle; } }
        internal DesktopWindow(DesktopSession session, DesktopMonitor monitor, DesktopGroup group, bool loose)
        {
            Session = session; Monitor = monitor; Group = group; Loose = loose;
            Title = loose ? "MAC UI Desktop" : "MAC UI · " + group.Title;
            WindowStyle = WindowStyle.None; ShowInTaskbar = false; ShowActivated = false;
            ResizeMode = ResizeMode.NoResize; FontFamily = new FontFamily("Segoe UI, Microsoft YaHei UI"); FontSize = 12;
            UseLayoutRounding = true; SnapsToDevicePixels = true; GlassControls.Apply(this);
            AllowsTransparency = true; Background = loose ? Glass.Brush(1, 0, 0, 0) : Brushes.Transparent;
            MinWidth = loose ? 0 : 180; MinHeight = loose ? 0 : 48;
            if (!loose)
            {
                root.Background = Brushes.Transparent;
                root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(42) }); root.RowDefinitions.Add(new RowDefinition());
                var header = new DockPanel { Margin = new Thickness(12, 4, 6, 2), Background = Brushes.Transparent };
                var menu = Glass.Button("", delegate { OpenGroupMenu(); }, true); menu.Content = Glass.MoreIcon();
                AutomationProperties.SetName(menu, Session.Model.T("boxes.edit")); DockPanel.SetDock(menu, Dock.Right); header.Children.Add(menu);
                collapse = Glass.Button("", ToggleCollapsed, true); collapse.Content = Glass.CollapseIcon(false); DockPanel.SetDock(collapse, Dock.Right); header.Children.Add(collapse);
                title = Glass.Label(group.Title, 12); title.FontWeight = FontWeights.SemiBold; header.Children.Add(title);
                header.MouseLeftButtonDown += delegate(object sender, MouseButtonEventArgs e) {
                    if (Hit<Button>(e.OriginalSource) != null || Session.Model.Locked) return;
                    if (e.ClickCount == 2) { ToggleCollapsed(); return; }
                    Win32.POINT cursor; Win32.RECT rect;
                    Win32.GetCursorPos(out cursor); Win32.GetWindowRect(Handle, out rect);
                    boxDragOffset = new Point((cursor.X - rect.Left) / Monitor.Scale, (cursor.Y - rect.Top) / Monitor.Scale);
                    boxDragSize = new Size((rect.Right - rect.Left) / Monitor.Scale, (rect.Bottom - rect.Top) / Monitor.Scale);
                    boxExpandedHeight = Session.Model.Size(Group.Id, Monitor, boxDragSize).Height;
                    Session.MovingBox = this;
                    try { DragMove(); }
                    finally { try { SaveGeometry(); } finally { Session.FinishBoxMove(); } }
                    e.Handled = true;
                };
                root.Children.Add(header);
                scroll = new ScrollViewer { VerticalScrollBarVisibility = ScrollBarVisibility.Auto, HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled, Content = Icons, Background = Brushes.Transparent, Margin = new Thickness(8, 0, 8, 10), CanContentScroll = false };
                scroll.Style = (Style)FindResource("GlassScrollViewer");
                scroll.ScrollChanged += delegate(object sender, ScrollChangedEventArgs e) {
                    if (e.ViewportHeightChange != 0) Icons.MinHeight = scroll.ViewportHeight;
                    if (e.ViewportWidthChange != 0) ArrangeIcons();
                };
                scroll.PreviewMouseWheel += delegate { var bar = (GlassScrollBar)scroll.Template.FindName("PART_VerticalScrollBar", scroll); bar.Reveal(); };
                Grid.SetRow(scroll, 1); root.Children.Add(scroll);
                foreach (string edge in new[] { "n", "s", "e", "w", "ne", "nw", "se", "sw" })
                {
                    string direction = edge;
                    bool horizontal = edge.Contains("e") || edge.Contains("w"), vertical = edge.Contains("n") || edge.Contains("s");
                    var resize = new Thumb { Width = horizontal ? 9 : Double.NaN, Height = vertical ? 9 : Double.NaN,
                        HorizontalAlignment = edge.Contains("w") ? HorizontalAlignment.Left : edge.Contains("e") ? HorizontalAlignment.Right : HorizontalAlignment.Stretch,
                        VerticalAlignment = edge.Contains("n") ? VerticalAlignment.Top : edge.Contains("s") ? VerticalAlignment.Bottom : VerticalAlignment.Stretch,
                        Cursor = !horizontal ? Cursors.SizeNS : !vertical ? Cursors.SizeWE : edge == "ne" || edge == "sw" ? Cursors.SizeNESW : Cursors.SizeNWSE,
                        Background = Brushes.Transparent, Focusable = true };
                    var template = new ControlTemplate(typeof(Thumb)); var surface = new FrameworkElementFactory(typeof(Border)); surface.SetValue(Border.BackgroundProperty, Brushes.Transparent); template.VisualTree = surface; resize.Template = template;
                    AutomationProperties.SetName(resize, Session.Model.T("resize_" + edge));
                    Grid.SetRowSpan(resize, 2); root.Children.Add(resize);
                    resize.DragDelta += delegate(object sender, DragDeltaEventArgs e) { ResizeBox(direction, e.HorizontalChange, e.VerticalChange); };
                    resize.DragCompleted += delegate { SaveGeometry(true); };
                    resize.KeyDown += delegate(object sender, KeyEventArgs e) {
                        double step = Json.Flag(Session.Model.Data, "snapToGrid", true) ? 20 : 4;
                        if ((Keyboard.Modifiers & ModifierKeys.Shift) != 0) step *= 4;
                        if (e.Key == Key.Left || e.Key == Key.Right || e.Key == Key.Up || e.Key == Key.Down)
                        { ResizeBox(direction, e.Key == Key.Left ? -step : e.Key == Key.Right ? step : 0, e.Key == Key.Up ? -step : e.Key == Key.Down ? step : 0); SaveGeometry(true); e.Handled = true; }
                    };
                }
            }
            else
            {
                root.Children.Add(Icons);
                Loaded += delegate { toolbar = new DesktopToolbar(this); toolbar.Show(); };
            }
            overlay.Children.Add(marquee); Grid.SetRowSpan(overlay, 2); root.Children.Add(overlay);
            frame = Loose ? null : GlassControls.Frame(root, Session.Contrast, Session.Model.GlassTransparency); Content = Loose ? (object)root : frame;
            AllowDrop = true;
            SourceInitialized += delegate {
                source = HwndSource.FromHwnd(Handle); source.AddHook(WindowMessage);
                long style = Win32.GetWindowLongPtr(Handle, -20).ToInt64(); Win32.SetWindowLongPtr(Handle, -20, new IntPtr((style | 0x80) & ~0x40000));
                if (!Loose)
                {
                    frost = new NativeGlass(this);
                    frost.SetBlur(Session.Model.GlassBlur);
                }
            };
            SizeChanged += delegate { ArrangeIcons(); };
            Activated += delegate { Session.Restack(); };
            Closed += delegate { dragImage.Dispose(); if (toolbar != null) toolbar.Close(); if (frost != null) frost.Dispose(); if (source != null) source.RemoveHook(WindowMessage); };
            PreviewMouseLeftButtonDown += Press;
            PreviewMouseMove += Move;
            PreviewMouseLeftButtonUp += Release;
            LostMouseCapture += delegate { if (selecting) EndMarquee(); };
            PreviewMouseRightButtonUp += RightClick;
            PreviewKeyDown += OnKey;
            DragEnter += delegate(object sender, DragEventArgs e) { OnDragOver(sender, e); dragImage.Enter(Handle, e.Data, e.Effects); };
            DragLeave += delegate { dragImage.Leave(); };
            DragOver += OnDragOver; Drop += OnDrop;
        }
        private IntPtr WindowMessage(IntPtr hwnd, int message, IntPtr wparam, IntPtr lparam, ref bool handled)
        {
            if (message == 0x46 && !Session.Restacking)
            { var position = (Win32.WINDOWPOS)Marshal.PtrToStructure(lparam, typeof(Win32.WINDOWPOS)); position.Flags |= 0x4; Marshal.StructureToPtr(position, lparam, false); }
            if (message == 0x216 && Session.MovingBox == this)
            {
                Win32.POINT cursor; Win32.GetCursorPos(out cursor);
                var pointer = new Point(cursor.X, cursor.Y);
                Monitor = Session.MonitorAt(pointer);
                var size = new Size(Math.Min(boxDragSize.Width, Monitor.Width), Math.Min(boxDragSize.Height, Monitor.Height));
                var location = Session.PlaceBox(this, DesktopLayout.DragPosition(pointer, boxDragOffset, Monitor), size, true);
                int left = Monitor.Work.Left + (int)Math.Round(location.X * Monitor.Scale), top = Monitor.Work.Top + (int)Math.Round(location.Y * Monitor.Scale);
                var rect = new Win32.RECT { Left = left, Top = top, Right = left + (int)Math.Round(size.Width * Monitor.Scale), Bottom = top + (int)Math.Round(size.Height * Monitor.Scale) };
                Marshal.StructureToPtr(rect, lparam, false); handled = true; return new IntPtr(1);
            }
            if (message == 0x2E0) Session.ScheduleDisplays();
            if (message == 0x10 && !Session.ClosingWindows && !Session.Stopping) { handled = true; Session.RequestStop(); }
            return IntPtr.Zero;
        }
        internal void Place(Point location, Size size)
        {
            Width = size.Width; Height = size.Height;
            if (Handle != IntPtr.Zero) Win32.SetWindowPos(Handle, IntPtr.Zero, Monitor.Work.Left + (int)Math.Round(location.X * Monitor.Scale), Monitor.Work.Top + (int)Math.Round(location.Y * Monitor.Scale),
                (int)Math.Round(size.Width * Monitor.Scale), (int)Math.Round(size.Height * Monitor.Scale), 0x4 | 0x10 | 0x200);
            if (Handle != IntPtr.Zero && !Win32.IsWindowVisible(Handle)) Win32.ShowWindowAsync(Handle, 4);
            if (toolbar != null) toolbar.Place();
        }
        private void SaveGeometry(bool resized = false)
        {
            if (Loose || Session.Model.Locked) return;
            Win32.RECT rect; Win32.GetWindowRect(Handle, out rect);
            if (Session.MovingBox == this) Monitor = Session.MonitorAt(new Point((rect.Left + rect.Right) / 2.0, (rect.Top + rect.Bottom) / 2.0));
            var location = new Point((rect.Left - Monitor.Work.Left) / Monitor.Scale, (rect.Top - Monitor.Work.Top) / Monitor.Scale);
            var size = new Size((rect.Right - rect.Left) / Monitor.Scale, (rect.Bottom - rect.Top) / Monitor.Scale);
            if (resized && Json.Flag(Session.Model.Data, "snapToGrid", true))
            { size.Width = Math.Max(180, Math.Round(size.Width / 20) * 20); size.Height = Math.Max(120, Math.Round(size.Height / 20) * 20); }
            location = Session.PlaceBox(this, location, size, true); Place(location, size);
            if (Json.Array(Session.Model.Data, "collapsed").Contains(Group.Id))
                size.Height = Session.MovingBox == this ? boxExpandedHeight : Session.Model.Size(Group.Id, Monitor, new Size(290, 330)).Height;
            Session.Model.SaveGeometry(Group.Id, Monitor.Id, location, size);
        }
        private void ResizeBox(string edge, double horizontal, double vertical)
        {
            if (Session.Model.Locked || Json.Array(Session.Model.Data, "collapsed").Contains(Group.Id)) return;
            Win32.RECT rect; Win32.GetWindowRect(Handle, out rect);
            var location = new Point((rect.Left - Monitor.Work.Left) / Monitor.Scale, (rect.Top - Monitor.Work.Top) / Monitor.Scale);
            double width = Width, height = Height;
            if (edge.Contains("w")) { width = Math.Max(MinWidth, Width - horizontal); location.X -= width - Width; }
            if (edge.Contains("e")) width = Math.Max(MinWidth, Width + horizontal);
            if (edge.Contains("n")) { height = Math.Max(120, Height - vertical); location.Y -= height - Height; }
            if (edge.Contains("s")) height = Math.Max(120, Height + vertical);
            Place(location, new Size(width, height));
        }
        internal void Update(DesktopGroup group)
        {
            string focused = Tiles.Where(tile => tile.IsKeyboardFocused).Select(tile => tile.Entry.Key).FirstOrDefault();
            Group = group; if (title != null) title.Text = group.Title + "  ·  " + group.Items.Count;
            if (!Loose)
            {
                bool folded = Json.Array(Session.Model.Data, "collapsed").Contains(Group.Id);
                collapse.Content = Glass.CollapseIcon(folded); collapse.ToolTip = Session.Model.T(folded ? "expand" : "collapse");
                AutomationProperties.SetName(collapse, (string)collapse.ToolTip);
                scroll.Visibility = folded ? Visibility.Collapsed : Visibility.Visible;
            }
            if (frame != null) { frame.Background = GlassControls.Fill(Session.Contrast, Session.Model.GlassTransparency); frost.SetBlur(Session.Model.GlassBlur); }
            if (Loose && Json.Flag(Session.Model.Data, "showGrid"))
            {
                var drawing = new GeometryDrawing(null, new Pen(Glass.Brush(45, 255, 255, 255), 1), Geometry.Parse("M 0,20 L 0,0 20,0"));
                Icons.Background = new DrawingBrush(drawing) { TileMode = TileMode.Tile, Viewport = new Rect(0, 0, 20, 20), ViewportUnits = BrushMappingMode.Absolute, Stretch = Stretch.None };
            }
            else Icons.Background = Brushes.Transparent;
            var existing = Tiles.ToDictionary(tile => tile.Entry.Key);
            var retained = new List<IconTile>();
            foreach (var entry in group.Items)
            {
                IconTile tile;
                if (existing.TryGetValue(entry.Key, out tile))
                { existing.Remove(entry.Key); if (tile.UpdateEntry(entry, Session.Model.IconSize)) Session.LoadIcon(tile); }
                else { tile = new IconTile(entry, Session.Model.IconSize, Loose); Icons.Children.Add(tile); Session.LoadIcon(tile); }
                retained.Add(tile);
            }
            foreach (var obsolete in existing.Values) Icons.Children.Remove(obsolete);
            Tiles.Clear(); Tiles.AddRange(retained);
            if (emptyHint != null) { Icons.Children.Remove(emptyHint); emptyHint = null; }
            if (group.Items.Count == 0 && !Loose)
            { emptyHint = Glass.Label(group.Error ?? Session.Model.T(group.Query ? "activity.empty" : "boxes.empty")); emptyHint.Margin = new Thickness(12); emptyHint.MaxWidth = Math.Max(100, Width - 60); Icons.Children.Add(emptyHint); }
            ArrangeIcons(); RefreshSelection();
            if (focused != null && IsActive)
            { var tile = Tiles.FirstOrDefault(item => item.Entry.Key == focused); if (tile != null) tile.Focus(); }
        }
        private void ArrangeIcons()
        {
            double size = Session.Model.IconSize, cellWidth = size + 30, cellHeight = size + 52;
            var spacing = Json.Map(Session.Model.Data, "iconSpacing"); double gapX = Json.Number(spacing, "horizontal", 8), gapY = Json.Number(spacing, "vertical", 10);
            double available = Loose ? ActualWidth : scroll.ViewportWidth;
            int columns = Math.Max(1, (int)((available + gapX) / (cellWidth + gapX))), rows = Math.Max(1, (int)((ActualHeight - 70) / (cellHeight + gapY)));
            double maxBottom = 60;
            foreach (var heading in Icons.Children.OfType<TextBlock>().Where(item => Equals(item.Tag, "time-group")).ToArray()) Icons.Children.Remove(heading);
            string previousSection = null; int sectionIndex = 0; double sectionTop = 0;
            double cutoff = (DateTime.UtcNow.AddDays(-7) - new DateTime(1970, 1, 1)).TotalMilliseconds;
            var positions = Json.Map(Json.Map(Session.Model.Data, "looseIconPositions"), Group.Id);
            for (int index = 0; index < Tiles.Count; index++)
            {
                var tile = Tiles[index]; tile.Width = cellWidth; tile.Height = cellHeight;
                double x = (index % columns) * (cellWidth + gapX), y = (index / columns) * (cellHeight + gapY);
                if (!Loose && Session.Model.SortMode(Group.Id) == "time-grouped")
                {
                    string section = tile.Entry.Modified > cutoff ? "recent-week" : "older";
                    if (section != previousSection)
                    {
                        if (previousSection != null) sectionTop += Math.Ceiling(sectionIndex / (double)columns) * (cellHeight + gapY) + 6;
                        var heading = Glass.Label(Session.Model.T("time_groups." + section)); heading.FontWeight = FontWeights.SemiBold; heading.Tag = "time-group";
                        Icons.Children.Add(heading); Canvas.SetTop(heading, sectionTop); Canvas.SetLeft(heading, 7);
                        sectionTop += 28; sectionIndex = 0; previousSection = section;
                    }
                    x = (sectionIndex % columns) * (cellWidth + gapX); y = sectionTop + (sectionIndex / columns) * (cellHeight + gapY); sectionIndex++;
                }
                if (Loose)
                {
                    x = 12 + (index / rows) * (cellWidth + gapX); y = 58 + (index % rows) * (cellHeight + gapY);
                    object value;
                    if (positions.TryGetValue(tile.Entry.Key, out value))
                    {
                        var point = (Dictionary<string, object>)value; x = Json.Number(point, "x", x); y = Json.Number(point, "y", y);
                        if (Json.Text(point, "coordinateUnit") != "dip") { x /= Monitor.Scale; y = (y - (Monitor.Work.Top - Monitor.Bounds.Top)) / Monitor.Scale; }
                    }
                    x = Math.Max(0, Math.Min(ActualWidth - cellWidth, x)); y = Math.Max(0, Math.Min(ActualHeight - cellHeight, y));
                }
                Canvas.SetLeft(tile, x); Canvas.SetTop(tile, y); maxBottom = Math.Max(maxBottom, y + cellHeight + 8);
            }
            Icons.Height = Loose ? Math.Max(0, ActualHeight) : maxBottom;
        }
        internal void RefreshSelection() { foreach (var tile in Tiles) tile.Select(Session.Selected.Contains(tile.Entry.Key)); }
        private static T Hit<T>(object sourceObject) where T : DependencyObject
        {
            DependencyObject node = sourceObject as DependencyObject;
            while (node != null) { if (node is T) return (T)node; node = node is Visual || node is System.Windows.Media.Media3D.Visual3D ? VisualTreeHelper.GetParent(node) : LogicalTreeHelper.GetParent(node); }
            return null;
        }
        private void Press(object sender, MouseButtonEventArgs e)
        {
            if (Hit<Button>(e.OriginalSource) != null || Hit<Thumb>(e.OriginalSource) != null || Hit<ScrollBar>(e.OriginalSource) != null) return;
            pressed = Hit<IconTile>(e.OriginalSource); pressedPoint = e.GetPosition(Icons);
            if (pressed != null)
            {
                pressed.Focus(); bool control = (Keyboard.Modifiers & ModifierKeys.Control) != 0, shift = (Keyboard.Modifiers & ModifierKeys.Shift) != 0;
                Session.Select(this, pressed, control, shift);
                if (e.ClickCount == 2) { Session.Open(this, pressed.Entry); pressed = null; }
                e.Handled = true;
            }
            else if (Hit<Canvas>(e.OriginalSource) == Icons || e.OriginalSource == root || (scroll != null && Hit<ScrollViewer>(e.OriginalSource) == scroll))
            {
                Focus(); selecting = true; marqueePoint = e.GetPosition(overlay); initialSelection = new HashSet<string>(Session.Selected);
                if ((Keyboard.Modifiers & ModifierKeys.Control) == 0) { initialSelection.Clear(); Session.Selected.Clear(); Session.RefreshSelection(); }
                CaptureMouse(); e.Handled = true;
            }
        }
        private void Move(object sender, MouseEventArgs e)
        {
            if (e.LeftButton != MouseButtonState.Pressed) { pressed = null; if (selecting) EndMarquee(); return; }
            if (selecting)
            {
                var point = e.GetPosition(overlay); var area = new Rect(marqueePoint, point);
                Canvas.SetLeft(marquee, area.Left); Canvas.SetTop(marquee, area.Top); marquee.Width = area.Width; marquee.Height = area.Height; marquee.Visibility = Visibility.Visible;
                Session.Selected.Clear(); Session.Selected.UnionWith(initialSelection);
                foreach (var tile in Tiles)
                {
                    var location = tile.TranslatePoint(new Point(), overlay);
                    if (area.IntersectsWith(new Rect(location, new Size(tile.ActualWidth, tile.ActualHeight)))) Session.Selected.Add(tile.Entry.Key);
                }
                Session.RefreshSelection(); e.Handled = true; return;
            }
            if (pressed == null || Session.Dragging != null) return;
            var distance = e.GetPosition(Icons) - pressedPoint;
            if (Math.Abs(distance.X) < SystemParameters.MinimumHorizontalDragDistance && Math.Abs(distance.Y) < SystemParameters.MinimumVerticalDragDistance) return;
            var tilePressed = pressed; pressed = null;
            Session.StartDrag(this, tilePressed.Entry, new Point(pressedPoint.X - Canvas.GetLeft(tilePressed), pressedPoint.Y - Canvas.GetTop(tilePressed)));
            e.Handled = true;
        }
        private void Release(object sender, MouseButtonEventArgs e) { pressed = null; if (selecting) EndMarquee(); }
        private void EndMarquee() { selecting = false; marquee.Visibility = Visibility.Collapsed; if (IsMouseCaptured) ReleaseMouseCapture(); }
        private void RightClick(object sender, MouseButtonEventArgs e)
        {
            var tile = Hit<IconTile>(e.OriginalSource);
            if (tile != null) { if (!Session.Selected.Contains(tile.Entry.Key)) Session.Select(this, tile, false, false); Session.FileMenu(this, tile); }
            else OpenGroupMenu();
            e.Handled = true;
        }
        private void OpenGroupMenu() { Session.GroupMenu(this); }
        private void ToggleCollapsed()
        {
            var collapsed = Json.Array(Session.Model.Data, "collapsed");
            if (collapsed.Contains(Group.Id)) collapsed.Remove(Group.Id); else collapsed.Add(Group.Id);
            Session.Model.Data["collapsed"] = collapsed; Session.Model.Save();
        }
        private void OnKey(object sender, KeyEventArgs e)
        {
            if (Keyboard.FocusedElement is Thumb) return;
            bool control = (Keyboard.Modifiers & ModifierKeys.Control) != 0;
            if (e.Key == Key.Escape) { pressed = null; EndMarquee(); Session.Selected.Clear(); Session.RefreshSelection(); }
            else if (e.Key == Key.A && control) { Session.Selected.UnionWith(Group.Items.Select(i => i.Key)); Session.RefreshSelection(); }
            else if (e.Key == Key.C && control) Session.Copy(false);
            else if (e.Key == Key.X && control) Session.Copy(true);
            else if (e.Key == Key.V && control) Session.Paste(this);
            else if (e.Key == Key.Delete) Session.Delete(this, (Keyboard.Modifiers & ModifierKeys.Shift) != 0);
            else if (e.Key == Key.F2) Session.Rename(this);
            else if (e.Key == Key.F5) Session.Refresh();
            else if (e.Key == Key.Enter) { var selected = Group.Items.FirstOrDefault(i => Session.Selected.Contains(i.Key)); if (selected != null) Session.Open(this, selected); }
            else if (e.Key == Key.Apps || (e.Key == Key.F10 && (Keyboard.Modifiers & ModifierKeys.Shift) != 0))
            { var selected = Tiles.FirstOrDefault(i => Session.Selected.Contains(i.Entry.Key)); if (selected != null) Session.FileMenu(this, selected); else OpenGroupMenu(); }
            else if (e.Key == Key.Left || e.Key == Key.Right || e.Key == Key.Up || e.Key == Key.Down)
            {
                var current = Tiles.FirstOrDefault(tile => tile.IsKeyboardFocused);
                var next = current == null ? Tiles.FirstOrDefault() : Tiles.Where(tile => tile != current).Select(tile => new {
                    Tile = tile, X = Canvas.GetLeft(tile) - Canvas.GetLeft(current), Y = Canvas.GetTop(tile) - Canvas.GetTop(current)
                }).Where(point => e.Key == Key.Left ? point.X < -1 : e.Key == Key.Right ? point.X > 1 : e.Key == Key.Up ? point.Y < -1 : point.Y > 1)
                  .OrderBy(point => e.Key == Key.Left || e.Key == Key.Right ? Math.Abs(point.X) + 3 * Math.Abs(point.Y) : Math.Abs(point.Y) + 3 * Math.Abs(point.X))
                  .Select(point => point.Tile).FirstOrDefault();
                if (next != null) { Session.Select(this, next, control, (Keyboard.Modifiers & ModifierKeys.Shift) != 0); next.Focus(); next.BringIntoView(); }
            }
            else return;
            e.Handled = true;
        }
        private void OnDragOver(object sender, DragEventArgs e)
        {
            var target = Hit<IconTile>(e.OriginalSource);
            e.Effects = Session.DropEffect(this, target == null ? null : target.Entry, e); e.Handled = true;
            dragImage.Over(e.Effects);
            if (scroll != null)
            { double y = e.GetPosition(scroll).Y; if (y < 32) scroll.ScrollToVerticalOffset(scroll.VerticalOffset - 12); else if (y > scroll.ActualHeight - 32) scroll.ScrollToVerticalOffset(scroll.VerticalOffset + 12); }
        }
        private void OnDrop(object sender, DragEventArgs e)
        {
            var target = Hit<IconTile>(e.OriginalSource); var entry = target == null ? null : target.Entry;
            e.Effects = Session.DropEffect(this, entry, e); e.Handled = true;
            dragImage.Drop(e.Effects);
            if (e.Effects != DragDropEffects.None) Session.Drop(this, entry, e.Data, e.KeyStates, e.Effects, e.GetPosition(Icons));
        }
    }
}
