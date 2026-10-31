using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace AircastQGC.Controls;

internal static class GimbalCalls
{
    public const string View = "view.gimbalIndicator";
    private const string OthersHaveControl = "othersHaveControl";
    private const string OnScreenSetting = "settings.gimbalControllerSettings.enableOnScreenControl";
    private const string ClickAndDragSetting = "settings.gimbalControllerSettings.clickAndDrag";

    public static event EventHandler? AsksForControl;

    public static string? Refusal(JsonObject answer)
    {
        if (MapUiJson.Text(answer["refusal"]) != OthersHaveControl) return MapUiJson.RefusalOf(answer);
        AsksForControl?.Invoke(null, EventArgs.Empty);
        return null;
    }

    public static string? Call(string path, params object?[] args) => Refusal(Bridge.Invoke(path, FormJson.Args(args)));

    public static void ChooseAim(VideoAim aim)
    {
        Bridge.Set(OnScreenSetting, JsonValue.Create(aim.OnScreen));
        Bridge.Set(ClickAndDragSetting, JsonValue.Create(aim.ClickAndDrag));
    }
}

public sealed partial class GimbalIndicator : UserControl
{
    private const string SettingsPage = "view.settings(Gimbal Controller)";
    private const string SettingsGroup = "gimbalControllerSettings";
    private const string GimbalGlyph = "";
    private const double SheetWidth = 300;

