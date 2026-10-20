using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Runtime.InteropServices.WindowsRuntime;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Shapes;
using Windows.ApplicationModel.DataTransfer;
using Windows.Storage.Streams;

namespace AircastQGC.Pages.Device;

internal sealed record DeviceContext(string Origin, Action<string> Open, Action Close, Action<string> Say)
{
    public string Url(string path) => DeviceLink.Resolve(Origin, path);
}

internal sealed record Confirm(string Title, string Text, string Yes);

internal enum FieldKind { Text, Number, Uri, Password }

internal enum RowState { Neutral, Done, NeedsAttention }

internal enum LinkLook { Filled, Tonal, Outlined, Text }

internal sealed record Field(
    string Name,
    string Label,
    string Value,
    string Placeholder = "",
    string Hint = "",
    FieldKind Kind = FieldKind.Text,
    IReadOnlyList<Option>? Options = null,
    int Lines = 1,
    Func<IReadOnlyDictionary<string, string>, bool>? ShownWhen = null);

internal static class Kit
{
    public const string Waiting = "Waiting for the device to answer…";
    public static readonly FontFamily MonoFont = new("Cascadia Mono, Consolas");
    public static readonly Thickness Pad = new(16, 8, 16, 8);

    public static string LevelBrush(int level) => level switch
    {
        DeviceLevel.Ready => "SuccessBrush",
        DeviceLevel.Warn => "CautionBrush",
        DeviceLevel.Blocked => "CriticalBrush",
        _ => "TextSecondaryBrush",
    };

    public static string LevelTextBrush(int level) => level == DeviceLevel.Neutral ? "TextPrimaryBrush" : LevelBrush(level);

    public static string LevelGlyph(int level) => level switch
    {
        DeviceLevel.Ready => "",
        DeviceLevel.Neutral => "",
        _ => "",
    };

    public static string PageGlyph(string icon) => icon switch
    {
        "qr-code" => "",
        "video" => "",
        "sliders-horizontal" => "",
        "network" => "",
        "activity" => "",
        "gauge" => "",
        "clock" => "",
        "settings" => "",
        "server" => "",
        "scan-search" => "",
        _ => "",
    };

    public static TextBlock MonoText(string text, string size = "FontSizeBody", string brush = "TextPrimaryBrush")
    {
        var block = FormLook.Text(text, size, brush);
        block.FontFamily = MonoFont;
        block.IsTextSelectionEnabled = true;
        return block;
    }

    public static T Padded<T>(T element) where T : FrameworkElement => FormLook.Margined(element, Pad);

    public static FlowPanel Flow(params UIElement?[] children) => FormLook.Fill(new FlowPanel(), children);

    public static Grid Split(FrameworkElement left, FrameworkElement right)
    {
        left.VerticalAlignment = VerticalAlignment.Center;
        right.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(right, 1);
        return new Grid
        {
            ColumnSpacing = 12,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
            Children = { left, right },
        };
    }

    public static Border Dot(int level, double size = 8) => FormLook.Dot(LevelBrush(level), size);

    public static FrameworkElement BadgeView(Badge badge) => badge.Text.Length == 0
        ? new Border()
        : FormLook.Line(6, Dot(badge.Level), FormLook.Text(badge.Text, "FontSizeCaption", LevelBrush(badge.Level)));

    public static Node? Text(string text) => text.Length == 0 ? null
        : DeviceTree.Leaf($"text:{text}", text, () => Padded(FormLook.Text(text, "FontSizeBody", "TextSecondaryBrush")));

    public static Node? Hint(string text) => text.Length == 0 ? null
        : DeviceTree.Leaf($"hint:{text}", text, () => Padded(FormLook.Caption(text, "TextTertiaryBrush")));

    public static Node? StatusLine(string text, int level = DeviceLevel.Neutral) => text.Length == 0 ? null
        : DeviceTree.Leaf($"status:{text}", $"{level}", () => Padded(FormLook.Text(text, "FontSizeBody", LevelBrush(level))));

    public static Node? Mono(string text) => text.Length == 0 ? null
        : DeviceTree.Leaf($"mono:{text}", text, () => Padded(MonoText(text, "FontSizeCaption")));

