using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;

namespace MacUI.Desktop.Native
{
    internal static class EntryPoint
    {
        private static readonly object outputLock = new object();
        private static StreamWriter output;
        private static string logPath;
        private static void Write(string type, object payload)
        {
            lock (outputLock)
            {
                if (type == "error" && logPath != null) File.AppendAllText(logPath, DateTime.UtcNow.ToString("o") + " " + Json.Write(payload) + Environment.NewLine, Encoding.UTF8);
                if (output != null)
                {
                    try { output.WriteLine(Json.Write(new { type = type, payload = payload })); output.Flush(); }
                    catch (IOException) { output = null; }
                }
            }
        }
        private static string Value(string[] arguments, string name, string fallback = null)
        { int index = Array.IndexOf(arguments, name); return index < 0 ? fallback : index + 1 < arguments.Length ? arguments[index + 1] : throwArgument(name); }
        private static string throwArgument(string name) { throw new ArgumentException("Missing value: " + name); }
        [STAThread] private static int Main(string[] arguments)
        {
            try
            {
                if (arguments.Length > 0 && arguments[0] == "--watchdog") return DesktopRecovery.Watchdog(arguments[1], arguments[2]);
                if (!arguments.Contains("--desktop") && !arguments.Contains("--preview")) throw new ArgumentException("Use --desktop or --preview");
                bool preview = arguments.Contains("--preview");
                string directory = Path.GetFullPath(Value(arguments, "--data"));
                string root = Value(arguments, "--root");
                if (preview && root == null) throw new ArgumentException("Preview requires an isolated --root directory");
                Directory.CreateDirectory(directory); logPath = Path.Combine(directory, "native-desktop.log");
                output = new StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false));
                bool created;
                using (var mutex = new Mutex(true, @"Local\macui.desktop." + DesktopRecovery.SessionName + (preview ? ".preview" : ""), out created))
                {
                    if (!created) throw new InvalidOperationException("MAC UI desktop is already running");
                    var app = new Application { ShutdownMode = ShutdownMode.OnExplicitShutdown };
                    var text = new Localizer(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "desktop-locales.json"), Value(arguments, "--locale", "en"));
                    var model = new DesktopModel(new LayoutStore(directory), text);
                    using (var session = new DesktopSession(model, new ShellCatalog(root), preview))
                    using (var stop = new EventWaitHandle(false, EventResetMode.ManualReset, @"Local\macui.desktop.stop." + Process.GetCurrentProcess().Id))
                    {
                        var stopRegistration = ThreadPool.RegisterWaitForSingleObject(stop, delegate { app.Dispatcher.BeginInvoke(new Action(session.RequestStop)); }, null, Timeout.Infinite, true);
                        session.Contrast = arguments.Contains("--contrast"); session.Send += Write;
                        session.Toolbar = Json.Parse(Value(arguments, "--toolbar", "{}"));
                        app.DispatcherUnhandledException += delegate(object sender, System.Windows.Threading.DispatcherUnhandledExceptionEventArgs error) {
                            Write("error", new { error = error.Exception.ToString() });
                            error.Handled = true; app.Shutdown(1);
                        };
                        string parentId = Value(arguments, "--parent"); Process parent = null;
                        if (parentId != null)
                        {
                            parent = Process.GetProcessById(Int32.Parse(parentId)); parent.EnableRaisingEvents = true;
                            parent.Exited += delegate { app.Dispatcher.BeginInvoke(new Action(session.RequestStop)); };
                            if (parent.HasExited) return 0;
                            Task.Run(delegate {
                                try
                                {
                                    using (var input = new StreamReader(Console.OpenStandardInput(), Encoding.UTF8))
                                    {
                                        string line;
                                        while ((line = input.ReadLine()) != null)
                                        {
                                            var message = Json.Parse(line);
                                            app.Dispatcher.BeginInvoke(new Action(delegate { Receive(session, message); }));
                                        }
                                    }
                                    app.Dispatcher.BeginInvoke(new Action(session.RequestStop));
                                }
                                catch (IOException error) { Write("error", new { error = error.Message }); app.Dispatcher.BeginInvoke(new Action(session.RequestStop)); }
                            });
                        }
                        app.Dispatcher.BeginInvoke(new Action(async delegate {
                            try { await session.Start(); }
                            catch (Exception error) { session.Error(error); app.Shutdown(1); }
                        }));
                        string lifetime = Value(arguments, "--lifetime");
                        if (lifetime != null) Task.Delay(TimeSpan.FromSeconds(Int32.Parse(lifetime))).ContinueWith(delegate { app.Dispatcher.BeginInvoke(new Action(session.RequestStop)); });
                        try { return app.Run(); }
                        finally { stopRegistration.Unregister(null); if (parent != null) parent.Dispose(); }
                    }
                }
            }
            catch (Exception error)
            {
                Write("error", new { error = error.ToString() });
                using (var stderr = new StreamWriter(Console.OpenStandardError(), new UTF8Encoding(false))) stderr.WriteLine(error);
                return 1;
            }
        }
        private static void Receive(DesktopSession session, Dictionary<string, object> message)
        {
            string type = Json.Text(message, "type");
            if (type == "stop") { session.RequestStop(); return; }
            if (type == "settings")
            {
                session.Toolbar = Json.Map(message, "toolbar"); session.ScheduleDisplays();
                session.Model.Text.SetLanguage(Json.Text(message, "locale", "en")); session.Contrast = Json.Flag(message, "contrast"); session.Model.Notify(); return;
            }
            if (type == "refresh") { session.Refresh(); return; }
            if (type != "controls") throw new ArgumentException("Unknown native widget request: " + type);
            string id = Json.Text(message, "id");
            try
            {
                object patch;
                if (message.TryGetValue("patch", out patch) && patch != null) session.Model.Patch((Dictionary<string, object>)patch);
                Write("controls", new { id = id, controls = session.Model.Controls() });
            }
            catch (Exception error) { Write("controls", new { id = id, error = error.Message }); }
        }
    }
}
