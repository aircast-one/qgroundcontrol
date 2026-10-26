using System;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Automation.Peers;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media.Animation;
using Windows.System;

namespace AircastQGC.Controls;

public sealed partial class RouteRecording : UserControl
{
    private const string View = "view.routeRecorder";
    private const string StartPath = "routeRecorder.start";
    private const string StopPath = "routeRecorder.stop";
    private const string RenamePath = "routeRecorder.rename";
    private const string DismissPath = "routeRecorder.dismiss";
    private static readonly TimeSpan Blink = TimeSpan.FromMilliseconds(600);

    private readonly TextBlock _pillText = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush", true);
    private readonly Border _pill;
    private readonly TextBlock _message = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush");
    private readonly TextBox _name = new() { Width = 240, VerticalAlignment = VerticalAlignment.Center };
    private readonly Button _open;
    private readonly Border _notice;
    private readonly Storyboard _blink = new();
    private string _savedFile = "";
    private string _savedName = "";
    private string? _refusal;
    private (string File, string Name)? _asked;

    public RouteRecording()
    {
        var dot = FormLook.Dot("RecordBrush", 10);
        var fade = new DoubleAnimation { From = 1, To = 0.2, Duration = Blink, AutoReverse = true, RepeatBehavior = RepeatBehavior.Forever };
        Storyboard.SetTarget(fade, dot);
        Storyboard.SetTargetProperty(fade, "Opacity");
        _blink.Children.Add(fade);
        _pill = new Border
        {
            Height = 32,
            Padding = new Thickness(14, 0, 14, 0),
            CornerRadius = new CornerRadius(16),
            HorizontalAlignment = HorizontalAlignment.Center,
            Background = FormLook.Brush("LayerBrush"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            Visibility = Visibility.Collapsed,
            Child = FormLook.Line(8, dot, FormLook.Margined(_pillText, new Thickness(0, 5, 0, 0))),
        };
        _message.VerticalAlignment = VerticalAlignment.Center;
        AutomationProperties.SetName(_name, "Route name");
        AutomationProperties.SetLiveSetting(_message, AutomationLiveSetting.Polite);
        _name.KeyDown += (_, e) =>
        {
            if (e.Key != VirtualKey.Enter) return;
            e.Handled = true;
            Rename();
        };
        _name.LostFocus += (_, _) => Rename();
        _open = FormLook.Button("Open in Plan", OpenInPlan);
        _notice = new Border
        {
            Padding = new Thickness(12, 8, 8, 8),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            HorizontalAlignment = HorizontalAlignment.Center,
            Background = FormLook.Brush("CardBrush"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            Visibility = Visibility.Collapsed,
            Child = FormLook.Line(8, _message, _name, _open, FormLook.Button("Dismiss", Dismiss)),
        };
        Content = FormLook.Stack(8, _pill, _notice);
        Visibility = Visibility.Collapsed;
        Live.Watch(this, View, Render);
    }

    public event EventHandler? StateChanged;

    public event EventHandler<string>? Refused;

    public event EventHandler<string>? OpenRequested;

    public bool Available { get; private set; }

    public bool Recording { get; private set; }

    public async void Toggle()
    {
        var path = Recording ? StopPath : StartPath;
        if (await Task.Run(() => MapUiJson.RefusalOf(Bridge.Invoke(path))) is { } refusal) Refused?.Invoke(this, refusal);
    }

    private void Render(JsonObject view)
    {
        var savedFile = MapUiJson.Text(view["savedFile"]);
        var message = MapUiJson.Text(view["message"]);
        Available = MapUiJson.Flag(view["available"]);
        Recording = MapUiJson.Flag(view["recording"]);
        _refusal = savedFile == _savedFile && message.Length > 0 ? _refusal : null;
        _savedFile = savedFile;
        _savedName = MapUiJson.Text(view["savedName"]);
        FormLook.Say(_pillText, MapUiJson.Text(view["text"]));
        _pill.Visibility = FormLook.Shown(Recording && _pillText.Text.Length > 0);
        if (Recording) _blink.Begin();
        else _blink.Stop();
        FormLook.Say(_message, _refusal ?? message);
        if (_name.FocusState == FocusState.Unfocused) _name.Text = _savedName;
        _name.Visibility = FormLook.Shown(savedFile.Length > 0);
        _open.Visibility = _name.Visibility;
        _notice.Visibility = FormLook.Shown(!Recording && message.Length > 0);
        Visibility = FormLook.Shown(_pill.Visibility == Visibility.Visible || _notice.Visibility == Visibility.Visible);
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    private async void Rename()
    {
        var wanted = _name.Text.Trim();
        var file = _savedFile;
        if (file.Length == 0 || wanted.Length == 0 || wanted == _savedName)
        {
            _name.Text = _savedName;
            return;
        }
        if (_asked == (file, wanted)) return;
        _asked = (file, wanted);
        var answer = await Task.Run(() => Bridge.Invoke(RenamePath, new JsonArray(file, wanted)));
        if (MapUiJson.RefusalOf(answer) is not { } refusal) return;
        _asked = null;
        _refusal = refusal;
        FormLook.Say(_message, refusal);
        _name.Text = _savedName;
    }

    private void OpenInPlan()
    {
        OpenRequested?.Invoke(this, _savedFile);
        Dismiss();
    }

    private static void Dismiss() => Task.Run(() => Bridge.Invoke(DismissPath));
}
