using System;
using System.Collections.Generic;
using System.Linq;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Windows.Foundation;

namespace AircastQGC.Controls;

internal sealed class SyntheticSurface
{
    private const double RenderScale = 0.75;
    private const double SoftwarePixels = 640 * 360;

    private readonly SwapChainPanel _panel = new();
    private readonly ScaleTransform _scale = new();
    private readonly int _orbit;
    private IntPtr _surface;
    private (int Width, int Height) _attached;

    public SyntheticSurface(bool orbit)
    {
        _orbit = orbit ? 1 : 0;
        _panel.RenderTransform = _scale;
        Stage.Children.Add(_panel);
    }

    public Canvas Stage { get; } = new();

    public bool BindFailed { get; private set; }

    public bool Sized => _panel.Width >= 1 && _panel.Height >= 1;

    public void Fit(double width, double height, bool software, double rasterization)
    {
        var scale = software
            ? Math.Min(1, Math.Sqrt(SoftwarePixels / Math.Max(width * height, 1)))
            : RenderScale * rasterization;
        Stage.Clip = new RectangleGeometry { Rect = new Rect(0, 0, Math.Max(width, 0), Math.Max(height, 0)) };
        _panel.Width = Math.Max(1, Math.Round(width * scale));
        _panel.Height = Math.Max(1, Math.Round(height * scale));
        _scale.ScaleX = width > 0 ? width / _panel.Width : 1;
        _scale.ScaleY = height > 0 ? height / _panel.Height : 1;
    }

    public void Hold(bool attachable)
    {
        if (attachable) Attach();
        else Detach();
    }

    public void Detach()
    {
        if (_surface == IntPtr.Zero) return;
        var surface = _surface;
        _surface = IntPtr.Zero;
        _attached = default;
        NativeMethods.qgc_synthetic_release(surface);
        CompositionSurface.Close(surface);
    }

    public static bool OnScreen(FrameworkElement view) =>
        view.IsLoaded && view.XamlRoot?.IsHostVisible == true && Lineage(view).All(e => e.Visibility == Visibility.Visible);

    private void Attach()
    {
        var size = ((int)_panel.Width, (int)_panel.Height);
        if (_surface != IntPtr.Zero && size == _attached) return;
        _surface = _surface != IntPtr.Zero ? _surface : CompositionSurface.Bind(_panel);
        BindFailed = _surface == IntPtr.Zero;
        if (BindFailed) return;
        _attached = size;
        NativeMethods.qgc_synthetic_attach(_surface, size.Item1, size.Item2, _orbit);
    }

    private static IEnumerable<UIElement> Lineage(DependencyObject? from) =>
        from is null ? [] : (from is UIElement element ? [element] : Array.Empty<UIElement>()).Concat(Lineage(VisualTreeHelper.GetParent(from)));
}
