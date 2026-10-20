using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed partial class GripperPanel : UserControl
{
    private const string GripperState = "vehicle.gripper";
    private const string GripperAction = "vehicle.sendGripperAction";
    private const string FixPayload = "vehicle.fixPayload";
    private const string NoCommand = "No payload command sent yet";
    private const int Release = 0;
    private const int Grab = 1;
    private const int Hold = 2;
    private static readonly TimeSpan StatusTick = TimeSpan.FromSeconds(1);

    private readonly string _client = $"gripper-panel.{Guid.NewGuid():N}";
    private readonly StackPanel _problems = new() { Spacing = 8 };
    private readonly StackPanel _actions = new() { Spacing = 8 };
    private readonly TextBlock _status = FormLook.Text(NoCommand, "FontSizeBody", "TextSecondaryBrush");
    private readonly DispatcherQueueTimer _tick;
    private IReadOnlyList<GuidedOffer> _offers = [];
    private Payload _payload = new(1, [], []);
    private string? _refusal;
    private string _shown = "";

    public GripperPanel()
    {
        var close = FormLook.IconButton("", "Close payload", () =>
        {
            Hide();
            CloseRequested?.Invoke(this, EventArgs.Empty);
        });
        var title = FormLook.Text("Payload", "FontSizeBody", "TextPrimaryBrush", true);
        title.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(close, 1);
        Content = new Border
        {
            Width = 320,
            Background = FormLook.Brush("CardBrush"),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            CornerRadius = FormLook.Radius("RadiusOverlay"),
            Padding = new Thickness(16),
            Child = FormLook.Stack(12,
                new Grid
                {
                    ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
                    Children = { title, close },
                },
                _problems,
                _status,
                _actions),
        };
        _status.Visibility = Visibility.Visible;
        Visibility = Visibility.Collapsed;
        _tick = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _tick.Interval = StatusTick;
        _tick.Tick += (_, _) => PaintStatus();
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Halt();
    }

    public event EventHandler<GuidedAsk>? ConfirmRequested;

    public event EventHandler? CloseRequested;

    public bool IsOpen => Visibility == Visibility.Visible;

    public void Show()
    {
        _refusal = null;
        Visibility = Visibility.Visible;
        PaintStatus();
        _tick.Start();
    }

    public void Hide()
    {
        Visibility = Visibility.Collapsed;
        _tick.Stop();
    }

    private void Start()
    {
        BridgeWatch.Watch(_client, [GuidedOffer.ViewPath], Render);
        Render(Bridge.Get(GuidedOffer.ViewPath));
    }

    private void Halt()
    {
        if (IsLoaded) return;
        BridgeWatch.Stop(_client);
        _tick.Stop();
    }

    private void Render(JsonObject view)
    {
        _offers = GuidedOffer.List(view).Where(o => PayloadJson.GripperIds.Contains(o.Id) && o.Shown).ToList();
        _payload = PayloadJson.PayloadOf(view);
        if (_offers.Count == 0) Hide();
        var signature = string.Join("|", _offers.Select(o => $"{o.Id}:{o.Ready}:{o.Title}"))
            + $"#{_payload.Bays}:{string.Join(",", _payload.Empty)}#" + string.Join("|", _payload.Problems.Select(p => $"{p.Code}:{p.Text}:{p.Fix}:{p.Blocking}"));
        if (signature == _shown) return;
        _shown = signature;
        FormLook.Fill(_problems, _payload.Problems.Select(Problem));
        _problems.Visibility = VideoJson.Shown(_payload.Problems.Count > 0);
        FormLook.Fill(_actions, _payload.MultiBay ? _payload.BayNumbers.Select(Bay) : Single());
    }

    private UIElement Problem(PayloadProblem problem)
    {
        var text = FormLook.Caption(problem.Text, problem.Blocking ? "CriticalBrush" : "TextSecondaryBrush");
        if (problem.Fix.Length == 0) return text;
        var fix = FormLook.Button(problem.Fix, () => Report(FormWrites.Call(FixPayload, "The vehicle", problem.Code)));
        fix.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(fix, 1);
        return new Grid
        {
            ColumnSpacing = 8,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
            Children = { text, fix },
        };
    }

    private IEnumerable<UIElement> Single()
    {
        var release = _offers.FirstOrDefault(o => o.Id == "release");
        var others = _offers.Where(o => o.Id != "release")
            .Select(o => Enabled(FormLook.Button(o.Title, () => Report(Send(o.Id == "hold" ? Hold : Grab, 0))), o.Ready))
            .ToArray();
        return new UIElement?[]
        {
            release is null ? null : Enabled(Wide(FormLook.Button(release.Title, () => AskRelease(0, release.Title, release.Prompt), ButtonLook.Accent, "")), release.Ready),
            others.Length == 0 ? null : FormLook.Line(8, others),
        }.OfType<UIElement>();
    }

    private UIElement Bay(int bay)
    {
        var empty = _payload.Empty.Contains(bay);
        var title = $"Drop bay {bay}";
        var drop = Enabled(Wide(FormLook.Button(empty ? $"{title} · empty" : title, () => AskRelease(bay, title, $"Opens bay {bay} so its payload falls."), ButtonLook.Accent, "")), Ready("release"));
        var grab = Enabled(FormLook.Button($"Grab {bay}", () => Report(Send(Grab, bay))), Ready("grab"));
        Grid.SetColumn(grab, 1);
        return new Grid
        {
            ColumnSpacing = 8,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
            Children = { drop, grab },
        };
    }

    private void AskRelease(int bay, string title, string prompt) =>
        ConfirmRequested?.Invoke(this, new GuidedAsk(title, prompt, () => Send(Release, bay))
        {
            Destructive = true,
            Warning = bay > 0 && _payload.Empty.Contains(bay) ? $"Bay {bay} is marked empty." : "",
            WatchPath = GuidedOffer.ViewPath,
            StillOffered = view => GuidedOffer.List(view).Any(o => o.Id == "release" && o.Ready),
        });

    private bool Ready(string id) => _offers.Any(o => o.Id == id && o.Ready);

    private static string? Send(int action, int bay) => FormWrites.Call(GripperAction, "The vehicle", action, bay);

    private void Report(string? refusal)
    {
        _refusal = refusal;
        PaintStatus();
    }

    private void PaintStatus()
    {
        var status = _refusal is { } refused
            ? new PayloadStatus(refused, true)
            : PayloadJson.Status(PayloadJson.GripperState(Bridge.Get(GripperState)), DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        _status.Text = status?.Text ?? NoCommand;
        _status.Foreground = FormLook.Brush(status?.Warning == true ? "CriticalBrush" : "TextSecondaryBrush");
    }

    private static Button Wide(Button button)
    {
        button.HorizontalAlignment = HorizontalAlignment.Stretch;
        return button;
    }

    private static Button Enabled(Button button, bool ready)
    {
        button.IsEnabled = ready;
        return button;
    }
}
