using System;
using System.IO;
using AircastQGC.Core;
using Microsoft.UI.Xaml;

namespace AircastQGC;

public partial class App : Application
{
    private static readonly string CrashLog = Path.Combine(Path.GetTempPath(), "aircast-crash.log");
    private static readonly string SavePath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments), "Aircast QGC");

    private Window? _window;

    public App()
    {
        InitializeComponent();
        UnhandledException += (_, e) => Record(e.Exception);
    }

    private static void Record(Exception ex) =>
        File.AppendAllText(CrashLog, $"{DateTime.Now:o} {ex}\n\n");

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        try
        {
            CoreHost.Start("AircastQGC", "--no-autoconnect", "--save-path", SavePath);
            VideoDriver.Start();
            _window = new MainWindow();
            _window.Closed += (_, _) =>
            {
                VideoDriver.Stop();
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
}
