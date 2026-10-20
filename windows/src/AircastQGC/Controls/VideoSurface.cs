using System;
using System.Runtime.InteropServices.WindowsRuntime;
using AircastQGC.Core;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;

namespace AircastQGC.Controls;

public sealed partial class VideoSurface : UserControl
{
    private static readonly TimeSpan Tick = TimeSpan.FromMilliseconds(33);

    private readonly Image _image = new() { Stretch = Stretch.Uniform };
    private DispatcherQueueTimer? _timer;
    private WriteableBitmap? _bitmap;
    private byte[] _scratch = Array.Empty<byte>();
    private long _shown = -1;

    public VideoSurface()
    {
        Content = _image;
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public int Channel { get; set; }

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
        }
        _scratch.CopyTo(0, _bitmap.PixelBuffer, 0, size);
        _bitmap.Invalidate();
        _shown = frames;
    }
}