    private readonly string _client = $"gimbal-indicator.{Guid.NewGuid():N}";
    private readonly Button _pill;
    private readonly TextBlock _pillText = FormLook.Caption("", "TextPrimaryBrush");
    private readonly Flyout _flyout;
    private readonly ContentControl _sheetHost = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch };
    private readonly StackPanel _sheet;
    private readonly StackPanel _choices = new() { Orientation = Orientation.Horizontal, Spacing = 4 };
    private readonly TextBlock _summary = FormLook.Text("", "FontSizeBody", "TextSecondaryBrush");
    private readonly Button _yawLock;
    private readonly Button _retract;
    private readonly Button _control;
    private readonly TextBlock _refusal = FormLook.Text("", "FontSizeBody", "CriticalBrush");
    private readonly Grid _aims = new()
    {
        ColumnSpacing = 4,
        ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition(), new ColumnDefinition() },
    };
    private readonly TextBlock _aimHint = FormLook.Caption("", "TextTertiaryBrush");
    private readonly StackPanel _aimSection;
    private readonly Lazy<UIElement> _settings;
    private GimbalState? _state;
    private string _choicesShown = "";
    private VideoAim? _aimShown;
    private long? _seenSerial;
    private bool _asking;

    public GimbalIndicator()
    {
        _pillText.Visibility = Visibility.Visible;
        _pill = new Button
        {
            Height = 28,
            Padding = new Thickness(10, 0, 10, 0),
            CornerRadius = new CornerRadius(14),
            Content = FormLook.Line(6, FormLook.Glyph(GimbalGlyph, 12, "TextTertiaryBrush"), _pillText),
        };
        PreflightLook.Chip(_pill);
        StatusLook.Dress(_pill, StatusTone.Normal);
        AutomationProperties.SetName(_pill, "Gimbal");

        _yawLock = Wide("Yaw lock", () => Act("gimbal.yawLock", !(_state?.YawLocked ?? false)));
        _retract = Wide("Retract", () => Act("gimbal.retract"));
        _control = Wide("Acquire Control", () => Act("gimbal.control", !(_state?.HaveControl ?? false)), ButtonLook.Accent);
        _aimSection = FormLook.Stack(4, FormLook.Text("On the video", "FontSizeBody", "TextPrimaryBrush", true), _aims, _aimHint);
        _sheet = FormLook.Stack(8,
            FormLook.Text("Gimbal", "FontSizeBodyLarge", "TextPrimaryBrush", true),
            _choices,
            _summary,
            _yawLock,
            Wide("Center", () => Act("gimbal.center")),
            Wide("Tilt 90", () => Act("gimbal.tilt90")),
            Wide("Point home", () => Act("gimbal.pointHome")),
            _retract,
            _control,
            _refusal,
            _aimSection,
            FormLook.Button("Gimbal settings", () => _sheetHost.Content = _settings!.Value, ButtonLook.Subtle, ""));
        _settings = new Lazy<UIElement>(SettingsSheet);
        _sheetHost.Content = _sheet;
        _flyout = new Flyout
        {
            Content = new ScrollViewer { Content = _sheetHost, MaxHeight = 600, Width = SheetWidth + 24, Padding = new Thickness(0, 0, 12, 0) },
            Placement = FlyoutPlacementMode.BottomEdgeAlignedRight,
            FlyoutPresenterStyle = PresenterStyle(),
        };
        _flyout.Opening += (_, _) =>
        {
            _sheetHost.Content = _sheet;
            FormLook.Say(_refusal, null);
            Paint();
        };
        _pill.Click += (_, _) => _flyout.ShowAt(_pill);
        Content = _pill;
        Visibility = Visibility.Collapsed;
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Halt();
    }

    public void ShowSheetAt(FrameworkElement anchor) => _flyout.ShowAt(anchor);

    private void Start()
    {
        GimbalCalls.AsksForControl -= OnAsked;
        GimbalCalls.AsksForControl += OnAsked;
        BridgeWatch.Watch(_client, [GimbalCalls.View], Render);
        Render(Bridge.Get(GimbalCalls.View));
    }

    private void Halt()
    {
        if (IsLoaded) return;
        GimbalCalls.AsksForControl -= OnAsked;
        BridgeWatch.Stop(_client);
    }

    private void OnAsked(object? sender, EventArgs e) => DispatcherQueue.TryEnqueue(AskForControl);

    private void Render(JsonObject view)
    {
        _state = PayloadJson.Gimbal(view);
        Visibility = VideoJson.Shown(_state is not null);
        if (_state is null)
        {
            _flyout.Hide();
            return;
        }
        _pillText.Text = _state.CellText.Length > 0 ? _state.CellText : "Gimbal";
        AutomationProperties.SetHelpText(_pill, _state.CellText);
        ToolTipService.SetToolTip(_pill, _state.CellText);
        if (_seenSerial is long seen && _state.AskSerial > seen) AskForControl();
        if (_state.AskSerial >= 0) _seenSerial = _state.AskSerial;
        Paint();
    }

    private void Paint()
    {
        if (_state is not { } state) return;
        var choices = string.Join("|", state.Gimbals.Select(g => $"{g.Name}:{g.Active}"));
        if (choices != _choicesShown)
        {
            _choicesShown = choices;
            FormLook.Fill(_choices, state.Gimbals.Count > 1
                ? state.Gimbals.Select(g => FormLook.Button(g.Name, () => Act("gimbal.select", g.ManagerCompid, g.DeviceId), g.Active ? ButtonLook.Accent : ButtonLook.Standard))
                : []);
        }
        _choices.Visibility = VideoJson.Shown(state.Gimbals.Count > 1);
        FormLook.Say(_summary, state.CellText);
        Label(_yawLock, state.YawLockLabel);
        _yawLock.Visibility = VideoJson.Shown(state.YawLockOffered);
        _retract.Visibility = VideoJson.Shown(state.RetractOffered);
        Label(_control, state.ControlLabel);
        _control.Visibility = VideoJson.Shown(state.ControlOffered);
        _aimSection.Visibility = VideoJson.Shown(state.Aim is not null);
        if (state.Aim != _aimShown)
        {
            _aimShown = state.Aim;
            FormLook.Fill(_aims, VideoAim.All.Select((aim, column) => AimChoice(aim, column, aim == state.Aim)));
        }
        FormLook.Say(_aimHint, state.Aim?.Hint);
    }

    private void Act(string path, params object?[] args)
    {
        var refusal = GimbalCalls.Call(path, args);
        FormLook.Say(_refusal, refusal);
        if (refusal is null) _flyout.Hide();
    }

    private async void AskForControl()
    {
        if (_asking || XamlRoot is null) return;
        _asking = true;
        _flyout.Hide();
        var yes = await FormLook.Confirm(XamlRoot, "Request Gimbal Control?", "Command not sent. Another user has control of the gimbal.", "Yes", "No");
        _asking = false;
        if (yes) FormLook.Say(_refusal, MapUiJson.RefusalOf(Bridge.Invoke("gimbal.control", FormJson.Args(true))));
    }

    private UIElement SettingsSheet() => FormLook.Stack(8,
        FormLook.Line(8,
            FormLook.IconButton("", "Back to gimbal", () => _sheetHost.Content = _sheet),
            FormLook.Margined(FormLook.Text("Gimbal settings", "FontSizeBodyLarge", "TextPrimaryBrush", true), new Thickness(0, 4, 0, 0))),
        new FormView(
            () => Bridge.Get(SettingsPage),
            raw => FormParse.SettingsSections(raw).Where(section => section.Group == SettingsGroup).ToList())
        {
            LoadingText = "Reading gimbal settings.",
            EmptyText = "No gimbal settings exposed here.",
        });

    private static Button AimChoice(VideoAim aim, int column, bool chosen)
    {
        var button = Wide(aim.Label, () => GimbalCalls.ChooseAim(aim), chosen ? ButtonLook.Accent : ButtonLook.Standard);
        button.Padding = new Thickness(4);
        button.VerticalAlignment = VerticalAlignment.Stretch;
        if (button.Content is TextBlock label)
        {
            label.TextWrapping = TextWrapping.Wrap;
            label.TextAlignment = TextAlignment.Center;
            label.FontSize = FormLook.Size("FontSizeCaption");
        }
        Grid.SetColumn(button, column);
        return button;
    }

    private static Button Wide(string text, Action onClick, ButtonLook look = ButtonLook.Standard)
    {
        var button = FormLook.Button(text, onClick, look);
        button.HorizontalAlignment = HorizontalAlignment.Stretch;
        return button;
    }

    private static void Label(Button button, string text)
    {
        if (button.Content is TextBlock label) label.Text = text;
        AutomationProperties.SetName(button, text);
    }

    private static Style PresenterStyle() => new(typeof(FlyoutPresenter))
    {
        Setters =
        {
            new Setter(Control.BackgroundProperty, FormLook.Brush("LayerBrush")),
            new Setter(Control.BorderBrushProperty, FormLook.Brush("CardStrokeBrush")),
            new Setter(Control.CornerRadiusProperty, FormLook.Radius("RadiusOverlay")),
            new Setter(Control.PaddingProperty, new Thickness(16)),
            new Setter(FrameworkElement.MaxWidthProperty, SheetWidth + 64),
        },
    };
}
