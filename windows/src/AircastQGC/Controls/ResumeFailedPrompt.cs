using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed class ResumeFailedPrompt
{
    private const string ResumePath = "planFly.missionController.resumeMission";

    private readonly FrameworkElement _host;
    private ContentDialog? _dialog;
    private int? _failed;
    private int? _asked;

    public ResumeFailedPrompt(FrameworkElement host)
    {
        _host = host;
        Live.Watch(host, GuidedOffer.ViewPath, Apply);
    }

    private void Apply(JsonObject view)
    {
        _failed = MapUiJson.Number(view["resumeFailedIndex"]) is { } index ? (int)index : null;
        if (_failed is null)
        {
            _asked = null;
            _dialog?.Hide();
            return;
        }
        if (_failed == _asked || _host.XamlRoot is not { } root) return;
        _asked = _failed;
        Ask(root, _failed.Value);
    }

    private async void Ask(XamlRoot root, int index)
    {
        var dialog = FormLook.Dialog(root, "Resume FAILED",
            FormLook.Text("Upload of resume mission failed. Confirm to retry upload", "FontSizeBody", "TextSecondaryBrush"), "Confirm");
        _dialog = dialog;
        var answer = await PromptQueue.Show(dialog, () => _failed == index);
        if (_dialog == dialog) _dialog = null;
        if (answer == ContentDialogResult.Primary) await FormWrites.Off(() => Bridge.Invoke(ResumePath, FormJson.Args(index)));
    }
}
