using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Web.Script.Serialization;

namespace MacUI.Desktop.Native
{
    internal static class Json
    {
        private static JavaScriptSerializer Serializer { get { return new JavaScriptSerializer { MaxJsonLength = 16 * 1024 * 1024 }; } }
        internal static Dictionary<string, object> Parse(string text) { return Serializer.Deserialize<Dictionary<string, object>>(text); }
        internal static string Write(object value) { return Serializer.Serialize(value); }
        internal static Dictionary<string, object> Map(Dictionary<string, object> parent, string key)
        {
            object value;
            if (!parent.TryGetValue(key, out value)) parent[key] = value = new Dictionary<string, object>();
            return (Dictionary<string, object>)value;
        }
        internal static string Text(Dictionary<string, object> source, string key, string defaultValue = "")
        { object value; return source.TryGetValue(key, out value) && value != null ? Convert.ToString(value) : defaultValue; }
        internal static double Number(Dictionary<string, object> source, string key, double defaultValue)
        { object value; return source.TryGetValue(key, out value) && value != null ? Convert.ToDouble(value) : defaultValue; }
        internal static bool Flag(Dictionary<string, object> source, string key, bool defaultValue = false)
        { object value; return source.TryGetValue(key, out value) && value != null ? Convert.ToBoolean(value) : defaultValue; }
        internal static List<object> Array(Dictionary<string, object> source, string key)
        {
            object value;
            return source.TryGetValue(key, out value) && value != null ? ((IEnumerable)value).Cast<object>().ToList() : new List<object>();
        }
        internal static string Key(string path) { return path.Replace('/', '\\').ToLowerInvariant(); }
        internal static void AtomicWrite(string path, string content, string backup)
        {
            Directory.CreateDirectory(Path.GetDirectoryName(path));
            string temporary = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
            try
            {
                using (var stream = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None))
                using (var writer = new StreamWriter(stream, new UTF8Encoding(false)))
                { writer.Write(content); writer.Flush(); stream.Flush(true); }
                if (File.Exists(path)) File.Replace(temporary, path, backup);
                else File.Move(temporary, path);
            }
            finally { if (File.Exists(temporary)) File.Delete(temporary); }
        }
    }

    internal sealed class LayoutStore
    {
        internal readonly string DirectoryPath;
        internal string FilePath { get { return Path.Combine(DirectoryPath, "desktop-shell.json"); } }
        internal Dictionary<string, object> Data { get; private set; }
        internal LayoutStore(string directory)
        {
            DirectoryPath = Path.GetFullPath(directory);
            Directory.CreateDirectory(DirectoryPath);
            if (File.Exists(FilePath))
            {
                string original = File.ReadAllText(FilePath, Encoding.UTF8);
                Data = Json.Parse(original);
                if (Data == null || Json.Number(Data, "schemaVersion", 1) > 3) throw new InvalidDataException("Unsupported desktop layout version");
                string migrationCopy = Path.Combine(directory, "desktop-shell.before-native-csharp.json");
                if (!File.Exists(migrationCopy)) File.WriteAllText(migrationCopy, original, new UTF8Encoding(false));
            }
            else Data = new Dictionary<string, object>();
            Initialize();
        }
        private void Initialize()
        {
            Data["schemaVersion"] = 3;
            foreach (var name in new[] { "positions", "sizes", "iconOrders", "sortModes", "sortDirections", "activity", "looseIconPositions" }) Json.Map(Data, name);
            var organization = Json.Map(Data, "organization");
            foreach (var name in new[] { "assignments", "monitors", "layouts" }) Json.Map(organization, name);
            if (!organization.ContainsKey("boxes")) organization["boxes"] = new object[0];
        }
        internal void Save() { Json.AtomicWrite(FilePath, Json.Write(Data), Path.Combine(DirectoryPath, "desktop-shell.backup.json")); }
        internal void Snapshot() { Json.AtomicWrite(Path.Combine(DirectoryPath, "desktop-shell.manual-backup.json"), Json.Write(Data), null); }
        internal void Restore(bool manual)
        {
            var restored = Json.Parse(File.ReadAllText(Path.Combine(DirectoryPath, manual ? "desktop-shell.manual-backup.json" : "desktop-shell.backup.json"), Encoding.UTF8));
            if (restored == null || Json.Number(restored, "schemaVersion", 1) > 3) throw new InvalidDataException("Unsupported desktop layout version");
            Data = restored;
            Initialize();
            Save();
        }
    }

    internal sealed class Localizer
    {
        private readonly Dictionary<string, object> languages;
        private Dictionary<string, object> current;
        private readonly Dictionary<string, object> english;
        internal Localizer(string resource, string locale)
        {
            languages = Json.Parse(File.ReadAllText(resource, Encoding.UTF8));
            english = (Dictionary<string, object>)languages["en"];
            SetLanguage(locale);
        }
        internal void SetLanguage(string locale)
        {
            object language;
            current = languages.TryGetValue(locale, out language) ? (Dictionary<string, object>)language : english;
        }
        internal string this[string key]
        {
            get { object value; return current.TryGetValue(key, out value) || english.TryGetValue(key, out value) ? Convert.ToString(value) : key; }
        }
    }
}
