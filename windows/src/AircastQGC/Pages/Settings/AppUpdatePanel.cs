using System.Collections.Generic;
using System.Linq;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Settings;

internal sealed class AppUpdatePanel : UserControl
{
    private readonly TextBlock _refusal = FormLook.Caption("", "CriticalBrush");
    private readonly Border _card = new();

    public AppUpdatePanel()
    {
        Margin = new Thickness(0, 24, 0, 0);
        Content = FormLook.Stack(0,
            FormLook.Heading("Updates"),
            _card,
            FormLook.Footnote(AppUpdates.Installed ? "" : AppUpdates.PortableNote),
            FormLook.Margined(_refusal, new Thickness(4, 8, 4, 0)));
        Live.Watch(this, AppUpdates.View, view => _card.Child = FormLook.Card(Rows(AppUpdate.Parse(view))));
    }

    private IEnumerable<UIElement> Rows(AppUpdate update)
    {
        yield return FormLook.Row(
            FormLook.Stack(2,
                FormLook.Text($"{AppUpdates.Application} {update.Current}"),
                FormLook.Caption($"{update.ChannelName} channel · {update.Checked}")),
            Check(update));
        yield return FormLook.Row(
            FormLook.Stack(2,
                FormLook.Text("Check for updates automatically"),
                FormLook.Caption("Looks for a new release shortly after start and every six hours.")),
            AutoCheck(update));
        if (update.Phase == AppUpdate.Failed)
            yield return FormLook.Row(FormLook.Stack(2, FormLook.Text("Update check failed"), FormLook.Caption(update.Error, "CriticalBrush")), null);
        if (update.Offered) yield return Offer(update);
        if (update.Offered && update.PlainNotes.Length > 0) yield return Notes(update);
    }

    private Button Check(AppUpdate update)
    {
        var button = FormLook.Button(update.Phase == AppUpdate.Checking ? "Checking…" : "Check now", () => Run("appUpdate.check"));
        button.IsEnabled = !update.Busy;
        return button;
    }

    private ToggleSwitch AutoCheck(AppUpdate update)
    {
        var toggle = new ToggleSwitch { IsOn = update.AutoCheck, OnContent = "", OffContent = "", MinWidth = 0 };
        AutomationProperties.SetName(toggle, "Check for updates automatically");
        toggle.Toggled += (_, _) =>
        {
            if (toggle.IsOn != update.AutoCheck) Run("appUpdate.setAutoCheck", toggle.IsOn);
        };
        return toggle;
    }

    private Grid Offer(AppUpdate update)
    {
        var blocked = update.Phase == AppUpdate.Ready && !update.CanInstall;
        var detail = blocked ? AppUpdates.DisconnectFirst
            : update.WasSkipped ? string.Join(" · ", new[] { "Skipped", update.Size }.Where(part => part.Length > 0))
            : update.Size;
        var left = FormLook.Stack(2, FormLook.Text(update.Title), FormLook.Caption(detail, blocked ? "CautionBrush" : "TextSecondaryBrush"));
        var below = update.Phase == AppUpdate.Downloading
            ? new ProgressBar { Minimum = 0, Maximum = 1, Value = update.Progress, HorizontalAlignment = HorizontalAlignment.Stretch }
            : null;
        return FormLook.Row(left, FormLook.Line(8, update.CanSkip ? FormLook.Button("Skip this version", () => Run("appUpdate.skip")) : null, Action(update)), below);
    }

    private Button Action(AppUpdate update)
    {
        if (!AppUpdates.Installed) return FormLook.Button("Download", () => Open(update.ReleaseUrl), ButtonLook.Accent);
        var button = update.Phase switch
        {
            AppUpdate.Downloading => FormLook.Button("Cancel", () => Run("appUpdate.cancel")),
            AppUpdate.Ready => FormLook.Button("Install", () => _ = AppUpdates.Install(XamlRoot, update), ButtonLook.Accent),
            _ => FormLook.Button("Download", () => Download(update), ButtonLook.Accent),
        };
        button.IsEnabled = update.Phase != AppUpdate.Ready || update.CanInstall;
        return button;
    }

    private async void Download(AppUpdate update)
    {
        if (await AppUpdates.MayDownload(XamlRoot, update)) Run("appUpdate.download");
    }

    private Grid Notes(AppUpdate update)
    {
        var link = new HyperlinkButton { Content = "Release notes", FontFamily = PreflightLook.Font, Padding = new Thickness(0) };
        AutomationProperties.SetName(link, "Release notes");
        link.Click += (_, _) => Open(update.ReleaseUrl);
        return FormLook.Row(FormLook.Stack(4, link, FormLook.Caption(update.PlainNotes)), null);
    }

    private async void Run(string path, params object?[] args) => FormLook.Say(_refusal, await AppUpdates.Invoke(path, args));

    private async void Open(string url) => FormLook.Say(_refusal, await AppUpdates.OpenRelease(url));
}
