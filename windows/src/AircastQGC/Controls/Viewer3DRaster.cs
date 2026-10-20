using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

namespace AircastQGC.Controls;

internal sealed class Raster3D
{
    private const float Near = 1;
    private const float Ambient = 0.62f;
    private const float Diffuse = 0.38f;
    private const float GridAlpha = 0.35f;
    private const int GridLines = 12;
    private const float MaxHaze = 0.85f;
    private const int MinZoom = 3;
    private const int MaxZoom = 19;
    private const int FallbackLevels = 6;
    private const int TileShift = 8;
    private const int FixedShift = 16;
    private const double Fixed = 1 << FixedShift;
    private const int MaxBands = 8;
    private static readonly Vector3 Light = Vector3.Normalize(new Vector3(-0.3f, -0.6f, 0.75f));
    private static readonly float[] GridSteps = [10, 20, 50, 100, 200, 500, 1000, 2000, 5000];

    private sealed class Band(int top, int bottom)
    {
        public readonly int Top = top;
        public readonly int Bottom = bottom;
        public float[] Crossings = new float[128];
    }

    private struct Shape
    {
        public int Count;
        public int Rings;
        public float Top;
        public float Bottom;
        public float A;
        public float B;
        public float C;
        public uint Colour;
    }

    private sealed class Layout
    {
        public readonly IReadOnlyList<Face3D> Faces;
        public readonly int[] PointStart;
        public readonly int[] RingStart;
        public readonly Vector2[] Screen;
        public readonly int[] Ends;
        public readonly Shape[] Shapes;

        public Layout(IReadOnlyList<Face3D> faces)
        {
            Faces = faces;
            PointStart = Starts(faces.Select(face => face.Points.Length * 2));
            RingStart = Starts(faces.Select(face => face.Ends.Length));
            Screen = new Vector2[faces.Sum(face => face.Points.Length * 2)];
            Ends = new int[faces.Sum(face => face.Ends.Length)];
            Shapes = new Shape[faces.Count];
        }

        private static int[] Starts(IEnumerable<int> sizes)
        {
            var each = sizes.ToArray();
            var starts = new int[each.Length];
            for (var i = 1; i < each.Length; i++) starts[i] = starts[i - 1] + each[i - 1];
            return starts;
        }
    }

    [ThreadStatic] private static Vector3[]? _cameraScratch;
    [ThreadStatic] private static Vector3[]? _ringScratch;

    private float[] _depth = [];
    private float[] _rowHaze = [];
    private Band[] _bands = [];
    private Layout[] _layouts = [];
    private Vector3 _eye;
    private Vector3 _right;
    private Vector3 _up;
    private Vector3 _forward;
    private float _focal;
    private float _hazeFrom;
    private float _hazeTo;

    public int Width { get; private set; }
    public int Height { get; private set; }
    public uint[] Pixels { get; private set; } = [];

    public void Resize(int width, int height)
    {
        if (width == Width && height == Height) return;
        Width = width;
        Height = height;
        Pixels = new uint[width * height];
        _depth = new float[width * height];
        _rowHaze = new float[height];
        var count = Math.Clamp(Environment.ProcessorCount, 1, MaxBands);
        var size = (height + count - 1) / count;
        _bands = Enumerable.Range(0, count).Select(at => new Band(at * size, Math.Min(height, (at + 1) * size))).ToArray();
    }

    public Vector2? Project(Vector3 point)
    {
        var at = ToCamera(point);
        return at.Z < Near ? null : Screen(at);
    }

