using System;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed record DeepLinkSetup(string Host, string Title, string Text, string Accept, string Decline)
{
    public const string View = "view.deepLinkSetup";
    public const string AcceptPath = "deepLinkSetup.accept";
    public const string DeclinePath = "deepLinkSetup.decline";

    public static DeepLinkSetup? From(JsonObject view) =>
        MapUiJson.Flag(view["show"])
            ? new DeepLinkSetup(MapUiJson.Text(view["host"]), MapUiJson.Text(view["title"]), MapUiJson.Text(view["text"]),
                MapUiJson.Text(view["accept"]), MapUiJson.Text(view["decline"]))
            : null;
}

public sealed class DeepLinkSetupPrompt : UserControl
{
    private static readonly TimeSpan Retry = TimeSpan.FromSeconds(1);

    private readonly DispatcherTimer _retry = new() { Interval = Retry };
    private DeepLinkSetup? _wanted;
    private ContentDialog? _dialog;

    public DeepLinkSetupPrompt()
    {
        IsHitTestVisible = false;
        _retry.Tick += (_, _) =>
        {
            _retry.Stop();
            Show();
        };
        Live.Watch(this, DeepLinkSetup.View, view =>
        {
            _wanted = DeepLinkSetup.From(view);
            Show();
        });
    }

    private async void Show()
    {
        var wanted = _wanted;
        if (wanted is null)
        {
            _dialog?.Hide();
            return;
        }
        if (_dialog is not null || XamlRoot is null) return;
        var dialog = FormLook.Dialog(XamlRoot, wanted.Title, FormLook.Text(wanted.Text, "FontSizeBody", "TextSecondaryBrush"), wanted.Accept, wanted.Decline);
        _dialog = dialog;
        var result = await Shown(dialog);
        _dialog = null;
        if (result is null)
        {
            _retry.Start();
            return;
        }
        if (_wanted?.Host != wanted.Host) return;
        var path = result == ContentDialogResult.Primary ? DeepLinkSetup.AcceptPath : DeepLinkSetup.DeclinePath;
        await FormWrites.Off(() => FormWrites.Ok(path, wanted.Host));
    }

    private static async Task<ContentDialogResult?> Shown(ContentDialog dialog)
    {
        try { return await dialog.ShowAsync(); }
        catch (COMException) { return null; }
    }
}