    public static Node? Notice(Badge? notice) => notice is null || notice.Text.Length == 0 ? null
        : DeviceTree.Leaf($"notice:{notice.Text}", $"{notice.Level}", () => NoticeView(notice));

    public static UIElement NoticeView(Badge notice) => new Border
    {
        Margin = Pad,
        Padding = new Thickness(16, 12, 16, 12),
        CornerRadius = FormLook.Radius("RadiusOverlay"),
        Background = FormLook.Brush("ControlBrush"),
        Child = FormLook.Lead(12, FormLook.Glyph(LevelGlyph(notice.Level), 16, LevelBrush(notice.Level)), FormLook.Text(notice.Text)),
    };

    public static Node Fact(string label, string value, int level = DeviceLevel.Neutral) =>
        DeviceTree.Leaf($"fact:{label}", DJ.Sig(value, level), () => FactView(label, value, level));

    public static UIElement FactView(string label, string value, int level = DeviceLevel.Neutral)
    {
        var shown = FormLook.Text(value, "FontSizeBody", LevelTextBrush(level));
        shown.TextAlignment = TextAlignment.Right;
        shown.IsTextSelectionEnabled = true;
        var grid = new Grid
        {
            Margin = Pad,
            ColumnSpacing = 16,
            ColumnDefinitions = { new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition() },
            Children = { FormLook.Text(label, "FontSizeBody", "TextSecondaryBrush"), shown },
        };
        Grid.SetColumn(shown, 1);
        return grid;
    }

    public static Node? StackedFact(string label, string value) => value.Length == 0 && label.Length == 0 ? null
        : DeviceTree.Leaf($"stacked:{label}", value, () => Padded(FormLook.Stack(2, FormLook.Caption(label), FormLook.Text(value))));

    public static Node Step(int step, string title, string text = "") =>
        DeviceTree.Leaf($"step:{step}", DJ.Sig(title, text), () => FormLook.Margined(FormLook.Stack(2,
            FormLook.Text($"{step} · {title}", "FontSizeBody", "TextPrimaryBrush", true),
            FormLook.Text(text, "FontSizeBody", "TextSecondaryBrush")), new Thickness(16, 12, 16, 4)));

    public static Node Sub(string text) =>
        DeviceTree.Leaf($"sub:{text}", text, () => FormLook.Margined(FormLook.Text(text, "FontSizeBody", "TextPrimaryBrush", true), new Thickness(16, 16, 16, 4)));

    public static Node Section(string title, Badge? badge, IEnumerable<Node?> body) =>
        DeviceTree.Kept($"section:{title}", () => new SectionBox(), box => box.Show(title, badge, body));

    public static Node Group(string key, IEnumerable<Node?> body) =>
        DeviceTree.Kept($"group:{key}", () => new StackPanel(), panel => DeviceTree.Sync(panel, body.OfType<Node>()));

    public static Node Buttons(string key, IEnumerable<Node?> buttons) =>
        DeviceTree.Kept($"buttons:{key}", () => new FlowPanel { Margin = Pad }, panel => DeviceTree.Sync(panel, buttons.OfType<Node>()));

    public static Node Disclosure(string title, string summary, Badge? badge, bool initiallyOpen, Func<IEnumerable<Node?>> content) =>
        DeviceTree.Kept($"disclosure:{title}", () => new DisclosureBox(initiallyOpen), box => box.Show(title, summary, badge, content));

    public static Node Row(string title, string status, string glyph, RowState state, Action? open, string signature = "") =>
        DeviceTree.Leaf($"row:{title}", DJ.Sig(status, glyph, state, signature, open is null), () => RowView(title, status, glyph, state, open));