    public void Draw(Camera3D camera, Palette3D palette, Ground3D? ground, params IReadOnlyList<Face3D>[] layers)
    {
        if (Width == 0 || Height == 0) return;
        _eye = camera.Eye;
        _right = camera.Right;
        _up = camera.Up;
        _forward = camera.Forward;
        _focal = Camera3D.Focal(Height);
        _hazeFrom = MathF.Max(camera.Distance * 1.5f, 800);
        _hazeTo = _hazeFrom * 6;
        Parallel.For(0, Height, y => Background(y, palette, ground));
        if (ground is null) Grid(camera, palette.Grid);
        _layouts = layers.Select(faces => _layouts.FirstOrDefault(known => ReferenceEquals(known.Faces, faces)) ?? new Layout(faces)).ToArray();
        foreach (var layout in _layouts.Where(layout => layout.Faces.Count > 0))
            Parallel.ForEach(Partitioner.Create(0, layout.Faces.Count, Math.Max(256, layout.Faces.Count / 64)), range =>
            {
                for (var i = range.Item1; i < range.Item2; i++) Project(layout, i, palette.Sky);
            });
        Array.Clear(_depth);
        Parallel.ForEach(_bands, band =>
        {
            foreach (var layout in _layouts)
                for (var i = 0; i < layout.Shapes.Length; i++)
                    if (layout.Shapes[i].Count >= 3 && layout.Shapes[i].Bottom >= band.Top && layout.Shapes[i].Top <= band.Bottom) Fill(band, layout, i);
        });
    }

    private Vector3 ToCamera(Vector3 point)
    {
        var d = point - _eye;
        return new Vector3(Vector3.Dot(d, _right), Vector3.Dot(d, _up), Vector3.Dot(d, _forward));
    }

    private Vector2 Screen(Vector3 at) => new(Width / 2f + _focal * at.X / at.Z, Height / 2f - _focal * at.Y / at.Z);

    private float Haze(float distance) => Math.Clamp((distance - _hazeFrom) / (_hazeTo - _hazeFrom), 0, MaxHaze);

    private static uint Mix(uint from, uint to, float t)
    {
        var a = (uint)(t * 256);
        var rb = (((from & 0xFF00FF) * (256 - a) + (to & 0xFF00FF) * a) >> 8) & 0xFF00FF;
        var g = (((from & 0x00FF00) * (256 - a) + (to & 0x00FF00) * a) >> 8) & 0x00FF00;
        return 0xFF000000 | rb | g;
    }

    private static uint Shaded(uint colour, float factor)
    {
        uint Channel(int shift) => (uint)Math.Min(255, ((colour >> shift) & 0xFF) * factor) << shift;
        return 0xFF000000 | Channel(16) | Channel(8) | Channel(0);
    }

    private void Background(int y, Palette3D palette, Ground3D? ground)
    {
        var ray = _forward + _up * ((Height / 2f - (y + 0.5f)) / _focal);
        var below = ray.Z < 0 && _eye.Z > 0;
        _rowHaze[y] = below ? Haze(_eye.Z / -ray.Z * ray.Length()) : MaxHaze;
        if (below && ground is not null) Textured(y, ray, ground, palette);
        else Pixels.AsSpan(y * Width, Width).Fill(below ? Mix(palette.Ground, palette.Sky, _rowHaze[y]) : palette.Sky);
    }

    private void Textured(int y, Vector3 ray, Ground3D ground, Palette3D palette)
    {
        var reach = _eye.Z / -ray.Z;
        var metresPerPixel = reach / _focal;
        var zoom = Math.Clamp((int)Math.Floor(Math.Log2(1 / (ground.PixelsPerMetre * metresPerPixel))), MinZoom, MaxZoom);
        var scale = ground.PixelsPerMetre * (1 << zoom);
        var start = _eye + (ray + _right * ((0.5f - Width / 2f) / _focal)) * reach;
        var (fx, fy) = ((long)((ground.OriginX * (1 << zoom) + start.X * scale) * Fixed), (long)((ground.OriginY * (1 << zoom) - start.Y * scale) * Fixed));
        var (sx, sy) = ((long)(_right.X * metresPerPixel * scale * Fixed), (long)(-_right.Y * metresPerPixel * scale * Fixed));
        var fade = (uint)(_rowHaze[y] * 256);
        var keep = 256 - fade;
        var (skyRb, skyG) = ((palette.Sky & 0xFF00FF) * fade, (palette.Sky & 0x00FF00) * fade);
        var row = y * Width;
        var (tileX, tileY, drop) = (int.MinValue, int.MinValue, 0);
        byte[]? tile = null;
        var texels = ReadOnlySpan<uint>.Empty;
        for (var x = 0; x < Width; x++, fx += sx, fy += sy)
        {
            var (px, py) = ((int)(fx >> FixedShift), (int)(fy >> FixedShift));
            var (tx, ty) = (px >> TileShift, py >> TileShift);
            if (tx != tileX || ty != tileY)
            {
                (tileX, tileY) = (tx, ty);
                (tile, drop) = Find(ground, zoom, tx, ty);
                texels = tile is null ? ReadOnlySpan<uint>.Empty : MemoryMarshal.Cast<byte, uint>(tile);
            }
            var colour = tile is null
                ? palette.Ground
                : texels[(((py >> drop) - ((ty >> drop) << TileShift)) << TileShift) + (px >> drop) - ((tx >> drop) << TileShift)];
            Pixels[row + x] = 0xFF000000 | ((((colour & 0xFF00FF) * keep + skyRb) >> 8) & 0xFF00FF) | ((((colour & 0x00FF00) * keep + skyG) >> 8) & 0x00FF00);
        }
    }

