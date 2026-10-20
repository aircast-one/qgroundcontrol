using Microsoft.UI.Xaml;

namespace AircastQGC.Pages;

internal static class PageRegistry
{
    internal static UIElement? Create(string page) => page switch
    {
        Controls.NavPane.Plan => new Plan.PlanPage(),
        Controls.NavPane.Analyze => new Analyze.AnalyzePage(),
        Controls.NavPane.Settings => new Settings.SettingsPage(),
        Controls.NavPane.VehicleSetup => new Setup.SetupPage(),
        _ => null,
    };
}