    public static UIElement RowView(string title, string status, string glyph, RowState state, Action? open)
    {
        var mark = state switch
        {
            RowState.Done => (FrameworkElement)FormLook.Glyph("", 14, "SuccessBrush"),
            RowState.NeedsAttention => FormLook.Dot("CautionBrush", 8),
            _ => new Border(),
        };
        var grid = new Grid
        {
            Padding = new Thickness(16, 10, 16, 10),
            MinHeight = 52,
            ColumnDefinitions =
            {
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition(),
                new ColumnDefinition { Width = GridLength.Auto },
                new ColumnDefinition { Width = GridLength.Auto },
            },
        };
        var icon = FormLook.Glyph(glyph.Length > 0 ? glyph : "", 16);
        icon.Visibility = FormLook.Shown(glyph.Length > 0);
        icon.Margin = new Thickness(0, 0, 12, 0);
        mark.Margin = new Thickness(12, 0, 0, 0);
        var text = FormLook.Stack(2, FormLook.Text(title), FormLook.Caption(status));
        text.VerticalAlignment = VerticalAlignment.Center;
        var chevron = FormLook.Glyph("", 12);
        chevron.Visibility = FormLook.Shown(open is not null);
        chevron.Margin = new Thickness(12, 0, 0, 0);
        new FrameworkElement[] { icon, text, mark, chevron }.Select((child, at) => (child, at)).ToList().ForEach(pair =>
        {
            pair.child.VerticalAlignment = VerticalAlignment.Center;
            Grid.SetColumn(pair.child, pair.at);
            grid.Children.Add(pair.child);
        });
        return open is null ? grid : FormLook.Stretched(grid, status.Length > 0 ? $"{title}, {status}" : title, open);
    }

    public static Node Copy(DeviceContext ctx, string label, string text, string hint = "", bool qr = false, int level = DeviceLevel.Neutral) =>
        DeviceTree.Leaf($"copy:{label}:{text}", DJ.Sig(hint, qr, level), () => CopyView(ctx, label, text, hint, qr, level));

    public static UIElement CopyView(DeviceContext ctx, string label, string text, string hint, bool qr, int level)
    {
        var copy = FormLook.IconButton("", $"Copy {label}", () => Clip(ctx, text));
        var show = FormLook.IconButton("", $"Show {label} as a QR code", () => { });
        show.Click += async (_, _) => await QrDialog(show.XamlRoot, label, text);
        show.Visibility = FormLook.Shown(qr);
        var words = FormLook.Stack(2,
            FormLook.Text(label, "FontSizeBody", level == DeviceLevel.Neutral ? "TextSecondaryBrush" : LevelBrush(level)),
            MonoText(text),
            FormLook.Caption(hint, "TextTertiaryBrush"));
        var buttons = FormLook.Line(4, show, copy);
        buttons.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(buttons, 1);
        return new Grid
        {
            Margin = Pad,
            ColumnSpacing = 8,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
            Children = { words, buttons },
        };
    }

    public static void Clip(DeviceContext ctx, string text, string said = "Copied")
    {
        var package = new DataPackage();
        package.SetText(text);
        Clipboard.SetContent(package);
        ctx.Say(said);
    }

    public static async Task QrDialog(XamlRoot? root, string title, string text)
    {
        if (root is null) return;
        var image = new Image { Width = 240, Height = 240, Stretch = Stretch.Uniform };
        AutomationProperties.SetName(image, $"QR code for {text}");
        var caption = MonoText(text, "FontSizeCaption", "TextSecondaryBrush");
        caption.TextAlignment = TextAlignment.Center;
        caption.MaxWidth = 256;
        var content = FormLook.Stack(12, image, caption);
        content.HorizontalAlignment = HorizontalAlignment.Center;
        var dialog = FormLook.Dialog(root, title, content, null, "Done");
        _ = Fill(image, text);
        await FormLook.Show(dialog);
    }

    private static async Task Fill(Image image, string text)
    {
        if (await DeviceLink.Qr(text) is not { } png) return;
        using var stream = new InMemoryRandomAccessStream();
        await stream.WriteAsync(png.AsBuffer());
        stream.Seek(0);
        var bitmap = new BitmapImage();
        await bitmap.SetSourceAsync(stream);
        image.Source = bitmap;
    }

    public static Node Action(DeviceContext ctx, string label, DeviceRequest request, string success = "", Confirm? confirm = null, bool tonal = true, Action? done = null) =>
        DeviceTree.Kept($"action:{label}", () => new ActionBox(), box => box.Show(ctx, label, request, success, confirm, tonal, done));