    private static (byte[]? Texels, int Drop) Find(Ground3D ground, int zoom, int x, int y)
    {
        for (var drop = 0; drop <= FallbackLevels && zoom - drop >= 0; drop++)
            if (ground.Tile(zoom - drop, x >> drop, y >> drop, drop == 0) is { } texels) return (texels, drop);
        return (null, 0);
    }

    private void Grid(Camera3D camera, uint colour)
    {
        var step = Array.Find(GridSteps, s => s >= camera.Distance / 8) is var found and > 0 ? found : GridSteps[^1];
        var x0 = MathF.Round(camera.Target.X / step) * step;
        var y0 = MathF.Round(camera.Target.Y / step) * step;
        var reach = step * GridLines;
        for (var i = -GridLines; i <= GridLines; i++)
        {
            Line(new Vector3(x0 + i * step, y0 - reach, 0), new Vector3(x0 + i * step, y0 + reach, 0), colour);
            Line(new Vector3(x0 - reach, y0 + i * step, 0), new Vector3(x0 + reach, y0 + i * step, 0), colour);
        }
    }

    private void Line(Vector3 from, Vector3 to, uint colour)
    {
        var a = ToCamera(from);
        var b = ToCamera(to);
        if (a.Z < Near && b.Z < Near) return;
        if (a.Z < Near) a = Vector3.Lerp(a, b, (Near - a.Z) / (b.Z - a.Z));
        if (b.Z < Near) b = Vector3.Lerp(b, a, (Near - b.Z) / (a.Z - b.Z));
        var (sa, sb) = (Screen(a), Screen(b));
        if (!Clip(ref sa, ref sb)) return;
        var steps = (int)MathF.Max(MathF.Abs(sb.X - sa.X), MathF.Abs(sb.Y - sa.Y)) + 1;
        var delta = (sb - sa) / steps;
        for (var i = 0; i <= steps; i++)
        {
            var at = sa + delta * i;
            var (x, y) = ((int)at.X, (int)at.Y);
            if (x < 0 || y < 0 || x >= Width || y >= Height) continue;
            var index = y * Width + x;
            Pixels[index] = Mix(Pixels[index], colour, GridAlpha * (1 - _rowHaze[y] / MaxHaze));
        }
    }

    private bool Clip(ref Vector2 a, ref Vector2 b)
    {
        var (t0, t1) = (0f, 1f);
        var d = b - a;
        bool Edge(float p, float q)
        {
            if (p == 0) return q >= 0;
            var t = q / p;
            if (p < 0) t0 = MathF.Max(t0, t);
            else t1 = MathF.Min(t1, t);
            return t0 <= t1;
        }
        if (!(Edge(-d.X, a.X) && Edge(d.X, Width - 1 - a.X) && Edge(-d.Y, a.Y) && Edge(d.Y, Height - 1 - a.Y))) return false;
        (a, b) = (a + d * t0, a + d * t1);
        return true;
    }

    private static T[] Room<T>(T[] buffer, int needed) => buffer.Length >= needed ? buffer : new T[Math.Max(needed, buffer.Length * 2)];

