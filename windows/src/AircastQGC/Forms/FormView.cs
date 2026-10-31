using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Forms;

public sealed record FormCalculatorRequest(FormControl Control, JsonObject Calculator);

public sealed record FormHooks
{
    public Func<FormView, UIElement?>? Top { get; init; }
    public Func<FormView, FormSection, UIElement?>? Replace { get; init; }
    public Func<FormView, FormSection, FormBlock, UIElement?>? AfterBlock { get; init; }
    public Func<FormView, FormSection, UIElement?>? AfterSection { get; init; }
    public Func<FormView, UIElement?>? Bottom { get; init; }
}

public sealed class FormView : UserControl
{
    public const string SetupPageOpened = "setup.pageOpened";

    public static readonly IReadOnlyDictionary<string, string> EmptySetupTexts = new Dictionary<string, string>
    {
        ["Gimbal"] = "Gimbal settings are not available for this firmware version.",
    };

    private readonly Func<JsonObject> _read;
    private readonly Func<JsonObject, IReadOnlyList<FormSection>> _parse;
    private readonly StackPanel _body = new();
    private ImmutableDictionary<string, Control> _focusable = ImmutableDictionary<string, Control>.Empty;
    private ImmutableHashSet<string> _openAdvanced = ImmutableHashSet<string>.Empty;
    private string? _signature;
    private string? _refocus;
    private int _version;

    public FormView(Func<JsonObject> read, Func<JsonObject, IReadOnlyList<FormSection>> parse)
    {
        _read = read;
        _parse = parse;
        Content = _body;
        Loaded += (_, _) =>
        {
            if (_signature is null) FormLook.Fill(_body, [FormLook.Footnote(LoadingText)]);
            Reload();
        };
    }

    public event EventHandler<FormControl>? DialogRequested;

    public event EventHandler<FormCalculatorRequest>? CalculatorRequested;

    public event EventHandler<FormControl>? Rejected;

    public FormHooks Hooks { get; init; } = new();
    public string PageTitle { get; init; } = "";
    public string LoadingText { get; init; } = "Reading settings.";
    public string EmptyText { get; init; } = "No settings exposed here.";
    public Func<UIElement>? Empty { get; init; }
    public bool NotesFirst { get; init; }
    public IReadOnlySet<string> Highlighted { get; init; } = ImmutableHashSet<string>.Empty;
    public IReadOnlyList<FormSection> Sections { get; private set; } = [];

    internal bool HasCalculator => CalculatorRequested is not null;

    public static FormView SetupPage(string page, string? section = null, IReadOnlySet<string>? highlighted = null)
    {
        var opened = new Lazy<bool>(() => FormWrites.Ok(SetupPageOpened, page));
        return new FormView(
            () =>
            {
                _ = opened.Value;
                return Bridge.Get($"view.setup({page})");
            },
            raw => FormParse.SetupSections(raw).Where(s => section is null || s.Title == section).ToList())
        {
            LoadingText = "Reading parameters from the vehicle.",
            NotesFirst = true,
            Highlighted = highlighted ?? ImmutableHashSet<string>.Empty,
            Empty = () => FormLook.Empty("", "Nothing to set here",
                EmptySetupTexts.TryGetValue(page, out var text) ? text : "This vehicle exposes none of these parameters."),
        };
    }

    public async void Reload()
    {
        var version = ++_version;
        var raw = await FormWrites.Off(_read);
        if (version != _version) return;
        var signature = raw.ToJsonString();
        if (signature == _signature)
        {
            Refocus(null);
            return;
        }
        _signature = signature;
        Sections = _parse(raw);
        Render();
    }

    public UIElement BlockCard(FormBlock block) => FormLook.Stack(4,
        FormLook.Caption(block.SharedRebootNote ?? "", "CautionBrush"),
        FormLook.Card(Runs(block.Controls).Select(run => run.Count > 1 ? FormRows.Labels(run) : new FormRows(this).Row(run[0], block))),
        FormLook.Caption(block.SharedInertNote ?? "", "TextTertiaryBrush"));

    private static IEnumerable<ImmutableList<FormControl>> Runs(IEnumerable<FormControl> controls) =>
        controls.Aggregate(ImmutableList<ImmutableList<FormControl>>.Empty, (runs, c) =>
            c.IsLabel && runs.LastOrDefault() is { } last && last[^1].IsLabel
                ? runs.SetItem(runs.Count - 1, last.Add(c))
                : runs.Add(ImmutableList.Create(c)));

