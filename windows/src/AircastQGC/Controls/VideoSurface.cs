using System;
using System.Runtime.InteropServices.WindowsRuntime;
using AircastQGC.Core;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Foundation;

namespace AircastQGC.Controls;

public sealed partial class VideoSurface : UserControl
{
    public const int FitWidth = 0;
    public const int FitHeight = 1;
    public const int Fill = 2;
    public const int NoCrop = 3;

    private static readonly TimeSpan Tick = TimeSpan.FromMilliseconds(33);

    private readonly Image _image = new() { Stretch = Stretch.Fill };
    private readonly Canvas _canvas = new();
    private DispatcherQueueTimer? _timer;
    private WriteableBitmap? _bitmap;
    private byte[] _scratch = Array.Empty<byte>();
    private long _shown = -1;
    private int _fit = NoCrop;

    public VideoSurface()
    {
        _canvas.Children.Add(_image);
        Content = _canvas;
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
        SizeChanged += (_, _) => Place();
    }

    public int Channel { get; set; }

    public int Fit
    {
        get => _fit;
        set
        {
            _fit = value;
            Place();
        }
    }

    internal static (double Width, double Height) ContentSize(double width, double height, double aspect, int fit)
    {
        if (aspect <= 0 || width <= 0 || height <= 0) return (width, height);
        var box = width / height;
        var shownWidth = fit == FitHeight || (fit == Fill && box < aspect) || (fit == NoCrop && box > aspect) ? height * aspect : width;
        var shownHeight = fit == FitWidth || (fit == Fill && box > aspect) || (fit == NoCrop && box < aspect) ? width / aspect : height;
        return (shownWidth, shownHeight);
    }

    private void Start()
    {
        if (_timer != null || !VideoNative.Ready) return;
        _timer = DispatcherQueue.CreateTimer();
        _timer.Interval = Tick;
        _timer.Tick += (_, _) => Draw();
        _timer.Start();
    }

    private void Stop()
    {
        _timer?.Stop();
        _timer = null;
    }

    private void Place()
    {
        var (width, height) = (ActualWidth, ActualHeight);
        _canvas.Clip = new RectangleGeometry { Rect = new Rect(0, 0, Math.Max(width, 0), Math.Max(height, 0)) };
        if (_bitmap is not { } bitmap || width <= 0 || height <= 0) return;
        var (shownWidth, shownHeight) = ContentSize(width, height, (double)bitmap.PixelWidth / bitmap.PixelHeight, _fit);
        _image.Width = shownWidth;
        _image.Height = shownHeight;
        Canvas.SetLeft(_image, (width - shownWidth) / 2);
        Canvas.SetTop(_image, (height - shownHeight) / 2);
    }

    private unsafe void Draw()
    {
        var frames = VideoNative.qgc_video_frames(Channel);
        var (width, height) = (VideoNative.qgc_video_width(Channel), VideoNative.qgc_video_height(Channel));
        if (frames == _shown || width <= 0 || height <= 0) return;
        if (_scratch.Length < width * height * 4) _scratch = new byte[width * height * 4];

        bool copied;
        int w, h, stride;
        fixed (byte* destination = _scratch)
            copied = VideoNative.qgc_video_copy_frame(Channel, destination, _scratch.Length, out w, out h, out stride);
        var size = w * h * 4;
        if (!copied || stride != w * 4 || size > _scratch.Length) return;

        if (_bitmap is not { } bitmap || bitmap.PixelWidth != w || bitmap.PixelHeight != h)
        {
            _bitmap = new WriteableBitmap(w, h);
            _image.Source = _bitmap;
            Place();
        }
        _scratch.CopyTo(0, _bitmap.PixelBuffer, 0, size);
        _bitmap.Invalidate();
        _shown = frames;
    }
}
