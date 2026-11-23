using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Interop;
using MacUI.Desktop.Native;

internal static class VisualTests
{
    internal static int Run(string locales)
    {
        string root = Path.Combine(Path.GetTempPath(), "macui-visual-tests-" + Guid.NewGuid().ToString("N"));
        string files = Path.Combine(root, "files"); Directory.CreateDirectory(files);
        for (int i = 0; i < 40; i++) File.WriteAllText(Path.Combine(files, "Item " + i.ToString("00") + ".txt"), "native UI test");
        var app = new Application { ShutdownMode = ShutdownMode.OnExplicitShutdown };
        int code = 0;
        app.DispatcherUnhandledException += delegate(object sender, System.Windows.Threading.DispatcherUnhandledExceptionEventArgs e) {
            Console.Error.WriteLine(e.Exception); e.Handled = true; code = 1; app.Shutdown();
        };
        app.Dispatcher.BeginInvoke(new Action(async delegate {
            try
            {
                var model = new DesktopModel(new LayoutStore(Path.Combine(root, "config")), new Localizer(locales, "zh-CN"));
                model.Data["arrangeMode"] = "category";
                model.Data["iconSize"] = 48;
                using (var session = new DesktopSession(model, new ShellCatalog(files), true))
                {
                    await session.Start();
                    var window = app.Windows.OfType<DesktopWindow>().First(w => !w.Loose);
                    window.UpdateLayout(); await Task.Delay(100);
                    window.Place(new Point(200, 100), new Size(280, 330));
                    window.UpdateLayout(); await Task.Delay(100);
                    if (Canvas.GetTop(window.Tiles[0]) != Canvas.GetTop(window.Tiles[2])) throw new Exception("A 280 DIP organizer must fit three 48 DIP icons without scrollbar reservation");
                    var desktopWindows = app.Windows.Cast<Window>().Where(w => w is DesktopWindow || w is DesktopToolbar).ToArray();
                    var ordinary = new Window { Title = "MAC UI window interaction test", Width = 440, Height = 300, Content = new Button { Content = "Window interaction" } };
                    try
                    {
                        ordinary.Show(); await Task.Delay(100);
                        foreach (var desktop in desktopWindows)
                        {
                            long style = Win32.GetWindowLongPtr(new WindowInteropHelper(desktop).Handle, -20).ToInt64();
                            if (desktop.ShowInTaskbar || (style & 0x80) == 0 || (style & 0x40000) != 0) throw new Exception("Desktop surfaces must stay out of taskbar and Alt+Tab");
                            desktop.Activate(); await Task.Delay(60);
                            var order = new List<IntPtr>();
                            for (IntPtr hwnd = Win32.GetTopWindow(IntPtr.Zero); hwnd != IntPtr.Zero; hwnd = Win32.GetWindow(hwnd, 2)) order.Add(hwnd);
                            int appIndex = order.IndexOf(new WindowInteropHelper(ordinary).Handle);
                            if (desktopWindows.Any(w => order.IndexOf(new WindowInteropHelper(w).Handle) <= appIndex)) throw new Exception("Desktop surfaces must remain below ordinary application windows after activation");
                        }
                    }
                    finally { ordinary.Close(); }
                    string settingsMessage = null;
                    session.Send += delegate(string type, object payload) { settingsMessage = type; };
                    int windowCount = app.Windows.Count;
                    session.OpenSettings();
                    if (settingsMessage != "open-settings" || app.Windows.Count != windowCount) throw new Exception("Desktop settings must open the main settings page through the host");
                    var original = window.Tiles.ToArray();
                    var dragData = new DataObject();
                    dragData.SetData(DataFormats.FileDrop, new[] { original[0].Entry.Path });
                    dragData.SetData(DesktopSession.DragFormat, "test-selection", false);
                    var nativeDrag = ShellDragImage.CreateDataObject(dragData);
                    try
                    {
                        ShellDragImage.Attach(nativeDrag, original[0], new Point(20, 20), window.Monitor.Scale);
                        var wrapped = new DataObject(nativeDrag);
                        if (!wrapped.GetDataPresent("DragImageBits")) throw new Exception("Windows must receive the native drag image format");
                        if (((string[])wrapped.GetData(DataFormats.FileDrop))[0] != original[0].Entry.Path) throw new Exception("The drag image must preserve the native file-drop payload");
                        if (!Equals(wrapped.GetData(DesktopSession.DragFormat, false), "test-selection")) throw new Exception("Shell drag data must preserve the internal selection token");
                        using (var target = new ShellDragImage()) { target.Enter(window.Handle, wrapped, DragDropEffects.Move); target.Over(DragDropEffects.Copy); target.Leave(); }
                    }
                    finally { Win32.Release(nativeDrag); }
                    model.Patch(new Dictionary<string, object> { { "glassTransparency", 96 }, { "glassBlur", 75 } });
                    if (!original.SequenceEqual(window.Tiles)) throw new Exception("Material changes must preserve all icon controls");
                    var scroll = Descendants(window).OfType<ScrollViewer>().First();
                    scroll.RaiseEvent(new System.Windows.Input.MouseWheelEventArgs(System.Windows.Input.Mouse.PrimaryDevice, Environment.TickCount, -120) { RoutedEvent = System.Windows.Input.Mouse.MouseWheelEvent });
                    window.UpdateLayout(); await Task.Delay(60);
                    if (scroll.VerticalOffset <= 0) throw new Exception("Native mouse wheel input must scroll the organizer");
                    scroll.ScrollToVerticalOffset(180); window.UpdateLayout(); await Task.Delay(60);
                    var bar = Descendants(scroll).OfType<GlassScrollBar>().First();
                    if (scroll.VerticalOffset < 100 || bar.Opacity < 0.9) throw new Exception("Scroll activity must reveal the glass thumb");
                    await Task.Delay(1500);
                    if (bar.Opacity > 0.01) throw new Exception("The glass thumb must disappear after inactivity");
                    for (int cycle = 0; cycle < 4; cycle++)
                    {
                        var menu = Glass.Menu(session); var parent = new MenuItem { Header = "Submenu" };
                        parent.Items.Add(Glass.Item("Item", delegate { })); menu.Items.Add(parent);
                        menu.PlacementTarget = window; menu.IsOpen = true; await Task.Delay(80);
                        parent.IsSubmenuOpen = true; await Task.Delay(80);
                        parent.IsSubmenuOpen = false; await Task.Delay(80);
                        menu.IsOpen = false; await Task.Delay(250);
                    }
                    if (!original.SequenceEqual(window.Tiles)) throw new Exception("Menus must preserve desktop icon instances");
                    window.Update(new DesktopGroup { Id = window.Group.Id, Title = window.Group.Title, Kind = "manual", Items = window.Group.Items.Take(3).ToList() });
                    window.Place(new Point(200, 100), new Size(280, 600));
                    window.UpdateLayout(); await Task.Delay(60);
                    window.Activate();
                    var blank = window.InputHitTest(new Point(80, window.ActualHeight - 60)) as UIElement;
                    if (blank != window.Icons) throw new Exception("The icon canvas must cover blank space below its final row");
                    var press = new System.Windows.Input.MouseButtonEventArgs(System.Windows.Input.Mouse.PrimaryDevice, Environment.TickCount, System.Windows.Input.MouseButton.Left) { RoutedEvent = UIElement.PreviewMouseLeftButtonDownEvent, Source = blank };
                    window.RaiseEvent(press);
                    if (!press.Handled) throw new Exception("Blank space below the final icon row must enter the selection handler");
                    window.RaiseEvent(new System.Windows.Input.MouseButtonEventArgs(System.Windows.Input.Mouse.PrimaryDevice, Environment.TickCount, System.Windows.Input.MouseButton.Left) { RoutedEvent = UIElement.PreviewMouseLeftButtonUpEvent });
                    if (window.IsMouseCaptured) throw new Exception("Completing marquee selection must release mouse capture");
                    Console.WriteLine("PASS: native desktop layering, taskbar visibility, settings navigation, full-width icons, material updates, scrollbar activity, menu lifecycle and blank-body selection input");
                }
            }
            catch (Exception error) { Console.Error.WriteLine(error); code = 1; }
            finally { app.Shutdown(); }
        }));
        app.Run(); return code;
    }
    private static IEnumerable<DependencyObject> Descendants(DependencyObject element)
    {
        for (int index = 0; index < VisualTreeHelper.GetChildrenCount(element); index++)
        {
            var child = VisualTreeHelper.GetChild(element, index); yield return child;
            foreach (var nested in Descendants(child)) yield return nested;
        }
    }
}
