using Microsoft.Win32;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;

namespace MacUI.Desktop.Native
{
    internal sealed class ShellTemplate
    {
        internal string Extension, Label, FileName;
        internal byte[] Data;
    }
    internal static class ShellTemplates
    {
        [DllImport("shlwapi", CharSet = CharSet.Unicode)]
        private static extern int SHLoadIndirectString(string source, StringBuilder output, uint count, IntPtr reserved);
        internal static List<ShellTemplate> Read()
        {
            var templates = new List<ShellTemplate>();
            foreach (string extension in Registry.ClassesRoot.GetSubKeyNames().Where(name => name.StartsWith(".", StringComparison.Ordinal)))
            using (var association = Registry.ClassesRoot.OpenSubKey(extension))
            {
                if (association == null) continue;
                string program = association.GetValue("") as string;
                using (var key = association.OpenSubKey("ShellNew") ?? (String.IsNullOrEmpty(program) ? null : association.OpenSubKey(program + "\\ShellNew")))
                {
                    if (key == null || key.GetValue("Command") != null || key.GetValue("Handler") != null || key.GetValue("Directory") != null) continue;
                    using (var config = key.OpenSubKey("Config")) if (config != null && config.GetValue("NoExtension") != null) continue;
                    var template = new ShellTemplate { Extension = extension };
                    if (key.GetValueNames().Contains("NullFile", StringComparer.OrdinalIgnoreCase)) template.Data = new byte[0];
                    else if (key.GetValue("FileName") is string) template.FileName = FindTemplate((string)key.GetValue("FileName"));
                    else template.Data = key.GetValue("Data") as byte[];
                    if (template.FileName == null && template.Data == null) continue;
                    template.Label = key.GetValue("ItemName") as string;
                    if (String.IsNullOrWhiteSpace(template.Label) && !String.IsNullOrEmpty(program))
                    using (var type = Registry.ClassesRoot.OpenSubKey(program))
                        if (type != null) template.Label = type.GetValue("FriendlyTypeName") as string ?? type.GetValue("") as string;
                    if (!String.IsNullOrEmpty(template.Label) && template.Label.StartsWith("@", StringComparison.Ordinal))
                    {
                        var resolved = new StringBuilder(1024);
                        template.Label = SHLoadIndirectString(template.Label, resolved, (uint)resolved.Capacity, IntPtr.Zero) >= 0 ? resolved.ToString() : null;
                    }
                    if (String.IsNullOrWhiteSpace(template.Label)) template.Label = extension;
                    templates.Add(template);
                }
            }
            return templates.OrderBy(item => item.Label, StringComparer.CurrentCultureIgnoreCase).ToList();
        }
        private static string FindTemplate(string name)
        {
            string path = Environment.ExpandEnvironmentVariables(name);
            if (Path.IsPathRooted(path)) return File.Exists(path) ? path : null;
            foreach (string root in new[] { Environment.GetFolderPath(Environment.SpecialFolder.Templates), Environment.GetFolderPath(Environment.SpecialFolder.CommonTemplates), Environment.GetFolderPath(Environment.SpecialFolder.Windows) })
                foreach (string folder in new[] { root, Path.Combine(root, "ShellNew") })
                { string candidate = Path.Combine(folder, path); if (File.Exists(candidate)) return candidate; }
            return null;
        }
    }
}
