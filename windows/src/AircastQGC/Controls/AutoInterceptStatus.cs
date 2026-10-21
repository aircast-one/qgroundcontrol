using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Core;
using AircastQGC.Forms;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

public sealed record InterceptConfirm(string Title, string Text);

public sealed record InterceptAction(string Label, string Path, string Args, InterceptConfirm? Confirm)
{
    internal static InterceptAction? Read(JsonNode? node) =>
        node is JsonObject json && MapUiJson.Text(json["path"]) is { Length: > 0 } path
            ? new InterceptAction(
                MapUiJson.Text(json["label"]),
                path,
                (json["args"] as JsonArray ?? []).ToJsonString(),
                json["confirm"] is JsonObject confirm
                    ? new InterceptConfirm(MapUiJson.Text(confirm["title"]), MapUiJson.Text(confirm["text"]))
                    : null)
            : null;

    internal string? Run() => MapUiJson.RefusalOf(Bridge.Invoke(Path, JsonNode.Parse(Args) as JsonArray));
}

internal sealed record AutoInterceptView(
    bool Visible, bool Problem, string Headline, IReadOnlyList<InterceptAction> Actions, InterceptAction? Entry, string? Hint)
{
    internal const string Path = "view.autoIntercept";

    internal static AutoInterceptView Read(JsonObject view) => new(
        MapUiJson.Flag(view["visible"]),
        MapUiJson.Text(view["tone"]) == "problem",
        MapUiJson.Text(view["headline"]),
        MapUiJson.Objects(view["actions"]).Select(InterceptAction.Read).OfType<InterceptAction>().ToList(),
        InterceptAction.Read(view["entry"]),
        MapUiJson.Text(view["hint"]) is { Length: > 0 } hint ? hint : null);
}

public sealed partial class AutoInterceptStatus : UserControl
{
    private const string InfoGlyph = "";
    private const string ProblemGlyph = "";

    private readonly string _client = $"auto-intercept.{Guid.NewGuid():N}";
    private readonly FontIcon _icon = FormLook.Glyph(InfoGlyph, 14, "AccentBrush");
    private readonly TextBlock _headline = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush", true);
    private readonly StackPanel _buttons = new() { Orientation = Orientation.Horizontal, Spacing = 8 };
    private readonly TextBlock _hintText = FormLook.Caption("", "TextPrimaryBrush");
    private readonly Border _pill;
    private readonly Border _hint;
    private IReadOnlyList<InterceptAction> _actions = [];
    private InterceptAction? _entry;

    public event EventHandler<GuidedAsk>? ConfirmRequested;

    public event EventHandler<string>? Refused;

    public event EventHandler<InterceptAction?>? EntryChanged;

    public AutoInterceptStatus()
    {
        _headline.VerticalAlignment = VerticalAlignment.Center;
        _icon.VerticalAlignment = VerticalAlignment.Center;
        _pill = new Border
        {
            MinHeight = 32,
            Padding = new Thickness(14, 4, 6, 4),
            CornerRadius = new CornerRadius(16),
            BorderBrush = FormLook.Brush("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            HorizontalAlignment = HorizontalAlignment.Center,
            Child = FormLook.Line(8, _icon, _headline, _buttons),
        };
        _hint = new Border
        {
            Padding = new Thickness(10, 4, 10, 4),
            CornerRadius = new CornerRadius(8),
            Background = FormLook.Brush("ChipBrush"),
            HorizontalAlignment = HorizontalAlignment.Center,
            IsHitTestVisible = false,
            Child = _hintText,
        };
        Content = new StackPanel { Spacing = 6, Children = { _pill, _hint } };
        Visibility = Visibility.Collapsed;
        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, [AutoInterceptView.Path], Render);
            Render(Bridge.Get(AutoInterceptView.Path));
        };
        Unloaded += (_, _) =>
        {
            if (!IsLoaded) BridgeWatch.Stop(_client);
        };
    }

    public void Run(InterceptAction action)
    {
        if (action.Confirm is { } confirm) ConfirmRequested?.Invoke(this, new GuidedAsk(confirm.Title, confirm.Text, action.Run));
        else Ui.Off(this, action.Run, Refuse);
    }

    private void Refuse(string? reason)
    {
        if (reason is not null) Refused?.Invoke(this, reason);
    }

    private void Render(JsonObject view)
    {
        var shown = AutoInterceptView.Read(view);
        _pill.Visibility = Ui.Shown(shown.Visible);
        _pill.Background = Ui.Token(shown.Problem ? "CautionBgBrush" : "LayerBrush");
        _icon.Glyph = shown.Problem ? ProblemGlyph : InfoGlyph;
        _icon.Foreground = Ui.Token(shown.Problem ? "CautionBrush" : "AccentBrush");
        FormLook.Say(_headline, shown.Headline);
        if (!shown.Actions.SequenceEqual(_actions))
            FormLook.Fill(_buttons, shown.Actions.Select(action => FormLook.Button(action.Label, () => Run(action))));
        _actions = shown.Actions;
        _buttons.Visibility = Ui.Shown(_actions.Count > 0);
        FormLook.Say(_hintText, shown.Hint);
        _hint.Visibility = Ui.Shown(shown.Hint is not null);
        Visibility = Ui.Shown(shown.Visible || shown.Hint is not null);
        if (shown.Entry == _entry) return;
        _entry = shown.Entry;
        EntryChanged?.Invoke(this, _entry);
    }
}