    private void Project(Layout layout, int index, uint sky)
    {
        var face = layout.Faces[index];
        layout.Shapes[index].Count = 0;
        var plane = Vector3.Dot(face.Normal, face.Points[0] - _eye);
        if (plane > -1e-3f) return;
        var points = face.Points;
        var camera = _cameraScratch = Room(_cameraScratch ?? [], points.Length);
        var ring = _ringScratch = Room(_ringScratch ?? [], points.Length * 2);
        for (var i = 0; i < points.Length; i++) camera[i] = ToCamera(points[i]);
        var (pointStart, ringStart) = (layout.PointStart[index], layout.RingStart[index]);
        var (count, rings, start) = (0, 0, 0);
        var (top, bottom) = (float.MaxValue, float.MinValue);
        foreach (var end in face.Ends)
        {
            var clipped = ClipRing(camera, ring, start, end);
            if (clipped >= 3)
            {
                for (var i = 0; i < clipped; i++)
                {
                    var at = Screen(ring[i]);
                    layout.Screen[pointStart + count + i] = at;
                    top = MathF.Min(top, at.Y);
                    bottom = MathF.Max(bottom, at.Y);
                }
                count += clipped;
                layout.Ends[ringStart + rings++] = count;
            }
            start = end;
        }
        if (count < 3 || bottom < 0 || top > Height) return;
        var normal = new Vector3(Vector3.Dot(face.Normal, _right), Vector3.Dot(face.Normal, _up), Vector3.Dot(face.Normal, _forward));
        var (cx, cy) = (Width / 2f, Height / 2f);
        var lit = Shaded(face.Colour, Ambient + Diffuse * MathF.Max(0, Vector3.Dot(face.Normal, Light)));
        layout.Shapes[index] = new Shape
        {
            Count = count,
            Rings = rings,
            Top = top,
            Bottom = bottom,
            A = normal.X / (_focal * plane),
            B = -normal.Y / (_focal * plane),
            C = (normal.Z - normal.X * cx / _focal + normal.Y * cy / _focal) / plane,
            Colour = Mix(lit, sky, Haze(Vector3.Distance(face.Centre, _eye))),
        };
    }

    private static int ClipRing(Vector3[] camera, Vector3[] ring, int start, int end)
    {
        var count = 0;
        for (var i = start; i < end; i++)
        {
            var p = camera[i];
            var q = camera[i + 1 < end ? i + 1 : start];
            var (pIn, qIn) = (p.Z >= Near, q.Z >= Near);
            if (pIn) ring[count++] = p;
            if (pIn != qIn) ring[count++] = Vector3.Lerp(p, q, (Near - p.Z) / (q.Z - p.Z));
        }
        return count;
    }

    private void Fill(Band band, Layout layout, int index)
    {
        var shape = layout.Shapes[index];
        var (pointStart, ringStart) = (layout.PointStart[index], layout.RingStart[index]);
        var y0 = Math.Max(band.Top, (int)MathF.Ceiling(shape.Top - 0.5f));
        var y1 = Math.Min(band.Bottom - 1, (int)MathF.Floor(shape.Bottom - 0.5f));
        band.Crossings = Room(band.Crossings, shape.Count);
        var screen = layout.Screen;
        var crossings = band.Crossings;
        var (a, b, c, colour) = (shape.A, shape.B, shape.C, shape.Colour);
        for (var y = y0; y <= y1; y++)
        {
            var yc = y + 0.5f;
            var found = 0;
            var start = pointStart;
            for (var r = 0; r < shape.Rings; r++)
            {
                var end = pointStart + layout.Ends[ringStart + r];
                for (var i = start; i < end; i++)
                {
                    var p = screen[i];
                    var q = screen[i + 1 < end ? i + 1 : start];
                    if ((p.Y <= yc) == (q.Y <= yc)) continue;
                    var x = p.X + (yc - p.Y) * (q.X - p.X) / (q.Y - p.Y);
                    var k = found++;
                    while (k > 0 && crossings[k - 1] > x)
                    {
                        crossings[k] = crossings[k - 1];
                        k--;
                    }
                    crossings[k] = x;
                }
                start = end;
            }
            var row = y * Width;
            for (var j = 0; j + 1 < found; j += 2)
            {
                var xs = Math.Max(0, (int)MathF.Ceiling(crossings[j] - 0.5f));
                var xe = Math.Min(Width, (int)MathF.Ceiling(crossings[j + 1] - 0.5f));
                var depth = a * (xs + 0.5f) + b * yc + c;
                for (var x = xs; x < xe; x++, depth += a)
                {
                    if (depth <= _depth[row + x]) continue;
                    _depth[row + x] = depth;
                    Pixels[row + x] = colour;
                }
            }
        }
    }
}
