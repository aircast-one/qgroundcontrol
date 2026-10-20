using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed class ResumeFailedPrompt
{
    private const string ResumePath = "planFly.missionController.resumeMission";

    private readonly FrameworkElement _fly;
    private ContentDialog? _dialog;
    private bool _leaving;
    private int? _failed;
    private int? _dismissed;

    public ResumeFailedPrompt(FrameworkElement fly)
    {
        _fly = fly;
        Live.Watch(fly, GuidedOffer.ViewPath, Apply);
        fly.RegisterPropertyChangedCallback(UIElement.VisibilityProperty, (_, _) => Reconsider());
    }

    private bool Due => _failed is int index && index != _dismissed && _fly.Visibility == Visibility.Visible;

    private void Apply(JsonObject view)
    {
        _failed = MapUiJson.Number(view["resumeFailedIndex"]) is { } index ? (int)index : null;
        _dismissed = _failed is null ? null : _dismissed;
        Reconsider();
    }

    private void Reconsider()
    {
        if (!Due)
        {
            _leaving = _dialog != null;
            _dialog?.Hide();
            return;
        }
        _leaving = false;
        if (_dialog == null && _fly.XamlRoot is { } root) Ask(root, _failed!.Value);
    }

    private async void Ask(XamlRoot root, int index)
    {
        var dialog = FormLook.Dialog(root, "Resume FAILED",
            FormLook.Text("Upload of resume mission failed. Confirm to retry upload", "FontSizeBody", "TextSecondaryBrush"), "Confirm");
        _dialog = dialog;
        _leaving = false;
        var answer = await PromptQueue.Show(dialog, () => Due && _failed == index);
        var withdrawn = _leaving || !Due;
        _dialog = null;
        _leaving = false;
        if (withdrawn && answer != ContentDialogResult.Primary) return;
        _dismissed = index;
        if (answer == ContentDialogResult.Primary) await FormWrites.Off(() => Bridge.Invoke(ResumePath, FormJson.Args(index)));
    }
}
