using System;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Documents;

namespace AircastQGC.Controls;

internal sealed record LinkedPart(string Text, string? Url, bool Break);

internal static partial class ParameterLinks
{
    private const string Scheme = "param://";
    private const string DialogOpened = "setup.dialogOpened";
    private const string EditorDialog = "parameterEditor";

    [GeneratedRegex("<a\\s[^>]*?href=\"([^\"]*)\"[^>]*>(.*?)</a>|<br\\s*/?>|<[^>]*>", RegexOptions.IgnoreCase | RegexOptions.Singleline)]
    private static partial Regex Markup();

    public static string? Name(string url) =>
        url.StartsWith(Scheme, StringComparison.OrdinalIgnoreCase) && url[Scheme.Length..].Trim() is { Length: > 0 } name ? name : null;

    public static bool Opens(string name) => MapUiJson.Flag(Bridge.Invoke(DialogOpened, new JsonArray(EditorDialog, name))["exists"], true);

    public static IReadOnlyList<LinkedPart> Parts(string html)
    {
        var tags = Markup().Matches(html).Cast<Match>().ToList();
        var starts = tags.Select(tag => tag.Index + tag.Length).Prepend(0).ToList();
        var ends = tags.Select(tag => tag.Index).Append(html.Length).ToList();
        return starts.Zip(ends, (from, to) => Plain(html[from..to]))
            .Zip(tags.Select(Tag).Append(null), (text, tag) => new[] { text, tag })
            .SelectMany(pair => pair)
            .OfType<LinkedPart>()
            .ToList();
    }

    public static TextBlock Text(string html, Action<string> onParameter, string brush = "TextSecondaryBrush") =>
        Fill(Ui.Caption("", brush), html, onParameter);

    public static TextBlock Fill(TextBlock block, string html, Action<string> onParameter)
    {
        block.Text = "";
        block.TextWrapping = Microsoft.UI.Xaml.TextWrapping.Wrap;
        Parts(html).Select(part => Piece(part, onParameter)).ToList().ForEach(block.Inlines.Add);
        return block;
    }

    private static LinkedPart? Plain(string text) => text.Length == 0 ? null : new LinkedPart(WebUtility.HtmlDecode(text), null, false);

    private static LinkedPart? Tag(Match tag) =>
        tag.Groups[1].Success
            ? new LinkedPart(WebUtility.HtmlDecode(Regex.Replace(tag.Groups[2].Value, "<[^>]*>", "")), WebUtility.HtmlDecode(tag.Groups[1].Value), false)
            : tag.Value.StartsWith("<br", StringComparison.OrdinalIgnoreCase) ? new LinkedPart("", null, true) : null;

    private static Inline Piece(LinkedPart part, Action<string> onParameter)
    {
        if (part.Break) return new LineBreak();
        if (part.Url is not { } url) return new Run { Text = part.Text };
        var link = new Hyperlink { Inlines = { new Run { Text = part.Text.Length > 0 ? part.Text : url } } };
        link.Click += (_, _) =>
        {
            if (Name(url) is { } name) Ui.Off(link, () => Opens(name), opens => { if (opens) onParameter(name); });
            else if (Uri.TryCreate(url, UriKind.Absolute, out var target)) _ = Windows.System.Launcher.LaunchUriAsync(target);
        };
        return link;
    }
}