    public static Node Switch(DeviceContext ctx, string title, bool on, DeviceRequest onRequest, DeviceRequest offRequest, string detail = "", string success = "", string? offSuccess = null, bool enabled = true, Confirm? confirmOn = null) =>
        DeviceTree.Kept($"switch:{title}", () => new SwitchBox(), box => box.Show(ctx, title, on, onRequest, offRequest, detail, success, offSuccess ?? success, enabled, confirmOn));

    public static Node Form(DeviceContext ctx, string key, IReadOnlyList<Field> fields, string method, string path, string submit,
        string success = "", Confirm? confirm = null, string note = "", IReadOnlyDictionary<string, string>? extra = null, Action? done = null) =>
        DeviceTree.Leaf($"form:{path}:{key}",
            DJ.Sig(string.Join("|", fields.Select(f => $"{f.Name}={f.Value}:{string.Join(",", (f.Options ?? []).Select(o => o.Value + "/" + o.Label))}")), method, submit, note, confirm?.Text, string.Join(",", (extra ?? new Dictionary<string, string>()).Select(kv => kv.Key + "=" + kv.Value))),
            () => new FormBox(ctx, fields, method, path, submit, success, confirm, note, extra ?? new Dictionary<string, string>(), done));

    public static Node Link(DeviceContext ctx, string href, string text, LinkLook look = LinkLook.Tonal, bool external = false, string? glyph = null) =>
        DeviceTree.Leaf($"link:{text}", DJ.Sig(href, look, external, glyph), () => LinkView(ctx, href, text, look, external, glyph));

    public static Button LinkView(DeviceContext ctx, string href, string text, LinkLook look = LinkLook.Tonal, bool external = false, string? glyph = null)
    {
        var button = FormLook.Button(text, async () => await Launch(ctx, href), look switch
        {
            LinkLook.Filled => ButtonLook.Accent,
            LinkLook.Text => ButtonLook.Subtle,
            _ => ButtonLook.Standard,
        }, glyph ?? (external ? "" : null));
        ToolTipService.SetToolTip(button, href);
        return button;
    }

    public static async Task Launch(DeviceContext ctx, string href)
    {
        if (!Uri.TryCreate(href, UriKind.Absolute, out var uri))
        {
            ctx.Say($"{href} is not an address Windows can open.");
            return;
        }
        try
        {
            if (!await Windows.System.Launcher.LaunchUriAsync(uri)) ctx.Say($"Windows could not open {uri.Host}.");
        }
        catch (Exception error)
        {
            ctx.Say($"Windows could not open {uri.Host}: {error.Message}");
        }
    }

    public static Node Button(string key, string text, Action onClick, ButtonLook look = ButtonLook.Standard, bool enabled = true, string? glyph = null) =>
        DeviceTree.Kept($"button:{key}", () => new ButtonBox(), box => box.Show(text, onClick, look, enabled, glyph));

    public static ComboBox Select(string label, string value, IReadOnlyList<Option> options, Action<string> pick, bool enabled = true)
    {
        var at = options.ToList().FindIndex(o => o.Value == value);
        var combo = FormLook.Combo(options.Select(o => o.Label), at, double.NaN, at < 0 ? value : "");
        combo.Header = label.Length > 0 ? label : null;
        combo.HorizontalAlignment = HorizontalAlignment.Stretch;
        combo.IsEnabled = enabled;
        AutomationProperties.SetName(combo, label);
        combo.SelectionChanged += (_, _) =>
        {
            if (combo.SelectedIndex >= 0 && combo.SelectedIndex < options.Count) pick(options[combo.SelectedIndex].Value);
        };
        return combo;
    }

    public static Node Chart(Chart chart) =>
        DeviceTree.Kept($"chart:{chart.Label}", () => new ChartBox(), box => box.Show(chart));

    public static async Task<string?> TypedConfirm(XamlRoot? root, string title, string text, string label, string expected, string yes)
    {
        if (root is null) return null;
        var typed = FormLook.Field("", label, expected);
        var dialog = FormLook.Dialog(root, title, FormLook.Stack(16, FormLook.Text(text, "FontSizeBody", "TextSecondaryBrush"), typed), yes);
        dialog.IsPrimaryButtonEnabled = false;
        typed.TextChanged += (_, _) => dialog.IsPrimaryButtonEnabled = expected.Length > 0 && typed.Text.Trim() == expected;
        return await FormLook.Show(dialog) == ContentDialogResult.Primary ? typed.Text.Trim() : null;
    }

