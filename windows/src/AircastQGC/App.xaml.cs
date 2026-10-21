using System;
using System.IO;
using AircastQGC.Core;
using Microsoft.UI.Xaml;

namespace AircastQGC;

public partial class App : Application
{
    private static readonly string CrashLog = Path.Combine(Path.GetTempPath(), "aircast-crash.log");
    private static readonly string SavePath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments), "Aircast QGC");
    private static readonly string AppData = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Aircast");
    private static readonly string MapCache = Path.Combine(AppData, "QGCMapCache");

    private Window? _window;

    public App()
    {
        InitializeComponent();
        UnhandledException += (_, e) => Record(e.Exception);
    }

    private static void Record(Exception ex) =>
        File.AppendAllText(CrashLog, $"{DateTime.Now:o} {ex}\n\n");

    protected override async void OnLaunched(LaunchActivatedEventArgs args)
    {
        try
        {
            Elevation.AwaitPrevious(Environment.GetCommandLineArgs());
            var link = DeepLinks.In(Environment.CommandLine);
            if (await DeepLinks.Forwarded(link, Arrived))
            {
                Exit();
                return;
            }
            if (Environment.ProcessPath is { } exe) DeepLinks.Register(exe);
            Directory.CreateDirectory(MapCache);
            CoreHost.Start("AircastQGC", "--no-autoconnect", "--save-path", SavePath,
                "--settings", Path.Combine(AppData, "Aircast QGC.ini"), "--map-cache", Path.Combine(MapCache, "qgcMapCache.db"),
                "--position-source", GcsLocation.SourceToken);
            if (link is not null) DeepLinks.Handle(link);
            GcsLocation.Start();
            VideoDriver.Start();
            _window = new MainWindow();
            _window.Closed += (_, _) =>
            {
                VideoDriver.Stop();
                GcsLocation.Stop();
                CoreHost.Shutdown();
            };
            _window.Activate();
        }
        catch (Exception ex)
        {
            Record(ex);
            throw;
        }
    }

    private void Arrived(string link)
    {
        DeepLinks.Handle(link);
        if (_window is { } window) window.DispatcherQueue.TryEnqueue(() => window.Activate());
    }
}
