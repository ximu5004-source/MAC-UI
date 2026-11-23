using System;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;
using System.Windows.Media;

namespace MacUI.Desktop.Native
{
    internal sealed class DesktopToolbar : Window
    {
        private readonly DesktopWindow desktop;
        private NativeGlass glass;
        private Border frame;
        private HwndSource source;
        internal DesktopToolbar(DesktopWindow desktop)
        {
            this.desktop = desktop;
            Title = desktop.Session.Model.T("controls_label"); WindowStyle = WindowStyle.None;
            ShowInTaskbar = false; ShowActivated = false; ResizeMode = ResizeMode.NoResize;
            SizeToContent = SizeToContent.WidthAndHeight; AllowsTransparency = true; Background = Brushes.Transparent;
            FontFamily = desktop.FontFamily; FontSize = 12; UseLayoutRounding = true;
            var buttons = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(6) };
            var model = desktop.Session.Model;
            buttons.Children.Add(Glass.Button(model.T("organize.deskgo"), desktop.Session.Organize));
            buttons.Children.Add(Glass.Button(model.T("boxes.create"), delegate { desktop.Session.CreateBox(desktop); }));
            var more = Glass.Button("", delegate {
                var menu = Glass.Menu(desktop.Session);
                foreach (var command in desktop.Session.DesktopCommands(desktop))
                {
                    if (command.Title == null) { menu.Items.Add(new Separator()); continue; }
                    var item = Glass.Item(command.Title, command.Action, command.Enabled); item.IsChecked = command.Checked; menu.Items.Add(item);
                }
                menu.PlacementTarget = this; menu.IsOpen = true;
            });
            more.Content = Glass.MoreIcon();
            System.Windows.Automation.AutomationProperties.SetName(more, model.T("controls_label")); buttons.Children.Add(more);
            frame = GlassControls.Frame(buttons, desktop.Session.Contrast, model.GlassTransparency); Content = frame;
            SourceInitialized += delegate {
                source = HwndSource.FromHwnd(new WindowInteropHelper(this).Handle); source.AddHook(WindowMessage);
                long style = Win32.GetWindowLongPtr(source.Handle, -20).ToInt64(); Win32.SetWindowLongPtr(source.Handle, -20, new IntPtr((style | 0x80) & ~0x40000));
                glass = new NativeGlass(this); glass.SetBlur(model.GlassBlur); model.Changed += UpdateMaterial;
            };
            Loaded += delegate { Place(); };
            Activated += delegate { desktop.Session.Restack(); };
            Closed += delegate { model.Changed -= UpdateMaterial; if (glass != null) glass.Dispose(); if (source != null) source.RemoveHook(WindowMessage); };
        }
        private IntPtr WindowMessage(IntPtr hwnd, int message, IntPtr wparam, IntPtr lparam, ref bool handled)
        {
            if (message == 0x46 && !desktop.Session.Restacking)
            { var position = (Win32.WINDOWPOS)Marshal.PtrToStructure(lparam, typeof(Win32.WINDOWPOS)); position.Flags |= 0x4; Marshal.StructureToPtr(position, lparam, false); }
            return IntPtr.Zero;
        }
        private void UpdateMaterial()
        { frame.Background = GlassControls.Fill(desktop.Session.Contrast, desktop.Session.Model.GlassTransparency); glass.SetBlur(desktop.Session.Model.GlassBlur); }
        internal void Place()
        {
            var monitor = desktop.Monitor;
            Win32.SetWindowPos(new WindowInteropHelper(this).Handle, IntPtr.Zero,
                monitor.Work.Right - (int)Math.Ceiling((ActualWidth + 12) * monitor.Scale), monitor.Work.Top + (int)(12 * monitor.Scale), 0, 0, 0x1 | 0x4 | 0x10);
        }
    }
}
