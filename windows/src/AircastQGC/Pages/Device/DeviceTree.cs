using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.CompilerServices;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Device;

internal sealed record Node(string Key, Func<UIElement?, UIElement> Render);

internal static class DeviceTree
{
    private static readonly ConditionalWeakTable<UIElement, string> Keys = new();
    private static readonly ConditionalWeakTable<UIElement, string> Signatures = new();

    public static Node Leaf(string key, string signature, Func<UIElement> build) =>
        new(key, previous => previous is not null && Signatures.TryGetValue(previous, out var held) && held == signature
            ? previous
            : Signed(build(), signature));

    public static Node Kept<T>(string key, Func<T> make, Action<T> show) where T : UIElement =>
        new(key, previous =>
        {
            var element = previous as T ?? make();
            show(element);
            return element;
        });

    public static void Sync(Panel panel, IEnumerable<Node> nodes)
    {
        var listed = nodes.ToList();
        var qualified = listed.Select((node, at) => $"{node.Key}#{listed.Take(at).Count(other => other.Key == node.Key)}").ToList();
        var held = panel.Children
            .Select(child => (Key: Keys.TryGetValue(child, out var key) ? key : null, Child: child))
            .Where(pair => pair.Key is not null)
            .GroupBy(pair => pair.Key!)
            .ToDictionary(group => group.Key, group => group.First().Child);
        var next = listed.Select((node, at) => Keyed(node.Render(held.GetValueOrDefault(qualified[at])), qualified[at])).ToList();
        panel.Children.Where(child => !next.Contains(child)).ToList().ForEach(child => panel.Children.Remove(child));
        Enumerable.Range(0, next.Count).ToList().ForEach(at => Place(panel, next[at], at));
    }

    private static UIElement Signed(UIElement element, string signature)
    {
        Signatures.AddOrUpdate(element, signature);
        return element;
    }

    private static UIElement Keyed(UIElement element, string key)
    {
        Keys.AddOrUpdate(element, key);
        return element;
    }

    private static void Place(Panel panel, UIElement element, int at)
    {
        var now = panel.Children.IndexOf(element);
        if (now == at) return;
        if (now >= 0) panel.Children.Move((uint)now, (uint)at);
        else panel.Children.Insert(at, element);
    }
}

internal sealed class FlowPanel : Panel
{
    public double Spacing { get; set; } = 8;

    protected override Windows.Foundation.Size MeasureOverride(Windows.Foundation.Size available)
    {
        Children.ToList().ForEach(child => child.Measure(new Windows.Foundation.Size(available.Width, double.PositiveInfinity)));
        var lines = Lines(available.Width);
        return new Windows.Foundation.Size(
            lines.Count == 0 ? 0 : lines.Max(line => line.Width),
            lines.Sum(line => line.Height) + Spacing * Math.Max(0, lines.Count - 1));
    }

    protected override Windows.Foundation.Size ArrangeOverride(Windows.Foundation.Size final)
    {
        Lines(final.Width).Aggregate(0.0, (top, line) =>
        {
            line.Items.Aggregate(0.0, (left, child) =>
            {
                child.Arrange(new Windows.Foundation.Rect(left, top, child.DesiredSize.Width, line.Height));
                return left + child.DesiredSize.Width + Spacing;
            });
            return top + line.Height + Spacing;
        });
        return final;
    }

    private sealed record Line(IReadOnlyList<UIElement> Items, double Width, double Height);

    private List<Line> Lines(double width) =>
        Children.Aggregate(new List<Line>(), (lines, child) =>
        {
            var last = lines.LastOrDefault();
            var size = child.DesiredSize;
            var fits = last is not null && (double.IsInfinity(width) || last.Width + Spacing + size.Width <= width);
            return fits
                ? lines.Take(lines.Count - 1).Append(new Line([.. last!.Items, child], last.Width + Spacing + size.Width, Math.Max(last.Height, size.Height))).ToList()
                : lines.Append(new Line([child], size.Width, size.Height)).ToList();
        });
}
