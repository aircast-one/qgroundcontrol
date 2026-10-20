using System;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal sealed record MissionComplete(
    long Id,
    int ImagesTaken,
    int? ResumeFrom,
    bool BatteryWarning,
    bool Interrupted,
    string Title,
    string Detail,
    string ResumeLabel)
{
    public static MissionComplete? From(JsonObject view) =>
        MapUiJson.Flag(view["open"])
            ? new MissionComplete(
                (long)(MapUiJson.Number(view["id"]) ?? 0),
                (int)(MapUiJson.Number(view["imagesTaken"]) ?? 0),
                MapUiJson.Number(view["resumeFromWaypoint"]) is { } at ? (int)at : null,
                MapUiJson.Flag(view["batteryWarning"]),
                MapUiJson.Flag(view["interrupted"]),
                MapUiJson.Text(view["title"]) is { Length: > 0 } title ? title : MissionCompleteDialog.CompleteTitle,
                MapUiJson.Text(view["detail"]),
                MapUiJson.Text(view["resumeLabel"]))
            : null;
}

internal sealed class MissionCompleteDialog
{
    public const string CompleteTitle = "Flight plan complete";

    private const string ViewPath = "view.missionComplete";
    private const string DismissPath = "missionComplete.dismiss";
    private const string RemovePath = "plan.removeAllFromVehicle";
    private const string ResumePath = "planFly.missionController.resumeMission";
    private const string BatterySwap = "If you are changing batteries for Resume Mission do not disconnect from the vehicle.";
    private const string ResumeNote = "Resume Mission will rebuild the current mission from the last flown waypoint and upload it to the vehicle for the next flight.";

    private readonly FrameworkElement _host;
    private ContentDialog? _dialog;
    private MissionComplete? _latest;
    private long? _shownId;

    public MissionCompleteDialog(FrameworkElement host)
    {
        _host = host;
        Live.Watch(host, ViewPath, Apply);
    }

    private void Apply(JsonObject view)
    {
        var notice = MissionComplete.From(view);
        _latest = notice;
        if (notice?.Id == _shownId) return;
        _dialog?.Hide();
        if (notice is null || _host.XamlRoot is not { } root) return;
        _shownId = notice.Id;
        Present(root, notice);
    }

    private async void Present(XamlRoot root, MissionComplete notice)
    {
        var dialog = FormLook.Dialog(root, notice.Title, new Border(), null, "Close");
        dialog.Content = Body(notice, dialog);
        dialog.DefaultButton = ContentDialogButton.None;
        _dialog = dialog;
        await PromptQueue.Show(dialog, () => _latest?.Id == notice.Id);
        if (_dialog == dialog) _dialog = null;
        if (_latest?.Id == notice.Id) await FormWrites.Off(() => Bridge.Invoke(DismissPath, FormJson.Args(notice.Id)));
    }

    private static UIElement Body(MissionComplete notice, ContentDialog dialog) =>
        notice.Interrupted && notice.ResumeFrom is { } interruptedAt
            ? FormLook.Stack(12,
                FormLook.Text(notice.Detail, "FontSizeBody", "TextSecondaryBrush"),
                Choice(notice.ResumeLabel.Length > 0 ? notice.ResumeLabel : $"Resume mission from waypoint {interruptedAt}",
                    ButtonLook.Accent, dialog, () => Bridge.Invoke(ResumePath, FormJson.Args(interruptedAt))),
                Choice("Leave plan on vehicle", ButtonLook.Standard, dialog, null),
                Choice("Remove plan from vehicle", ButtonLook.Subtle, dialog, () => Bridge.Invoke(RemovePath)),
                Battery(notice))
            : FormLook.Stack(12,
                notice.ImagesTaken == 0 ? null : Centered(FormLook.Text($"{notice.ImagesTaken} Images Taken")),
                Choice("Remove plan from vehicle", ButtonLook.Accent, dialog, () => Bridge.Invoke(RemovePath)),
                Choice("Leave plan on vehicle", ButtonLook.Standard, dialog, null),
                notice.ResumeFrom is { } waypoint
                    ? FormLook.Stack(12,
                        new Border { Height = 1, Background = FormLook.Brush("DividerBrush") },
                        Choice($"Resume mission from waypoint {waypoint}", ButtonLook.Standard, dialog,
                            () => Bridge.Invoke(ResumePath, FormJson.Args(waypoint))),
                        FormLook.Caption(ResumeNote))
                    : null,
                Battery(notice));

    private static Button Choice(string text, ButtonLook look, ContentDialog dialog, Func<JsonObject>? send)
    {
        var button = FormLook.Button(text, async () =>
        {
            if (send is not null) await FormWrites.Off(send);
            dialog.Hide();
        }, look);
        button.HorizontalAlignment = HorizontalAlignment.Stretch;
        return button;
    }

    private static TextBlock? Battery(MissionComplete notice) =>
        notice.BatteryWarning ? FormLook.Caption(BatterySwap, "CautionBrush") : null;

    private static TextBlock Centered(TextBlock text)
    {
        text.HorizontalAlignment = HorizontalAlignment.Center;
        text.TextAlignment = TextAlignment.Center;
        return text;
    }
}