    public static async Task<bool> Ask(XamlRoot? root, Confirm? confirm) =>
        confirm is null || await FormLook.Confirm(root, confirm.Title, confirm.Text, confirm.Yes);

    public static TextBlock Refusal() => FormLook.Caption("", "CriticalBrush");

    public static IEnumerable<Node?> Endpoints(DeviceContext ctx, IEnumerable<Endpoint> endpoints) =>
        endpoints.Select(endpoint => (Node?)Copy(ctx, EndpointLabel(endpoint), endpoint.Url,
            string.Join(" ", new[] { endpoint.Hint, endpoint.Note }.Where(t => t.Length > 0)), true,
            endpoint.Down ? DeviceLevel.Warn : DeviceLevel.Neutral));

    public static string EndpointLabel(Endpoint endpoint) =>
        endpoint.Proto.Length > 0 && !endpoint.Label.StartsWith(endpoint.Proto) ? $"{endpoint.Proto} · {endpoint.Label}" : endpoint.Label;

    public static RowState State(bool needs) => needs ? RowState.NeedsAttention : RowState.Neutral;

    public static Node PageRow(DeviceContext ctx, PageLink page) =>
        Row(page.Title, page.Summary, PageGlyph(page.Icon), State(page.Needs), () => ctx.Open(page.Href), page.Href);
}

internal sealed class SectionBox : UserControl
{
    private readonly TextBlock _title = FormLook.Text("", "FontSizeBody", "TextPrimaryBrush", true);
    private readonly ContentControl _badge = new() { IsTabStop = false, VerticalAlignment = VerticalAlignment.Center };
    private readonly StackPanel _body = new() { Padding = new Thickness(0, 8, 0, 8) };

    public SectionBox()
    {
        _title.Visibility = Visibility.Visible;
        _title.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(_badge, 1);
        Content = FormLook.Stack(0,
            new Grid
            {
                Margin = new Thickness(0, 20, 0, 8),
                ColumnSpacing = 12,
                ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
                Children = { _title, _badge },
            },
            new Border
            {
                Background = FormLook.Brush("CardBrush"),
                BorderBrush = FormLook.Brush("CardStrokeBrush"),
                BorderThickness = new Thickness(1),
                CornerRadius = FormLook.Radius("RadiusOverlay"),
                Child = _body,
            });
    }

    public void Show(string title, Badge? badge, IEnumerable<Node?> body)
    {
        _title.Text = title;
        AutomationProperties.SetName(this, title);
        _badge.Content = badge is null ? null : Kit.BadgeView(badge);
        DeviceTree.Sync(_body, body.OfType<Node>());
        _body.Visibility = FormLook.Shown(_body.Children.Count > 0);
    }
}

internal sealed class DisclosureBox : UserControl
{
    private readonly TextBlock _title = FormLook.Text("");
    private readonly TextBlock _summary = FormLook.Caption("");
    private readonly ContentControl _badge = new() { IsTabStop = false, VerticalAlignment = VerticalAlignment.Center };
    private readonly FontIcon _chevron = FormLook.Glyph("", 12);
    private readonly StackPanel _body = new();
    private readonly Button _head;
    private Func<IEnumerable<Node?>> _content = () => [];
    private bool _open;

