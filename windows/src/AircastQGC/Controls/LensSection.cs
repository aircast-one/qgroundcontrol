using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class LensSection : UserControl
{
    private const string MeasurePath = "camera.measureLens";
    private const string ForgetPath = "camera.forgetLens";

    private readonly TextBlock _summary = FormLook.Text("");
    private readonly TextBlock _measured = FormLook.Caption("");
    private readonly TextBlock _progress = FormLook.Caption("");
    private readonly TextBlock _error = FormLook.Caption("", "CriticalBrush");
    private readonly TextBlock _reason = FormLook.Caption("", "TextTertiaryBrush");
    private readonly Button _measure;
    private readonly Button _forget;

    public LensSection()
    {
        _measure = FormLook.Button("Measure the lens", () => Call(MeasurePath), ButtonLook.Subtle);
        _forget = FormLook.Button("Forget measurements", () => Call(ForgetPath), ButtonLook.Subtle);
        Content = FormLook.Stack(PreflightLook.Size("SpaceXs"),
            new TextBlock { Text = "LENS", Style = (Style)Application.Current.Resources["StatLabel"] }, _summary, _measured, _progress, _error, _measure, _reason, _forget);
        Visibility = Visibility.Collapsed;
        Live.Watch(this, VideoMarksModel.LensView, Render);
    }

    internal void Render(JsonObject view)
    {
        var lens = VideoMarksModel.Lens(view);
        Visibility = FormLook.Shown(lens is not null);
        if (lens is null) return;
        FormLook.Say(_summary, lens.Summary);
        FormLook.Say(_measured, VideoMarksModel.MeasuredText(lens.MeasuredZooms));
        FormLook.Say(_progress, lens.Measuring ? lens.Progress : null);
        FormLook.Say(_error, lens.Error);
        FormLook.Say(_reason, lens.Measuring ? null : lens.Reason);
        _measure.IsEnabled = lens.CanMeasure;
        ((TextBlock)_measure.Content).Text = lens.Measuring ? "Measuring…" : "Measure the lens";
        _forget.Visibility = FormLook.Shown(lens.MeasuredZooms.Count > 0 && !lens.Measuring);
    }

    private void Call(string path) => Task.Run(() => Bridge.Invoke(path)).ContinueWith(
        answer => DispatcherQueue.TryEnqueue(() =>
        {
            Render(Bridge.Get(VideoMarksModel.LensView));
            if (VideoJson.Refusal(answer.Result, "The camera did not answer.") is { } reason) FormLook.Say(_error, reason);
        }), TaskScheduler.Default);
}
