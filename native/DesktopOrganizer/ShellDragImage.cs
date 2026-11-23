using System;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using ComDataObject = System.Runtime.InteropServices.ComTypes.IDataObject;

namespace MacUI.Desktop.Native
{
    [StructLayout(LayoutKind.Sequential)]
    internal struct ShellDragBitmap
    { internal Win32.SIZE Size; internal Win32.POINT Offset; internal IntPtr Bitmap; internal uint ColorKey; }

    [ComImport, Guid("DE5BF786-477A-11D2-839D-00C04FD918D0"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IDragSourceHelper
    {
        void InitializeFromBitmap(ref ShellDragBitmap image, ComDataObject data);
        void InitializeFromWindow(IntPtr hwnd, ref Win32.POINT point, ComDataObject data);
    }
    [ComImport, Guid("4657278B-411B-11D2-839A-00C04FD918D0"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    internal interface IDropTargetHelper
    {
        void DragEnter(IntPtr hwnd, ComDataObject data, ref Win32.POINT point, uint effect);
        void DragLeave();
        void DragOver(ref Win32.POINT point, uint effect);
        void Drop(ComDataObject data, ref Win32.POINT point, uint effect);
        void Show([MarshalAs(UnmanagedType.Bool)] bool show);
    }
    internal sealed class ShellDragImage : IDisposable
    {
        [StructLayout(LayoutKind.Sequential)] private struct BitmapHeader
        { internal uint Size; internal int Width, Height; internal ushort Planes, BitCount; internal uint Compression, SizeImage; internal int XPels, YPels; internal uint ClrUsed, ClrImportant; }
        [DllImport("gdi32", SetLastError = true)] private static extern IntPtr CreateDIBSection(IntPtr dc, ref BitmapHeader header, uint colors, out IntPtr pixels, IntPtr section, uint offset);
        [DllImport("shell32")] private static extern int SHCreateDataObject(IntPtr parent, uint count, IntPtr children, ComDataObject inner, ref Guid iid, out ComDataObject data);
        [DllImport("shell32")] private static extern int SHDoDragDrop(IntPtr hwnd, ComDataObject data, IntPtr source, uint allowed, out uint effect);
        private static readonly Guid HelperClass = new Guid("4657278A-411B-11D2-839A-00C04FD918D0");
        private IDropTargetHelper target;
        private ComDataObject targetData;
        private bool entered;

        internal static ComDataObject CreateDataObject(IDataObject inner)
        {
            var iid = typeof(ComDataObject).GUID; ComDataObject data;
            Win32.Check(SHCreateDataObject(IntPtr.Zero, 0, IntPtr.Zero, (ComDataObject)inner, ref iid, out data));
            return data;
        }
        internal static void Start(IntPtr hwnd, DataObject data, FrameworkElement tile, Point offset, double scale)
        {
            var native = CreateDataObject(data);
            try
            {
                Attach(native, tile, offset, scale);
                uint effect; Win32.Check(SHDoDragDrop(hwnd, native, IntPtr.Zero, (uint)(DragDropEffects.Copy | DragDropEffects.Move), out effect));
            }
            finally { Win32.Release(native); }
        }
        internal static void Attach(ComDataObject data, FrameworkElement tile, Point offset, double scale)
        {
            int width = (int)Math.Ceiling(tile.ActualWidth * scale), height = (int)Math.Ceiling(tile.ActualHeight * scale);
            var rendering = new RenderTargetBitmap(width, height, 96 * scale, 96 * scale, PixelFormats.Pbgra32);
            var visual = new DrawingVisual();
            using (var drawing = visual.RenderOpen()) drawing.DrawRectangle(new VisualBrush(tile), null, new Rect(0, 0, tile.ActualWidth, tile.ActualHeight));
            rendering.Render(visual);
            var bitmap = new FormatConvertedBitmap(rendering, PixelFormats.Bgra32, null, 0);
            var pixels = new byte[width * height * 4]; bitmap.CopyPixels(pixels, width * 4, 0);
            var header = new BitmapHeader { Size = 40, Width = width, Height = -height, Planes = 1, BitCount = 32, SizeImage = (uint)pixels.Length };
            IntPtr bits; IntPtr handle = CreateDIBSection(IntPtr.Zero, ref header, 0, out bits, IntPtr.Zero, 0);
            if (handle == IntPtr.Zero) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            IDragSourceHelper helper = null;
            try
            {
                helper = (IDragSourceHelper)Activator.CreateInstance(Type.GetTypeFromCLSID(HelperClass));
                Marshal.Copy(pixels, 0, bits, pixels.Length);
                var image = new ShellDragBitmap { Size = new Win32.SIZE { X = width, Y = height },
                    Offset = new Win32.POINT { X = (int)Math.Round(offset.X * scale), Y = (int)Math.Round(offset.Y * scale) }, Bitmap = handle, ColorKey = 0xFFFFFFFF };
                helper.InitializeFromBitmap(ref image, data);
                handle = IntPtr.Zero;
            }
            finally { if (handle != IntPtr.Zero) Win32.DeleteObject(handle); Win32.Release(helper); }
        }

        internal void Enter(IntPtr hwnd, IDataObject data, DragDropEffects effect)
        {
            if (target == null) target = (IDropTargetHelper)Activator.CreateInstance(Type.GetTypeFromCLSID(HelperClass));
            Leave(); targetData = CreateDataObject(data);
            Win32.POINT point; Win32.GetCursorPos(out point);
            target.DragEnter(hwnd, targetData, ref point, (uint)effect); entered = true;
        }
        internal void Over(DragDropEffects effect)
        {
            if (!entered) return;
            Win32.POINT point; Win32.GetCursorPos(out point); target.DragOver(ref point, (uint)effect);
        }
        internal void Leave()
        {
            try { if (entered) target.DragLeave(); }
            finally { entered = false; Win32.Release(targetData); targetData = null; }
        }
        internal void Drop(DragDropEffects effect)
        {
            if (!entered) return;
            try { Win32.POINT point; Win32.GetCursorPos(out point); target.Drop(targetData, ref point, (uint)effect); }
            finally { entered = false; Win32.Release(targetData); targetData = null; }
        }
        public void Dispose() { Leave(); Win32.Release(target); target = null; }
    }
}
