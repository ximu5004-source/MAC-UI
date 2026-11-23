using System;
using System.Collections.Generic;
using System.Numerics;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;
using Windows.Graphics.Effects;
using Windows.UI.Composition;
using Windows.UI.Composition.Desktop;

namespace MacUI.Desktop.Native
{
    [ComImport, Guid("29E691FA-4567-4DCA-B319-D0F207EB6807"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface ICompositorDesktopInterop
    {
        DesktopWindowTarget CreateDesktopWindowTarget(IntPtr hwnd, [MarshalAs(UnmanagedType.Bool)] bool topmost);
    }
    [ComImport, Guid("2FC57384-A068-44D7-A331-30982FCF7177"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IGraphicsEffectD2D1Interop
    {
        void GetEffectId(out Guid effect);
        [PreserveSig] int GetNamedPropertyMapping([MarshalAs(UnmanagedType.LPWStr)] string name, out uint index, out uint mapping);
        void GetPropertyCount(out uint count);
        void GetProperty(uint index, out IntPtr value);
        void GetSource(uint index, out IntPtr source);
        void GetSourceCount(out uint count);
    }
    [ComVisible(true), ClassInterface(ClassInterfaceType.None)]
    public sealed class BackdropBlur : IGraphicsEffect, IGraphicsEffectD2D1Interop
    {
        internal IGraphicsEffectSource Source;
        internal float Amount;
        public string Name { get; set; }
        public void GetEffectId(out Guid effect) { effect = new Guid("1FEB6D69-2FE6-4AC9-8C58-1D7F93E7A6A5"); }
        public int GetNamedPropertyMapping(string name, out uint index, out uint mapping) { index = 0; mapping = 1; return name == "BlurAmount" ? 0 : unchecked((int)0x80070057); }
        public void GetPropertyCount(out uint count) { count = 3; }
        public void GetProperty(uint index, out IntPtr value)
        {
            IntPtr inspectable;
            if (index == 0) PropertyFactory.Instance.CreateSingle(Amount, out inspectable);
            else if (index < 3) PropertyFactory.Instance.CreateUInt32(1, out inspectable);
            else throw new ArgumentOutOfRangeException("index");
            var propertyId = new Guid("4BD682DD-7554-40E9-9A9B-82654EDE7E62");
            try { Win32.Check(Marshal.QueryInterface(inspectable, ref propertyId, out value)); }
            finally { Marshal.Release(inspectable); }
        }
        public void GetSource(uint index, out IntPtr source) { if (index != 0) throw new ArgumentOutOfRangeException("index"); source = Marshal.GetComInterfaceForObject(Source, typeof(IGraphicsEffectSource)); }
        public void GetSourceCount(out uint count) { count = 1; }
    }

    [ComImport, Guid("629BDBC8-D932-4FF4-96B9-8D96C5C1E858"), InterfaceType(ComInterfaceType.InterfaceIsIInspectable)]
    internal interface IPropertyFactory
    {
        void CreateEmpty(out IntPtr value);
        void CreateUInt8(byte data, out IntPtr value);
        void CreateInt16(short data, out IntPtr value);
        void CreateUInt16(ushort data, out IntPtr value);
        void CreateInt32(int data, out IntPtr value);
        void CreateUInt32(uint data, out IntPtr value);
        void CreateInt64(long data, out IntPtr value);
        void CreateUInt64(ulong data, out IntPtr value);
        void CreateSingle(float data, out IntPtr value);
    }
    internal static class PropertyFactory
    {
        internal static readonly IPropertyFactory Instance = (IPropertyFactory)System.Runtime.InteropServices.WindowsRuntime.WindowsRuntimeMarshal.GetActivationFactory(typeof(Windows.Foundation.PropertyValue));
    }

    internal sealed class NativeGlass : IDisposable
    {
        [StructLayout(LayoutKind.Sequential)] private struct QueueOptions { internal int Size, Thread, Apartment; }
        [DllImport("CoreMessaging.dll")] private static extern int CreateDispatcherQueueController(QueueOptions options, out IntPtr controller);
        [DllImport("gdi32", SetLastError = true)] private static extern IntPtr CreateRoundRectRgn(int left, int top, int right, int bottom, int ellipseWidth, int ellipseHeight);
        [DllImport("user32", SetLastError = true)] private static extern int SetWindowRgn(IntPtr hwnd, IntPtr region, bool redraw);
        [DllImport("user32")] private static extern uint GetDpiForWindow(IntPtr hwnd);
        [ThreadStatic] private static object queue;
        private static readonly HashSet<IntPtr> Hosts = new HashSet<IntPtr>();
        internal static bool IsHost(IntPtr hwnd) { return Hosts.Contains(hwnd); }
        private readonly FrameworkElement view;
        private readonly HwndSource host, content;
        private readonly IntPtr foreground;
        private readonly Compositor compositor;
        private readonly DesktopWindowTarget target;
        private readonly SpriteVisual surface;
        private readonly CompositionEffectBrush brush;
        private readonly CompositionRoundedRectangleGeometry shape;
        private readonly CompositionGeometricClip clip;
        private readonly CompositionBackdropBrush backdrop;
        private readonly CompositionEffectFactory factory;
        private Vector2 clippedSize;
        private double clippedScale;
        private bool disposed;
        internal NativeGlass(Window window) : this(window, HwndSource.FromHwnd(new WindowInteropHelper(window).Handle)) { }
        internal NativeGlass(FrameworkElement view, HwndSource source)
        {
            this.view = view;
            if (queue == null && Windows.System.DispatcherQueue.GetForCurrentThread() == null)
            {
                IntPtr controller;
                Win32.Check(CreateDispatcherQueueController(new QueueOptions { Size = 12, Thread = 2, Apartment = 0 }, out controller));
                try { queue = Marshal.GetObjectForIUnknown(controller); } finally { Marshal.Release(controller); }
            }
            foreground = source.Handle;
            content = source;
            host = new HwndSource(new HwndSourceParameters("MAC UI Glass") { Width = 1, Height = 1,
                WindowStyle = unchecked((int)0x88000000), ExtendedWindowStyle = 0x082000A0 });
            var hwnd = host.Handle;
            Hosts.Add(hwnd);
            host.CompositionTarget.BackgroundColor = Colors.Transparent;
            var margins = new Win32.MARGINS { Left = -1, Top = -1, Right = -1, Bottom = -1 };
            Win32.Check(Win32.DwmExtendFrameIntoClientArea(hwnd, ref margins));
            int enabled = 1, corner = 1, nonClient = 1;
            Win32.Check(Win32.DwmSetWindowAttribute(hwnd, 17, ref enabled, 4));
            Win32.Check(Win32.DwmSetWindowAttribute(hwnd, 2, ref nonClient, 4));
            Win32.Check(Win32.DwmSetWindowAttribute(hwnd, 33, ref corner, 4));
            compositor = new Compositor();
            factory = compositor.CreateEffectFactory(new BackdropBlur { Name = "Glass", Source = new CompositionEffectSourceParameter("Desktop"), Amount = 14 }, new[] { "Glass.BlurAmount" });
            brush = factory.CreateBrush(); backdrop = compositor.CreateHostBackdropBrush(); brush.SetSourceParameter("Desktop", backdrop);
            target = ((ICompositorDesktopInterop)(object)compositor).CreateDesktopWindowTarget(hwnd, false);
            surface = compositor.CreateSpriteVisual(); surface.Brush = brush;
            shape = compositor.CreateRoundedRectangleGeometry(); clip = compositor.CreateGeometricClip(shape); surface.Clip = clip;
            target.Root = surface;
            content.AddHook(WindowMessage);
            view.SizeChanged += Resized; view.IsVisibleChanged += VisibilityChanged; Update();
        }
        internal void SetBlur(int percentage) { brush.Properties.InsertScalar("Glass.BlurAmount", percentage * 0.4f); }
        private void Resized(object sender, SizeChangedEventArgs e) { Update(); }
        private void VisibilityChanged(object sender, DependencyPropertyChangedEventArgs e) { Update(); }
        private IntPtr WindowMessage(IntPtr hwnd, int message, IntPtr wparam, IntPtr lparam, ref bool handled)
        { if (message == 0x2) Dispose(); else if (message == 0x47) Update(); return IntPtr.Zero; }
        private void Update()
        {
            var scale = GetDpiForWindow(foreground) / 96.0;
            Win32.RECT rect; Win32.GetWindowRect(foreground, out rect);
            surface.Size = new Vector2(Math.Max(0, rect.Right - rect.Left), Math.Max(0, rect.Bottom - rect.Top));
            shape.Size = surface.Size; shape.CornerRadius = new Vector2((float)(22 * scale));
            if (clippedSize != surface.Size || clippedScale != scale)
            {
                clippedSize = surface.Size; clippedScale = scale;
                foreach (var hwnd in new[] { foreground, host.Handle })
                {
                    IntPtr region = CreateRoundRectRgn(0, 0, (int)surface.Size.X + 1, (int)surface.Size.Y + 1, (int)Math.Round(44 * scale), (int)Math.Round(44 * scale));
                    if (SetWindowRgn(hwnd, region, true) == 0) { Win32.DeleteObject(region); throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error()); }
                }
            }
            Win32.SetWindowPos(host.Handle, foreground, rect.Left, rect.Top, (int)surface.Size.X, (int)surface.Size.Y, 0x10);
            Win32.ShowWindowAsync(host.Handle, view.IsVisible ? 4 : 0);
        }
        public void Dispose()
        {
            if (disposed) return;
            disposed = true;
            view.SizeChanged -= Resized; view.IsVisibleChanged -= VisibilityChanged; content.RemoveHook(WindowMessage);
            target.Dispose(); surface.Dispose(); clip.Dispose(); shape.Dispose(); brush.Dispose(); factory.Dispose(); backdrop.Dispose(); compositor.Dispose();
            Hosts.Remove(host.Handle);
            host.Dispose();
        }
    }
}