    public DisclosureBox(bool open)
    {
        _open = open;
        _title.Visibility = Visibility.Visible;
        var text = FormLook.Stack(2, _title, _summary);
        var line = new Grid
        {
            Padding = new Thickness(16, 10, 16, 10),
            MinHeight = 52,
            ColumnSpacing = 12,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition { Width = GridLength.Auto } },
            Children = { text, _badge, _chevron },
        };
        text.VerticalAlignment = VerticalAlignment.Center;
        _chevron.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(_badge, 1);
        Grid.SetColumn(_chevron, 2);
        _head = FormLook.Stretched(line, "", Toggle);
        Content = FormLook.Stack(0, _head, _body);
    }

    public void Show(string title, string summary, Badge? badge, Func<IEnumerable<Node?>> content)
    {
        _title.Text = title;
        FormLook.Say(_summary, summary);
        _badge.Content = badge is null ? null : Kit.BadgeView(badge);
        _content = content;
        Paint();
    }

    private void Toggle()
    {
        _open = !_open;
        Paint();
    }

    private void Paint()
    {
        _chevron.Glyph = _open ? "" : "";
        AutomationProperties.SetName(_head, $"{(_open ? "Collapse" : "Expand")} {_title.Text}");
        _body.Visibility = FormLook.Shown(_open);
        if (_open) DeviceTree.Sync(_body, _content().OfType<Node>());
    }
}

internal sealed class ButtonBox : UserControl
{
    private Action _click = () => { };

    public void Show(string text, Action onClick, ButtonLook look, bool enabled, string? glyph)
    {
        _click = onClick;
        var button = Content as Button;
        if (button is null || AutomationProperties.GetName(button) != text || Tag as string != $"{look}{glyph}")
        {
            button = FormLook.Button(text, () => _click(), look, glyph);
            Tag = $"{look}{glyph}";
            Content = button;
        }
        button.IsEnabled = enabled;
    }
}

internal sealed class ActionBox : UserControl
{
    private readonly TextBlock _refusal = Kit.Refusal();
    private readonly ContentControl _host = new() { IsTabStop = false };
    private DeviceContext? _ctx;
    private DeviceRequest? _request;
    private string _success = "";
    private Confirm? _confirm;
    private Action? _done;
    private Button? _button;
    private string _look = "";
    private bool _busy;

    public ActionBox()
    {
        Content = FormLook.Stack(4, _host, _refusal);
    }

    public void Show(DeviceContext ctx, string label, DeviceRequest request, string success, Confirm? confirm, bool tonal, Action? done)
    {
        (_ctx, _request, _success, _confirm, _done) = (ctx, request, success, confirm, done);
        var look = $"{label}{tonal}";
        if (_button is null || look != _look)
        {
            _button = FormLook.Button(label, Press, tonal ? ButtonLook.Standard : ButtonLook.Subtle);
            _look = look;
            _host.Content = _button;
        }
        _button.IsEnabled = !_busy;
    }

    private async void Press()
    {
        if (_request is not { } request || _ctx is not { } ctx || !await Kit.Ask(XamlRoot, _confirm)) return;
        _busy = true;
        if (_button is not null) _button.IsEnabled = false;
        FormLook.Say(_refusal, null);
        var refused = await DeviceLink.Call(request);
        _busy = false;
        if (_button is not null) _button.IsEnabled = true;
        FormLook.Say(_refusal, refused?.Message);
        if (refused is not null) return;
        if (_success.Length > 0) ctx.Say(_success);
        _done?.Invoke();
    }
}

internal sealed class SwitchBox : UserControl
{
    private readonly TextBlock _title = FormLook.Text("");
    private readonly TextBlock _detail = FormLook.Text("", "FontSizeBody", "TextSecondaryBrush");
    private readonly ToggleSwitch _toggle = new() { OnContent = "", OffContent = "", MinWidth = 0 };
    private readonly TextBlock _refusal = Kit.Refusal();
    private DeviceContext? _ctx;
    private (DeviceRequest On, DeviceRequest Off, string Success, string OffSuccess, Confirm? ConfirmOn)? _wiring;
    private bool _device;
    private bool _busy;
    private bool _quiet;

