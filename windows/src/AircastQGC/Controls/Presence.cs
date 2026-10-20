using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Linq;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Controls;

internal static class Presence
{
    public static void OnHidden(FrameworkElement element, Action hidden)
    {
        var held = ImmutableList<(UIElement Ancestor, long Token)>.Empty;
        element.Loaded += (_, _) =>
        {
            Drop(held);
            held = Ancestors(element).OfType<UIElement>()
                .Select(ancestor => (ancestor, ancestor.RegisterPropertyChangedCallback(UIElement.VisibilityProperty, (_, _) =>
                {
                    if (ancestor.Visibility == Visibility.Collapsed) hidden();
                })))
                .ToImmutableList();
        };
        element.Unloaded += (_, _) =>
        {
            Drop(held);
            held = held.Clear();
        };
    }

    private static void Drop(ImmutableList<(UIElement Ancestor, long Token)> held) =>
        held.ForEach(watch => watch.Ancestor.UnregisterPropertyChangedCallback(UIElement.VisibilityProperty, watch.Token));

    private static IEnumerable<DependencyObject> Ancestors(DependencyObject element) =>
        VisualTreeHelper.GetParent(element) is { } parent ? Ancestors(parent).Prepend(parent) : [];
}
