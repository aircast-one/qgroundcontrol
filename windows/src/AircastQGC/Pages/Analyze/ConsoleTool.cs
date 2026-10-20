using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Documents;
using Microsoft.UI.Xaml.Input;
using Windows.ApplicationModel.DataTransfer;
using Windows.System;

namespace AircastQGC.Pages.Analyze;

internal sealed class ConsoleTool : UserControl
{
    internal const string ConsolePath = "view.mavlinkConsole";
    private const string Root = "mavlinkConsole";
    private readonly string _client = $"analyzeTool.console.{System.Guid.NewGuid():N}";
    private const string EmptyText = "> ";

    private readonly ObservableCollection<UIElement> _items = [];
    private readonly ItemsControl _lines;
    private readonly ScrollViewer _scroll;
    private readonly TextBox _input;
    private IReadOnlyList<string> _shown = [];

    public ConsoleTool()
    {
        _items.Add(Line(EmptyText));
        _lines = Ui.Column([], 4).With(column => column.ItemsSource = _items);
        _scroll = new ScrollViewer
        {
            Background = Ui.Token("BgSecondaryBrush"),
            Padding = new Thickness(16),
            Content = _lines,
        };
        _input = new TextBox
        {
            PlaceholderText = "Enter commands here...",
            FontFamily = Ui.Mono,
            AcceptsReturn = false,
            VerticalAlignment = VerticalAlignment.Center,
        }.Named("Console command");
        _input.KeyDown += OnKey;
        _input.Paste += OnPaste;

        var bar = new Grid
        {
            Background = Ui.Token("LayerBrush"),
            Padding = new Thickness(16, 10, 16, 10),
            ColumnSpacing = 8,
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) },
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = GridLength.Auto },
            },
            Children =
            {
                _input,
                Ui.IconButton("\uE70E", "Previous command", () => Recall("historyUp")).With(b => Grid.SetColumn(b, 1)),
                Ui.IconButton("\uE70D", "Next command", () => Recall("historyDown")).With(b => Grid.SetColumn(b, 2)),
                Ui.IconButton("\uE724", "Send", Send, Look.Accent).With(b => Grid.SetColumn(b, 3)),
            },
        };
        Content = new Grid
        {
            RowDefinitions =
            {
                new RowDefinition { Height = new GridLength(1, GridUnitType.Star) },
                new RowDefinition { Height = GridLength.Auto },
            },
            Children = { _scroll, bar.With(b => Grid.SetRow(b, 1)) },
        };

        Loaded += (_, _) =>
        {
            BridgeWatch.Watch(_client, [ConsolePath], Apply);
            Apply(Bridge.Group(ConsolePath));
            Lane.Run(() => Bridge.Invoke($"{Root}.open"));
        };
        Unloaded += (_, _) => BridgeWatch.Stop(_client);
    }

    private void Apply(JsonObject view)
    {
        IReadOnlyList<string> lines = view["lines"] is JsonArray raw ? raw.Select(MapUiJson.Text).ToList() : [];
        if (lines.SequenceEqual(_shown)) return;
        var following = _scroll.VerticalOffset >= _scroll.ScrollableHeight - 48;
        var kept = _shown.Count == 0 ? 0 : _shown.Zip(lines).TakeWhile(pair => pair.First == pair.Second).Count();
        _shown = lines;
        Enumerable.Range(kept, _items.Count - kept).Reverse().ToList().ForEach(_items.RemoveAt);
        (lines.Count == 0 ? new[] { EmptyText } : lines.Skip(kept)).Select(Line).ToList().ForEach(_items.Add);
        if (following) Tail();
    }

    private void Tail() => DispatcherQueue.TryEnqueue(Microsoft.UI.Dispatching.DispatcherQueuePriority.Low, () =>
    {
        _scroll.UpdateLayout();
        _scroll.ChangeView(null, _scroll.ScrollableHeight, null, true);
    });

    private static UIElement Line(string raw)
    {
        var line = raw.Replace("\r", "");
        var marked = new[] { ("WARN", "CautionBrush"), ("ERROR", "CriticalBrush") }
            .Where(mark => line.StartsWith(mark.Item1, StringComparison.Ordinal))
            .Select(mark => ((string Prefix, string Brush)?)mark)
            .FirstOrDefault();
        var text = new TextBlock
        {
            FontFamily = Ui.Mono,
            FontSize = Ui.Space("FontSizeBody"),
            Foreground = Ui.Token(Regex.IsMatch(line, @"^\w*sh> ") ? "AccentBrush" : "TextPrimaryBrush"),
            TextWrapping = TextWrapping.Wrap,
            IsTextSelectionEnabled = true,
        };
        return marked is { } mark
            ? text.With(styled =>
            {
                styled.Inlines.Add(new Run { Text = mark.Prefix, Foreground = Ui.Token(mark.Brush) });
                styled.Inlines.Add(new Run { Text = line[mark.Prefix.Length..] });
            })
            : text.With(plain => plain.Text = line);
    }

    private void OnKey(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key != VirtualKey.Enter) return;
        e.Handled = true;
        Send();
    }

    private async void OnPaste(object sender, TextControlPasteEventArgs e)
    {
        var clip = Clipboard.GetContent();
        if (!clip.Contains(StandardDataFormats.Text)) return;
        e.Handled = true;
        var pasted = (await clip.GetTextAsync()).Replace("\r\n", "\n");
        var start = _input.SelectionStart;
        var typed = _input.Text[..start] + pasted + _input.Text[(start + _input.SelectionLength)..];
        var cut = typed.LastIndexOf('\n');
        var leftover = cut < 0 ? typed : typed[(cut + 1)..];
        _input.Text = leftover;
        _input.SelectionStart = cut < 0 ? start + pasted.Length : leftover.Length;
        if (cut >= 0) SendText(typed[..cut]);
    }

    private void Send()
    {
        var text = _input.Text;
        _input.Text = "";
        SendText(text);
        Tail();
    }

    private static void SendText(string text) =>
        Lane.Run(() => Bridge.Invoke($"{Root}.sendCommand", new JsonArray(JsonValue.Create(text))));

    private void Recall(string step)
    {
        var current = _input.Text;
        Ui.Off(this, () => Bridge.Invoke($"{Root}.{step}", new JsonArray(JsonValue.Create(current))), answer =>
        {
            var recalled = answer["result"] is JsonValue value && value.TryGetValue<string>(out var text) ? text : current;
            _input.Text = recalled;
            _input.SelectionStart = recalled.Length;
            _input.Focus(FocusState.Programmatic);
        });
    }
}