    public SwitchBox()
    {
        _title.Visibility = Visibility.Visible;
        var text = FormLook.Stack(2, _title, _detail);
        text.VerticalAlignment = VerticalAlignment.Center;
        _toggle.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(_toggle, 1);
        var line = new Grid
        {
            Padding = new Thickness(16, 8, 16, 8),
            ColumnSpacing = 16,
            ColumnDefinitions = { new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
            Children = { text, _toggle },
        };
        _toggle.Toggled += (_, _) => Flip();
        Content = FormLook.Stack(0, line, FormLook.Margined(_refusal, new Thickness(16, 0, 16, 8)));
    }

    public void Show(DeviceContext ctx, string title, bool on, DeviceRequest onRequest, DeviceRequest offRequest, string detail, string success, string offSuccess, bool enabled, Confirm? confirmOn)
    {
        _ctx = ctx;
        _wiring = (onRequest, offRequest, success, offSuccess, confirmOn);
        _title.Text = title;
        AutomationProperties.SetName(_toggle, title);
        FormLook.Say(_detail, detail);
        _device = on;
        _toggle.IsEnabled = enabled && !_busy;
        if (!_busy) Set(on);
    }

    private void Set(bool on)
    {
        _quiet = true;
        _toggle.IsOn = on;
        _quiet = false;
    }

    private async void Flip()
    {
        if (_quiet || _busy || _wiring is not { } wiring || _ctx is not { } ctx || _toggle.IsOn == _device) return;
        var next = _toggle.IsOn;
        if (next && !await Kit.Ask(XamlRoot, wiring.ConfirmOn))
        {
            Set(_device);
            return;
        }
        _busy = true;
        _toggle.IsEnabled = false;
        FormLook.Say(_refusal, null);
        var refused = await DeviceLink.Call(next ? wiring.On : wiring.Off);
        _busy = false;
        _toggle.IsEnabled = true;
        FormLook.Say(_refusal, refused?.Message);
        if (refused is not null) Set(_device);
        else if ((next ? wiring.Success : wiring.OffSuccess) is { Length: > 0 } said) ctx.Say(said);
    }
}

internal sealed class FormBox : UserControl
{
    private readonly DeviceContext _ctx;
    private readonly IReadOnlyList<Field> _fields;
    private readonly string _method;
    private readonly string _path;
    private readonly string _success;
    private readonly Confirm? _confirm;
    private readonly IReadOnlyDictionary<string, string> _extra;
    private readonly Action? _done;
    private readonly IReadOnlyDictionary<string, (FrameworkElement Input, TextBlock Said)> _inputs;
    private readonly TextBlock _refusal = Kit.Refusal();
    private readonly Button _submit;
    private ImmutableDictionary<string, string> _values = ImmutableDictionary<string, string>.Empty;

    public FormBox(DeviceContext ctx, IReadOnlyList<Field> fields, string method, string path, string submit, string success, Confirm? confirm, string note,
        IReadOnlyDictionary<string, string> extra, Action? done)
    {
        (_ctx, _fields, _method, _path, _success, _confirm, _extra, _done) = (ctx, fields, method, path, success, confirm, extra, done);
        _inputs = fields.ToDictionary(field => field.Name, field => (Input(field), FormLook.Caption(field.Hint, "TextTertiaryBrush")));
        _submit = FormLook.Button(submit, Send, ButtonLook.Standard);
        _submit.HorizontalAlignment = HorizontalAlignment.Left;
        Content = FormLook.Margined(FormLook.Stack(12, fields
            .Select(field => (UIElement)FormLook.Stack(4, _inputs[field.Name].Input, _inputs[field.Name].Said))
            .Append(FormLook.Caption(note, "TextTertiaryBrush"))
            .Append(_refusal)
            .Append(_submit)), Kit.Pad);
        Reveal();
    }

    public IReadOnlyDictionary<string, string> Current => _fields.ToDictionary(field => field.Name, field => _values.GetValueOrDefault(field.Name, field.Value));

    private FrameworkElement Input(Field field)
    {
        if (field.Options is { } options) return Kit.Select(field.Label, field.Value, options, value => Typed(field, value));
        if (field.Kind == FieldKind.Password)
        {
            var secret = new PasswordBox { Header = field.Label, PlaceholderText = field.Placeholder, Password = field.Value, FontFamily = PreflightLook.Font };
            AutomationProperties.SetName(secret, field.Label);
            secret.PasswordChanged += (_, _) => Typed(field, secret.Password);
            return secret;
        }
        var box = FormLook.Field(field.Value, field.Label, field.Placeholder);
        AutomationProperties.SetName(box, field.Label);
        if (field.Lines > 1)
        {
            box.AcceptsReturn = true;
            box.TextWrapping = TextWrapping.Wrap;
            box.MinHeight = 24 * field.Lines;
        }
        box.TextChanged += (_, _) => Typed(field, box.Text);
        return box;
    }

