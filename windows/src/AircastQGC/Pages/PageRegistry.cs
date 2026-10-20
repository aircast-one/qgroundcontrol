using Microsoft.UI.Xaml;

namespace AircastQGC.Pages;

internal static class PageRegistry
{
    internal static UIElement? Create(string page) => page switch
    {
        _ => null,
    };
}
