using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed class CloseGuard
{
    private const string ViewPath = "view.closeChecks";
    private const string Title = "Close Aircast QGC";

    private readonly Window _window;
    private bool _approved;
    private bool _asking;

    public CloseGuard(Window window)
    {
        _window = window;
        window.AppWindow.Closing += OnClosing;
    }

    private static IReadOnlyList<string> Prompts(JsonObject view) =>
        MapUiJson.Objects(view["prompts"])
            .Select(prompt => MapUiJson.Text(prompt["message"]).Trim())
            .Where(message => message.Length > 0)
            .ToList();

    private void OnClosing(AppWindow sender, AppWindowClosingEventArgs args)
    {
        if (_approved) return;
        args.Cancel = true;
        if (!_asking) Ask();
    }

    private async void Ask()
    {
        _asking = true;
        var prompts = Prompts(await FormWrites.Off(() => Bridge.Get(ViewPath)));
        var agreed = await Agrees(prompts);
        _asking = false;
        if (!agreed) return;
        _approved = true;
        _window.Close();
    }

    private async Task<bool> Agrees(IReadOnlyList<string> prompts) =>
        prompts.Count == 0 || (await Confirmed(prompts[0]) && await Agrees(prompts.Skip(1).ToList()));

    private async Task<bool> Confirmed(string prompt) =>
        _window.Content?.XamlRoot is { } root
        && await PromptQueue.Show(
            FormLook.Dialog(root, Title, FormLook.Text(prompt, "FontSizeBody", "TextSecondaryBrush"), "Yes", "No"),
            () => true) == ContentDialogResult.Primary;
}