    private void Typed(Field field, string value)
    {
        _values = _values.SetItem(field.Name, value);
        Reveal();
    }

    private void Reveal()
    {
        var current = Current;
        _fields.ToList().ForEach(field =>
        {
            var shown = field.ShownWhen?.Invoke(current) ?? true;
            _inputs[field.Name].Input.Visibility = FormLook.Shown(shown);
            _inputs[field.Name].Said.Visibility = FormLook.Shown(shown && _inputs[field.Name].Said.Text.Length > 0);
        });
    }

    private void Refuse(DeviceRefusal? refusal)
    {
        var shownNames = _fields.Where(field => _inputs[field.Name].Input.Visibility == Visibility.Visible).Select(field => field.Name).ToHashSet();
        _fields.ToList().ForEach(field =>
        {
            var said = _inputs[field.Name].Said;
            var mine = refusal is not null && refusal.Field == field.Name;
            said.Text = mine ? refusal!.Message : field.Hint;
            said.Foreground = FormLook.Brush(mine ? "CriticalBrush" : "TextTertiaryBrush");
        });
        FormLook.Say(_refusal, refusal is not null && !shownNames.Contains(refusal.Field) ? refusal.Message : null);
        Reveal();
    }

    private async void Send()
    {
        if (!await Kit.Ask(XamlRoot, _confirm)) return;
        var body = Current.Concat(_extra.Where(kv => !Current.ContainsKey(kv.Key))).ToList();
        _submit.IsEnabled = false;
        Refuse(null);
        var refused = await DeviceLink.Call(new DeviceRequest(_method, _path, DeviceLink.Body(body)));
        _submit.IsEnabled = true;
        Refuse(refused);
        if (refused is not null) return;
        if (_success.Length > 0) _ctx.Say(_success);
        _done?.Invoke();
    }
}

internal sealed class ChartBox : UserControl
{
    private readonly TextBlock _label = FormLook.Text("", "FontSizeBody", "TextSecondaryBrush");
    private readonly TextBlock _value = FormLook.Text("", "FontSizeBodyLarge");
    private readonly Polyline _line = new() { StrokeThickness = 2 };
    private readonly Grid _plot = new() { Height = 40 };
    private Chart? _chart;

    public ChartBox()
    {
        _label.Visibility = Visibility.Visible;
        _value.Visibility = Visibility.Visible;
        _plot.Children.Add(_line);
        _plot.SizeChanged += (_, _) => Draw();
        Grid.SetColumn(_plot, 1);
        var words = FormLook.Stack(2, _label, _value);
        words.Width = 112;
        words.VerticalAlignment = VerticalAlignment.Center;
        Content = new Grid
        {
            Margin = Kit.Pad,
            ColumnSpacing = 16,
            ColumnDefinitions = { new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition() },
            Children = { words, _plot },
        };
    }

    public void Show(Chart chart)
    {
        _chart = chart;
        _label.Text = chart.Label;
        _value.Text = chart.Value;
        _value.Foreground = FormLook.Brush(Kit.LevelTextBrush(chart.Level));
        _line.Stroke = FormLook.Brush(chart.Level == DeviceLevel.Neutral ? "AccentBrush" : Kit.LevelBrush(chart.Level));
        AutomationProperties.SetName(_plot, $"{chart.Label} history");
        Draw();
    }

    public static IReadOnlyList<Windows.Foundation.Point> Points(IReadOnlyList<double> values, int max, double width, double height) =>
        values.Count < 2 ? [] : values.Select((value, at) => new Windows.Foundation.Point(
            at / (double)(values.Count - 1) * width,
            height - Math.Clamp(value, 0, max) / (max > 0 ? max : 1) * height)).ToList();

    private void Draw()
    {
        if (_chart is null) return;
        var points = new PointCollection();
        Points(_chart.Values, _chart.Max, _plot.ActualWidth, _plot.ActualHeight).ToList().ForEach(points.Add);
        _line.Points = points;
    }
}
