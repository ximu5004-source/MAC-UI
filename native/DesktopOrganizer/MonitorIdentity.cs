using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
using Windows.Devices.Display.Core;

namespace MacUI.Desktop.Native
{
    internal static class MonitorIdentity
    {
        [StructLayout(LayoutKind.Sequential)] private struct LUID { internal uint Low; internal int High; }
        [StructLayout(LayoutKind.Sequential)] private struct Source { internal LUID Adapter; internal uint Id, Mode, Status; }
        [StructLayout(LayoutKind.Sequential)] private struct Target
        { internal LUID Adapter; internal uint Id, Mode, Technology, Rotation, Scaling, RefreshNumerator, RefreshDenominator, ScanLine, Available, Status; }
        [StructLayout(LayoutKind.Sequential)] private struct DisplayPath { internal Source Source; internal Target Target; internal uint Flags; }
        [StructLayout(LayoutKind.Explicit, Size = 64)] private struct Mode { [FieldOffset(0)] internal uint Type; }
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] private struct SourceName
        { internal uint Type, Size; internal LUID Adapter; internal uint Id; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] internal string Name; }
        [DllImport("user32")] private static extern int GetDisplayConfigBufferSizes(uint flags, out uint paths, out uint modes);
        [DllImport("user32")] private static extern int QueryDisplayConfig(uint flags, ref uint pathCount, [Out] DisplayPath[] paths, ref uint modeCount, [Out] Mode[] modes, IntPtr topology);
        [DllImport("user32", CharSet = CharSet.Unicode)] private static extern int DisplayConfigGetDeviceInfo(ref SourceName name);
        internal static Dictionary<string, string> Read()
        {
            uint pathCount, modeCount; int result;
            DisplayPath[] paths;
            do
            {
                result = GetDisplayConfigBufferSizes(2, out pathCount, out modeCount);
                if (result != 0) throw new Win32Exception(result);
                paths = new DisplayPath[pathCount]; var modes = new Mode[modeCount];
                result = QueryDisplayConfig(2, ref pathCount, paths, ref modeCount, modes, IntPtr.Zero);
            } while (result == 122);
            if (result != 0) throw new Win32Exception(result);
            var names = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            using (var manager = DisplayManager.Create(DisplayManagerOptions.None))
            {
                foreach (var target in manager.GetCurrentTargets())
                {
                    var adapter = target.Adapter.Id;
                    for (int index = 0; index < pathCount; index++)
                    {
                        var path = paths[index];
                        if (path.Target.Adapter.Low != adapter.LowPart || path.Target.Adapter.High != adapter.HighPart || path.Target.Id != target.AdapterRelativeId) continue;
                        var name = new SourceName { Type = 1, Size = (uint)Marshal.SizeOf(typeof(SourceName)), Adapter = path.Source.Adapter, Id = path.Source.Id };
                        result = DisplayConfigGetDeviceInfo(ref name);
                        if (result != 0) throw new Win32Exception(result);
                        if (!names.ContainsKey(name.Name)) names[name.Name] = target.StableMonitorId;
                    }
                }
            }
            return names;
        }
    }
}