    internal void Track(string key, Control control) => _focusable = _focusable.SetItem(key, control);

    internal void Written(string key)
    {
        _refocus = key;
        Reload();
    }

    internal void RequestDialog(FormControl control) => DialogRequested?.Invoke(this, control);

    internal void Refused(FormControl control) => Rejected?.Invoke(this, control);

    internal void RequestCalculator(FormControl control, JsonObject calculator) =>
        CalculatorRequested?.Invoke(this, new FormCalculatorRequest(control, calculator));

    private void Render()
    {
        var offset = Scroller()?.VerticalOffset;
        _focusable = ImmutableDictionary<string, Control>.Empty;
        FormLook.Fill(_body, Sections.Count == 0
            ? [Empty?.Invoke() ?? FormLook.Footnote(EmptyText)]
            : new[] { Hooks.Top?.Invoke(this) }
                .Concat(Sections.SelectMany(SectionView))
                .Append(Hooks.Bottom?.Invoke(this)));
        Refocus(offset);
    }

    private void Refocus(double? offset)
    {
        var key = _refocus;
        _refocus = null;
        DispatcherQueue.TryEnqueue(() =>
        {
            if (offset is { } y) Scroller()?.ChangeView(null, y, null, true);
            if (key is not null && _focusable.TryGetValue(key, out var control)) control.Focus(FocusState.Programmatic);
        });
    }

    private IEnumerable<UIElement?> SectionView(FormSection section) =>
        Hooks.Replace?.Invoke(this, section) is { } replaced
            ? [FormLook.Heading(MapUiJson.SentenceCase(section.Title)), replaced, Hooks.AfterSection?.Invoke(this, section)]
            : section.Blocks.SelectMany((block, at) => BlockView(section, block, at == 0))
                .Append(NotesFirst ? null : FormLook.Footnote(section.Note))
                .Append(Hooks.AfterSection?.Invoke(this, section));

    private IEnumerable<UIElement?> BlockView(FormSection section, FormBlock block, bool first) =>
        block.Controls.Count == 0
            ? [Hooks.AfterBlock?.Invoke(this, section, block)]
            : block.Title == FormBlock.Advanced
                ? [Advanced(section, block), Hooks.AfterBlock?.Invoke(this, section, block)]
                :
                [
                    Heading(MapUiJson.SentenceCase(BlockHeading(section, block)), first ? section.Image : ""),
                    NotesFirst && first ? FormLook.Margined(FormLook.Caption(section.Note), new Thickness(0, 0, 0, 8)) : null,
                    BlockCard(block),
                    Hooks.AfterBlock?.Invoke(this, section, block),
                ];

    private static UIElement Heading(string text, string image)
    {
        if (text.Length == 0 || image.Length == 0) return FormLook.Heading(text);
        var title = FormLook.Text(text, "FontSizeBody", "TextPrimaryBrush", true);
        title.VerticalAlignment = VerticalAlignment.Center;
        var picture = AssetImages.Svg(AssetImages.SetupSections, image, 24, AssetImages.Hex("TextPrimaryBrush"), 48);
        return FormLook.Margined(FormLook.Lead(12, picture, title), new Thickness(0, 24, 0, 8));
    }

    private string BlockHeading(FormSection section, FormBlock block) =>
        block.Title.Length > 0 ? block.Title
        : !section.Title.Equals(PageTitle, StringComparison.OrdinalIgnoreCase) ? section.Title
        : "";

    private UIElement Advanced(FormSection section, FormBlock block)
    {
        var key = $"{section.Group}#{block.Title}";
        var expander = new Expander
        {
            Header = FormLook.Text(FormBlock.Advanced, "FontSizeBody", "TextPrimaryBrush", true),
            Content = BlockCard(block),
            IsExpanded = _openAdvanced.Contains(key),
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Margin = new Thickness(0, 24, 0, 0),
        };
        expander.Expanding += (_, _) => _openAdvanced = _openAdvanced.Add(key);
        expander.Collapsed += (_, _) => _openAdvanced = _openAdvanced.Remove(key);
        return expander;
    }

    private ScrollViewer? Scroller() => Ancestors(this).OfType<ScrollViewer>().FirstOrDefault();

    private static IEnumerable<DependencyObject> Ancestors(DependencyObject start) =>
        VisualTreeHelper.GetParent(start) is { } parent ? Ancestors(parent).Prepend(parent) : [];
}
