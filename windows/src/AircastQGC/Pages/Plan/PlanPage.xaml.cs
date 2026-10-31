using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Windows.Foundation;
using VirtualKey = Windows.System.VirtualKey;
using VirtualKeyModifiers = Windows.System.VirtualKeyModifiers;

namespace AircastQGC.Pages.Plan;

public sealed partial class PlanPage : UserControl
{
    private const string Client = "planPage.";
    private const string UndoTracking = "plan.undoTracking";
    private const int NewestPattern = -1;
    private const int CrowdedItems = 40;
    private const double PinSize = 24;
    private const double CurrentPinSize = 30;
    private const double DotSize = 10;
    private const double HandleSize = 14;
    private const double RallySize = 22;
    private const double CircleHandleSize = 18;
    private const string LegSplitLabel = "Add a waypoint here";
    private const string CornerSplitLabel = "Add a corner here";
    private const double SameSpotDegrees = 1e-6;
    private const double FieldZoom = 16;
    private const double FitPadding = 96;
    private const double ClearInset = 64;
    private static readonly TimeSpan PollInterval = TimeSpan.FromMilliseconds(700);
    private static readonly TimeSpan NoticeTime = TimeSpan.FromMilliseconds(2500);

    private readonly PlanCanvas _canvas;
    private readonly PlanDialogs _dialogs;
    private DispatcherQueueTimer? _poll;
    private DispatcherQueueTimer? _noticeTimer;
    private string _itemsRaw = string.Empty;
    private string _fencesRaw = string.Empty;
    private int _reads;
    private bool _polling;
    private Task _refresh = Task.CompletedTask;
    private Task _edit = Task.CompletedTask;
    private bool _refreshAgain;
    private IReadOnlyList<PlanItem> _items = [];
    private IReadOnlyList<LandingPattern> _landings = [];
    private IReadOnlyList<IReadOnlyList<Geo>> _route = [];
    private PlanFences _fences = PlanFences.None;
    private PlanStatus _status = PlanStatus.Empty;
    private IReadOnlyList<MissionKind> _kinds = [];
    private JsonObject _summary = new();
    private TerrainProfile _profile = TerrainProfile.None;
    private IReadOnlySet<int> _colliding = new HashSet<int>();
    private IReadOnlyList<(Geo From, Geo To)> _collisionLegs = [];
    private bool _terrainChecking;
    private Hit? _selected;
    private PlanLayer _layer = PlanLayer.Mission;
    private bool _panelOpen = true;
    private bool _firstRead = true;
    private int? _newPattern;
    private int? _sentSequence;
    private EditableShape? _editable;
    private readonly Dictionary<Hit, Geo> _moves = [];
    private readonly HashSet<Hit> _moving = [];
    private bool _answering;
    private string _editorIdentity = string.Empty;
    private string _editorStamp = string.Empty;
    private bool _placeable;

    public PlanPage()
    {
        InitializeComponent();
        _canvas = new PlanCanvas(Overlay, Map);
        _dialogs = new PlanDialogs(this);
        Map.PlanMode = true;
        Map.ScaleInCorner = true;
        Map.MapTapped += (latitude, longitude, point) => Settled(() => OnMapTapped(latitude, longitude, point));
        Map.ViewChanged += (_, _) =>
        {
            _canvas.Place();
            if ((PlaceAt() != null) != _placeable) RenderRail();
        };
        Overlay.SizeChanged += (_, e) => Overlay.Clip = new RectangleGeometry { Rect = new Rect(0, 0, e.NewSize.Width, e.NewSize.Height) };
        _canvas.Pressed += hit => Select(SelectionOf(hit), true);
        _canvas.Dragged += OnDragged;
        _canvas.Dropped += OnDropped;
        _canvas.SplitClicked += split => Settled(() => OnSplit(split));
        Accelerators();
        Loaded += OnLoaded;
        Unloaded += OnUnloaded;
    }

    public event EventHandler? FlyRequested;

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        PlanCalls.Queued(() => PlanCalls.Set(UndoTracking, true));
        Watch(PlanCalls.PlanView, view =>
        {
            _status = PlanStatus.From(view);
            RenderChrome();
            FitIfSettled();
            Prompts();
        });
        Watch(PlanCalls.KindsView, view =>
        {
            _kinds = PlanJson.Kinds(view);
            RenderRail();
        });
        Watch(PlanCalls.SummaryView, view =>
        {
            _summary = view;
            RenderChrome();
            RenderList();
        });
        Watch(PlanCalls.TerrainView, view =>
        {
            _profile = PlanJson.Profile(view);
            _colliding = PlanJson.Colliding(view, "collidingItems").Concat(PlanJson.Colliding(view, "collidingSimpleItems")).ToHashSet();
            _collisionLegs = PlanJson.CollisionLegs(view);
            _terrainChecking = MapUiJson.Flag(view["checking"]);
            Render();
        });
        _poll ??= Timer(PollInterval, Poll);
        _poll.Start();
        Changed();
    }

    private void OnUnloaded(object sender, RoutedEventArgs e)
    {
        _poll?.Stop();
        new[] { PlanCalls.PlanView, PlanCalls.KindsView, PlanCalls.SummaryView, PlanCalls.TerrainView }.ToList()
            .ForEach(path => BridgeWatch.Stop(Client + path));
        PlanCalls.Queued(() => PlanCalls.Set(UndoTracking, false));
    }

    private DispatcherQueueTimer Timer(TimeSpan interval, Action tick)
    {
        var timer = DispatcherQueue.CreateTimer();
        timer.Interval = interval;
        timer.Tick += (_, _) => tick();
        return timer;
    }

    private static void Watch(string path, Action<JsonObject> apply)
    {
        try { BridgeWatch.Watch(Client + path, new[] { path }, apply); }
        catch (Exception) { }
        apply(PlanCalls.Read(path));
    }

    private static JsonObject Parse(string raw)
    {
        if (raw.Length == 0) return new JsonObject();
        try { return JsonNode.Parse(raw) as JsonObject ?? new JsonObject(); }
        catch (JsonException) { return new JsonObject(); }
    }

    private sealed record PlanData(
        string ItemsRaw, string FencesRaw, IReadOnlyList<PlanItem> Items, IReadOnlyList<IReadOnlyList<Geo>> Route,
        PlanFences Fences, IReadOnlyList<LandingPattern> Landings);

    private static PlanData? Fetch(string knownItems, string knownFences)
    {
        var itemsRaw = PlanCalls.Raw(PlanCalls.ItemsView);
        var fencesRaw = PlanCalls.Raw(PlanCalls.FencesView);
        if (itemsRaw == knownItems && fencesRaw == knownFences) return null;
        var view = Parse(itemsRaw);
        var items = PlanJson.Items(view);
        var landings = items
            .Where(item => item.Kind == "land")
            .Select(item => PlanJson.Landing(item.Index, PlanCalls.Read($"view.landingPattern({item.Index})")))
            .OfType<LandingPattern>()
            .ToList();
        return new PlanData(itemsRaw, fencesRaw, items, MapOverlayJson.Mission(view).Route, PlanJson.Fences(Parse(fencesRaw)), landings);
    }

    private async void Poll()
    {
        Prompts();
        if (_polling || !_refresh.IsCompleted) return;
        _polling = true;
        var reads = _reads;
        var (items, fences) = (_itemsRaw, _fencesRaw);
        var data = await PlanCalls.Queued(() => Fetch(items, fences));
        _polling = false;
        if (data != null && reads == _reads) Apply(data);
    }

    private void Apply(PlanData data)
    {
        _itemsRaw = data.ItemsRaw;
        _fencesRaw = data.FencesRaw;
        _items = data.Items;
        _route = data.Route;
        _fences = data.Fences;
        _landings = data.Landings;
        if (!Survives(_selected)) _selected = null;
        _editable = EditableFor(_selected);
        SizeNewPattern();
        FitOnEntry();
        FitIfSettled();
        SyncSequence();
        RenderUndrawn();
        Render();
    }

    private Task Changed()
    {
        _reads++;
        Prompts();
        _refreshAgain = !_refresh.IsCompleted;
        return _refreshAgain ? _refresh : _refresh = Refreshed();
    }

    private async Task Refreshed()
    {
        _refreshAgain = false;
        var fences = _fencesRaw;
        if (await PlanCalls.Queued(() => Fetch(string.Empty, fences)) is { } data) Apply(data);
        if (_refreshAgain) await Refreshed();
    }

    private async void Settled(Action act)
    {
        if (PlanCalls.Pending.IsCompleted && _refresh.IsCompleted && _edit.IsCompleted)
        {
            act();
            return;
        }
        await PlanCalls.Pending;
        await _refresh;
        await _edit;
        Settled(act);
    }

    private bool Survives(Hit? hit) => hit switch
    {
        null => true,
        WaypointHit w => w.Index < _items.Count,
        SurveyVertexHit s => _items.ElementAtOrDefault(s.Item)?.Shape is { } shape && s.Vertex < shape.Vertices.Count,
        FenceVertexHit f => _fences.Polygons.FirstOrDefault(p => p.Index == f.Polygon) is { } polygon && f.Vertex < polygon.Vertices.Count,
        CircleHit c => _fences.Circles.Any(circle => circle.Index == c.Index),
        RallyHit r => _fences.Rally.Any(point => point.Index == r.Index),
        BreachHit => _fences.Breach != null,
        _ => false,
    };

    private EditableShape? EditableFor(Hit? hit) => hit switch
    {
        FenceVertexHit f => PlanCalls.Editable(PlanCalls.FencePath(f.Polygon)),
        SurveyVertexHit s when _items.ElementAtOrDefault(s.Item) is { Shape: { } shape } item => PlanCalls.Editable(PlanCalls.ShapePath(item), shape.Shape),
        WaypointHit w when _items.ElementAtOrDefault(w.Index) is { Shape: { } shape } item => PlanCalls.Editable(PlanCalls.ShapePath(item), shape.Shape),
        _ => null,
    };

    private Hit SelectionOf(Hit hit) => hit switch
    {
        LandingHit landing => new WaypointHit(landing.Index),
        CircleRadiusHit radius => new CircleHit(radius.Index),
        ShapeCentreHit { Fence: true } centre => _selected is FenceVertexHit corner && corner.Polygon == centre.Owner ? corner : new FenceVertexHit(centre.Owner, 0),
        ShapeCentreHit centre => _selected is { } chosen && PlanRules.MissionIndex(chosen) == centre.Owner ? chosen : new WaypointHit(centre.Owner),
        _ => hit,
    };

    private void Select(Hit? hit, bool openPanel)
    {
        _selected = hit;
        _panelOpen = openPanel;
        if (PlanRules.LayerOf(hit) is { } layer) _layer = layer;
        _editable = EditableFor(hit);
        SyncSequence();
        Render();
    }

    private void SyncSequence()
    {
        var sequence = PlanRules.MissionIndex(_selected) is int index && _items.ElementAtOrDefault(index) is { } item
            ? item.Sequence
            : _selected == null ? PlanRules.AppendSequence(_items) : (int?)null;
        if (sequence is not int wanted || wanted == _sentSequence) return;
        _sentSequence = wanted;
        PlanCalls.Queued(() => PlanCalls.SelectSequence(wanted));
    }

    private void FocusItem(int index)
    {
        Select(new WaypointHit(index), true);
        if (_items.ElementAtOrDefault(index)?.At is { } at) Recentre(at);
    }

    private void FocusNotReady(PlanStatus status)
    {
        if (status.NextNotReady is int next) FocusItem(next);
    }

    private int ItemCount => Math.Max(0, _items.Count - 1);

    private Geo? MapCentre() =>
        double.IsFinite(Map.Latitude) && double.IsFinite(Map.Longitude) && (Map.Latitude != 0 || Map.Longitude != 0)
            ? Map.ActualWidth > SidePanel.ActualWidth ? Map.GeoOf(new Point((SidePanel.ActualWidth + Map.ActualWidth) / 2, (TitleRoom.ActualHeight + Map.ActualHeight) / 2)) : new Geo(Map.Latitude, Map.Longitude)
            : null;

    private Geo? PlaceAt() => MapCentre() ?? PlanCalls.VehiclePosition();

    private IReadOnlyList<Geo> Visible()
    {
        var width = Map.ActualWidth;
        var height = Map.ActualHeight;
        var left = SidePanel.ActualWidth + ClearInset;
        var right = width - ClearInset;
        var top = TitleRoom.ActualHeight + ClearInset;
        if (right <= left || height - 8 <= top) return [];
        var bottom = height - 8;
        return new[] { new Point(left, top), new Point(right, top), new Point(right, bottom), new Point(left, bottom) }
            .Select(Map.GeoOf)
            .ToList();
    }

    private void OnMapTapped(double latitude, double longitude, Point _)
    {
        var at = new Geo(latitude, longitude);
        if (_selected != null && (_panelOpen || _selected is not WaypointHit))
        {
            Select(null, false);
            return;
        }
        switch (_layer)
        {
            case PlanLayer.Mission:
                _panelOpen = false;
                AddMissionItem("waypoint", "Adding a waypoint", at, PlanRules.InsertAfter(_selected, _items), false);
                break;
            case PlanLayer.Rally when _status.AddRally:
                AddRally(at);
                break;
            default:
                Select(null, false);
                break;
        }
    }

    private void AddMissionItem(string kind, string label, Geo? at, int index, bool openPanel) =>
        _edit = Inserting(kind, label, at, index, openPanel);

    private async Task Inserting(string kind, string label, Geo? at, int index, bool openPanel)
    {
        if (at is not { } where)
        {
            Say("Move the map to where this should go");
            return;
        }
        Busy(label);
        var outcome = await PlanCalls.Queued(() => PlanCalls.InsertPlaced(kind, where, index));
        if (!outcome.Ok)
        {
            Say(outcome.Reason);
            return;
        }
        Busy(null);
        if (outcome.Index is int added)
        {
            if (PlanRules.IsPattern(kind)) _newPattern = added;
            await Changed();
            Select(new WaypointHit(added), openPanel);
            return;
        }
        Changed();
    }

    private void AddRally(Geo at)
    {
        var next = _fences.Rally.Count;
        Bridged("Adding rally", () => PlanCalls.AddRally(at), () => Select(new RallyHit(next), true));
    }

    private void RemoveItem(PlanItem item) => _edit = Removing(item);

    private async Task Removing(PlanItem item)
    {
        Busy($"Removing #{item.Sequence}");
        var count = _items.Count;
        var outcome = await PlanCalls.Queued(() => PlanCalls.Remove(item.Index));
        if (!outcome.Ok)
        {
            Say(outcome.Reason);
            return;
        }
        Busy(null);
        await Changed();
        Select(PlanRules.AfterRemove(item.Index, count), _panelOpen);
        RebuildEditor(true);
    }

    private void SizeNewPattern()
    {
        if (_newPattern is not int wanted) return;
        _newPattern = null;
        var target = wanted == NewestPattern
            ? _items.Where(item => item.Shape != null).MaxBy(item => item.Index)
            : _items.ElementAtOrDefault(wanted) is { Shape: not null } item ? item : null;
        var visible = Visible();
        if (target?.Shape is not { } shape || visible.Count != 4) return;
        Resize(PlanCalls.ShapePath(target), shape.Line ? PlanShapes.Line(visible) : PlanShapes.Rectangle(visible), shape.Line);
    }

    private async void Resize(string path, IReadOnlyList<Geo> vertices, bool line)
    {
        await PlanCalls.Queued(() => PlanShapes.Replace(path, vertices, line));
        Changed();
    }

    private void FitOnEntry()
    {
        if (!_firstRead || _itemsRaw.Length == 0) return;
        _firstRead = false;
        Fit(AllPoints());
    }

    private IEnumerable<Geo> MissionPoints() => _items.Select(item => item.At).OfType<Geo>();

    private IEnumerable<Geo> AllPoints() =>
        MissionPoints()
            .Concat(_items.SelectMany(item => item.Shape?.Vertices ?? []))
            .Concat(_fences.Polygons.SelectMany(polygon => polygon.Vertices))
            .Concat(_fences.Circles.SelectMany(circle => MapOverlayJson.Ring(circle.Centre, circle.RadiusMetres)))
            .Concat(_fences.Rally.Select(point => point.At));

    private void Fit(IEnumerable<Geo> points)
    {
        var listed = points.ToList();
        if (listed.Count == 0) return;
        if (listed.Count == 1)
        {
            Recentre(listed[0], FieldZoom);
            return;
        }
        var world = listed.Select(World).ToList();
        var (minX, maxX) = (world.Min(p => p.X), world.Max(p => p.X));
        var (minY, maxY) = (world.Min(p => p.Y), world.Max(p => p.Y));
        var width = Math.Max(Map.ActualWidth - SidePanel.ActualWidth - FitPadding * 2, 64);
        var height = Math.Max(Map.ActualHeight - TitleRoom.ActualHeight - FitPadding * 2, 64);
        var zoom = Math.Log2(Math.Min(width / Math.Max(maxX - minX, 1e-9), height / Math.Max(maxY - minY, 1e-9)));
        var centre = Unworld((minX + maxX) / 2, (minY + maxY) / 2);
        Recentre(centre, Math.Clamp(zoom, 2, FieldZoom + 2));
    }

    private void Recentre(Geo at, double? zoom = null)
    {
        Follow(false);
        var scale = Math.Pow(2, zoom ?? Map.Zoom);
        var world = World(at);
        var centre = Unworld(world.X - SidePanel.ActualWidth / 2 / scale, world.Y - TitleRoom.ActualHeight / 2 / scale);
        Map.CenterOn(centre.Latitude, centre.Longitude, zoom);
    }

    private void Follow(bool on)
    {
        Map.Follow = on;
        if (on) Map.CenterOnVehicle();
        CentreButton.Background = on ? Token("SubtleSelectedBrush") : new SolidColorBrush(Microsoft.UI.Colors.Transparent);
        CentreButton.Foreground = Token(on ? "AccentTextBrush" : "TextPrimaryBrush");
    }

    private static Point World(Geo at)
    {
        var latitude = Math.Clamp(at.Latitude, -85.05112878, 85.05112878) * Math.PI / 180;
        return new Point((at.Longitude + 180) / 360 * 256, (1 - Math.Log(Math.Tan(latitude) + 1 / Math.Cos(latitude)) / Math.PI) / 2 * 256);
    }

    private static Geo Unworld(double x, double y) =>
        new(Math.Atan(Math.Sinh(Math.PI * (1 - 2 * y / 256))) * 180 / Math.PI, x / 256 * 360 - 180);

    private void OnDragged(Hit hit, Geo at)
    {
        _moves[hit] = at;
        if (_moving.Add(hit)) Moving(hit);
    }

    private async void Moving(Hit hit)
    {
        if (!_moves.Remove(hit, out var at))
        {
            _moving.Remove(hit);
            return;
        }
        if (await PlanCalls.Queued(Mover(hit, at))) Changed();
        Moving(hit);
    }

    private async void OnDropped(Hit hit, Geo at)
    {
        _moves.Remove(hit);
        Say(await PlanCalls.Queued(Mover(hit, at)) ? MovedText(hit) : "That did not work");
        await Changed();
        RebuildEditor(true);
    }

    private Func<bool> Mover(Hit hit, Geo at) => hit switch
    {
        WaypointHit w => () => PlanCalls.Move(w.Index, at),
        FenceVertexHit f => () => PlanCalls.AdjustVertex(PlanCalls.FencePath(f.Polygon), f.Vertex, at),
        SurveyVertexHit s when _items.ElementAtOrDefault(s.Item) is { Shape: not null } item => () => PlanCalls.AdjustVertex(PlanCalls.ShapePath(item), s.Vertex, at),
        RallyHit r when (_fences.Rally.FirstOrDefault(point => point.Index == r.Index)?.AltitudeMetres ?? 0) is var metres => () => PlanCalls.MoveRally(r.Index, at, metres),
        CircleHit c => () => PlanCalls.MoveCircle(c.Index, at),
        BreachHit => () => PlanCalls.SetBreach(at),
        LandingHit l => () => PlanCalls.MoveLanding(l.Index, l.Place, at),
        CircleRadiusHit r when _fences.Circles.FirstOrDefault(circle => circle.Index == r.Index) is { } circle && PlanRules.DraggedRadius(circle, at) is var radius => () => PlanCalls.SetCircleRadius(r.Index, radius),
        ShapeCentreHit s when CentreTarget(s) is { } target && PlanShapes.MovedTo(target.Vertices, at) is var moved => () => PlanShapes.Replace(target.Path, moved, false),
        _ => () => false,
    };

    private (string Path, IReadOnlyList<Geo> Vertices)? CentreTarget(ShapeCentreHit hit) => hit.Fence
        ? _fences.Polygons.FirstOrDefault(polygon => polygon.Index == hit.Owner) is { } polygon ? (PlanCalls.FencePath(polygon.Index), polygon.Vertices) : null
        : _items.ElementAtOrDefault(hit.Owner) is { Shape: { Line: false } shape } item ? (PlanCalls.ShapePath(item), shape.Vertices) : null;

    private string MovedText(Hit hit) => hit switch
    {
        WaypointHit w => _items.ElementAtOrDefault(w.Index) is { } item ? $"Moved #{item.Sequence}" : "Moved an item",
        FenceVertexHit => "Moved a fence corner",
        SurveyVertexHit => "Moved a survey corner",
        RallyHit => "Moved a rally point",
        CircleHit => "Moved a fence circle",
        BreachHit => "Moved the breach return point",
        LandingHit { Place: LandingPlace.Approach } => "Moved the final approach",
        LandingHit => "Moved the touchdown",
        CircleRadiusHit => "Changed a fence radius",
        ShapeCentreHit { Fence: true } => "Moved a fence",
        ShapeCentreHit => "Moved a survey area",
        _ => "Moved",
    };

    private void OnSplit(SplitHandle split)
    {
        if (split.Path == MissionRoot) AddMissionItem("waypoint", "Adding a waypoint", split.At, split.Segment, _panelOpen);
        else Bridged("Adding a corner", () => PlanCalls.Invoke($"{split.Path}.{split.Invokable}", split.Segment).Ok, () => _editable = EditableFor(_selected));
    }

    private SplitHandle? LegSplit()
    {
        if (_selected is not WaypointHit chosen || _items.ElementAtOrDefault(chosen.Index) is not { Routed: true, At: { } to } current) return null;
        var previous = _items
            .Where(item => item.Index > PlanRules.HomeItem && item.Index < current.Index && item.Routed && item.At != null)
            .MaxBy(item => item.Index);
        return (previous?.Exit ?? previous?.At) is { } from
            ? new SplitHandle(MissionRoot, string.Empty, current.Index, PlanShapes.Centre([from, to]), LegSplitLabel)
            : null;
    }

    private void Bridged(string? label, Func<bool> work, Action? then = null, string? done = null) =>
        _edit = Bridging(label, work, then, done);

    private async Task Bridging(string? label, Func<bool> work, Action? then, string? done)
    {
        Busy(label);
        if (!await PlanCalls.Queued(work))
        {
            Say($"{label ?? "That"} did not work");
            return;
        }
        then?.Invoke();
        if (done != null) Say(done);
        else Busy(null);
        Changed();
    }

    private void Busy(string? message)
    {
        _noticeTimer?.Stop();
        BusyText.Text = message ?? string.Empty;
        BusyCard.Visibility = message == null ? Visibility.Collapsed : Visibility.Visible;
    }

    private void Say(string message)
    {
        Busy(message);
        _noticeTimer ??= Timer(NoticeTime, () =>
        {
            _noticeTimer?.Stop();
            Busy(null);
        });
        _noticeTimer.Start();
    }

    private void Accelerators()
    {
        void Key(VirtualKey key, VirtualKeyModifiers modifiers, Action act)
        {
            var accelerator = new KeyboardAccelerator { Key = key, Modifiers = modifiers };
            accelerator.Invoked += (_, e) =>
            {
                if (_dialogs.Showing || (XamlRoot != null && FocusManager.GetFocusedElement(XamlRoot) is TextBox)) return;
                e.Handled = true;
                Settled(act);
            };
            KeyboardAccelerators.Add(accelerator);
        }
        Key(VirtualKey.Delete, VirtualKeyModifiers.None, DeleteSelected);
        Key(VirtualKey.Escape, VirtualKeyModifiers.None, () => Select(null, false));
        Key(VirtualKey.Z, VirtualKeyModifiers.Control, Undo);
        Key(VirtualKey.Y, VirtualKeyModifiers.Control, Redo);
        Key(VirtualKey.S, VirtualKeyModifiers.Control, Save);
        Key(VirtualKey.O, VirtualKeyModifiers.Control, () => Open());
    }

    private void DeleteSelected()
    {
        switch (_selected)
        {
            case WaypointHit w when w.Index > PlanRules.HomeItem && _items.ElementAtOrDefault(w.Index) is { } item:
                RemoveItem(item);
                break;
            case RallyHit r:
                RemoveRally(r.Index);
                break;
            case FenceVertexHit f when _editable?.CanRemoveVertex == true:
                Bridged("Removing corner", () => PlanCalls.RemoveVertex(PlanCalls.FencePath(f.Polygon), f.Vertex), () => Select(null, false));
                break;
            case CircleHit c:
                Bridged("Removing circle", () => PlanCalls.DeleteCircle(c.Index), () => Select(null, false));
                break;
        }
    }

    private void RemoveRally(int index)
    {
        var count = _fences.Rally.Count;
        Bridged("Removing rally point", () => PlanCalls.RemoveRally(index), () => Select(PlanRules.RallyAfterRemove(index, count), true));
    }

    private void Undo() => Bridged(null, () => PlanCalls.Invoke("plan.undo").Ok, () => RebuildEditor(true));

    private void Redo() => Bridged(null, () => PlanCalls.Invoke("plan.redo").Ok, () => RebuildEditor(true));

    private Scene BuildScene()
    {
        var selectedItem = PlanRules.MissionIndex(_selected);
        var crowded = _items.Count > CrowdedItems;
        var home = _items.FirstOrDefault(item => item.Index == PlanRules.HomeItem && item.At != null);
        var homeCovered = home?.At is { } spot && _items.Any(item => item.Index != PlanRules.HomeItem && item.At is { } at
            && Math.Abs(at.Latitude - spot.Latitude) < SameSpotDegrees && Math.Abs(at.Longitude - spot.Longitude) < SameSpotDegrees);
        var mission = _layer == PlanLayer.Mission;
        var fence = _layer == PlanLayer.Fence;

        var traces = _route.Select(route => new Trace(route, Ink.Path))
            .Concat(_items.Where(item => item.Shape != null).SelectMany(item => ShapeTraces(item.Shape!)))
            .Concat(_fences.Polygons.Select(polygon => new Trace(polygon.Vertices, polygon.Inclusion ? Ink.KeepIn : Ink.KeepOut, Closed: true, Filled: true, Thickness: 2)))
            .Concat(_fences.Circles.Select(circle => new Trace(MapOverlayJson.Ring(circle.Centre, circle.RadiusMetres), circle.Inclusion ? Ink.KeepIn : Ink.KeepOut, Closed: true, Filled: true, Thickness: 2)))
            .Concat(_fences.Firmware.Count > 0 ? [new Trace(_fences.Firmware, Ink.KeepIn, Closed: true, Dashed: true, Thickness: 2)] : [])
            .Concat(_collisionLegs.Select(leg => new Trace([leg.From, leg.To], Ink.Collision, Thickness: 4)))
            .Concat(_landings.SelectMany(LandingTraces))
            .ToList();

        var missionPins = _items
            .Where(item => item.At != null && !(item.Index == PlanRules.HomeItem && homeCovered))
            .Select(item => ItemPin(item, item.At!.Value, PlanRules.Seal(item), crowded, item.Index == selectedItem, mission))
            .Concat(_items.Where(item => item.Complex && item.Exit != null)
                .Select(item => ItemPin(item, item.Exit!.Value, PlanRules.Label(item.Sequence + item.Folded, string.Empty), crowded, item.Index == selectedItem, false)));

        var surveyHandles = selectedItem is int chosen && _items.ElementAtOrDefault(chosen)?.Shape is { } chosenShape
            ? chosenShape.Vertices.Select((vertex, at) => new Pin(new SurveyVertexHit(chosen, at), vertex, string.Empty, string.Empty, Ink.Handle, HandleSize,
                _selected == new SurveyVertexHit(chosen, at), mission))
            : [];

        var fenceHandles = fence
            ? _fences.Polygons.SelectMany(polygon => polygon.Vertices.Select((vertex, at) => new Pin(new FenceVertexHit(polygon.Index, at), vertex, string.Empty, string.Empty,
                Ink.Handle, HandleSize, _selected == new FenceVertexHit(polygon.Index, at), true)))
            : [];

        var circleHandles = _fences.Circles.Select(circle => new Pin(new CircleHit(circle.Index), circle.Centre, string.Empty, string.Empty,
            circle.Inclusion ? Ink.KeepIn : Ink.KeepOut, CircleHandleSize, _selected == new CircleHit(circle.Index), fence));

        var rally = _fences.Rally.Select(point => new Pin(new RallyHit(point.Index), point.At, "R", string.Empty, Ink.Rally, RallySize,
            _selected == new RallyHit(point.Index), _layer == PlanLayer.Rally));

        var breach = _fences.Breach is { } point
            ? new[] { new Pin(new BreachHit(), point.At, string.Empty, string.Empty, Ink.Breach, RallySize, _selected is BreachHit, fence, Glyph: "") }
            : [];

        var landingHandles = _landings
            .Where(landing => landing.Index == selectedItem)
            .SelectMany(landing => new[] { (LandingPlace.Approach, landing.Approach, "Approach"), (LandingPlace.Touchdown, landing.Landing, "Touchdown") }
                .Where(place => place.Item2 != null)
                .Select(place => new Pin(new LandingHit(landing.Index, place.Item1), place.Item2!.Value, string.Empty, place.Item3, Ink.Handle, HandleSize, false, mission)));

        var radiusHandles = _fences.Circles
            .Where(circle => fence && _selected == new CircleHit(circle.Index))
            .Select(circle => new Pin(new CircleRadiusHit(circle.Index), MapOverlayJson.PointAt(circle.Centre, circle.RadiusMetres, 90), string.Empty, "Radius",
                Ink.Handle, HandleSize, false, true));

        var fenceCentres = fence
            ? _fences.Polygons.Select(polygon => new Pin(new ShapeCentreHit(true, polygon.Index), PlanShapes.Centre(polygon.Vertices), string.Empty, "Move", Ink.Handle, HandleSize, false, true))
            : [];

        var surveyCentre = selectedItem is int owner && _items.ElementAtOrDefault(owner)?.Shape is { Line: false, Vertices.Count: >= 3 } area
            ? new[] { new Pin(new ShapeCentreHit(false, owner), PlanShapes.Centre(area.Vertices), string.Empty, "Move", Ink.Handle, HandleSize, false, mission) }
            : [];

        var splits = (_editable is { } shape && shape.SplitInvokable.Length > 0
                ? shape.Midpoints.Select((at, segment) => new SplitHandle(shape.Path, shape.SplitInvokable, segment, at, CornerSplitLabel))
                : [])
            .Concat(mission && LegSplit() is { } leg ? [leg] : []);

        var pins = missionPins.Concat(surveyHandles).Concat(fenceHandles).Concat(circleHandles).Concat(rally).Concat(breach)
            .OrderBy(pin => pin.Selected)
            .Concat(fenceCentres).Concat(surveyCentre).Concat(radiusHandles).Concat(landingHandles)
            .ToList();
        return new Scene(traces, pins, splits.ToList());
    }

    private static IEnumerable<Trace> LandingTraces(LandingPattern landing) =>
        new[]
        {
            new[] { landing.Approach, landing.SlopeStart, landing.Landing }.OfType<Geo>().ToList() is { Count: >= 2 } path ? new Trace(path, Ink.Land, Thickness: 2) : null,
            landing is { LoiterToAltitude: true, Approach: { } centre, LoiterRadiusMetres: double radius }
                ? new Trace(MapOverlayJson.Ring(centre, radius), Ink.Land, Closed: true, Dashed: true, Thickness: 2)
                : null,
        }.OfType<Trace>();

    private static IEnumerable<Trace> ShapeTraces(ItemShape shape) =>
        new[]
        {
            shape.Line ? new Trace(shape.Vertices, Ink.Area, Thickness: 2) : new Trace(shape.Vertices, Ink.Area, Closed: true, Filled: true, Thickness: 2),
            shape.Outline.Count > 2 ? new Trace(shape.Outline, Ink.Area, Closed: true, Dashed: true, Thickness: 1) : null,
            shape.Transects.Count > 1 ? new Trace(shape.Transects, Ink.Transect, Thickness: 2) : null,
            shape.FlightLoop.Count > 1 ? new Trace(shape.FlightLoop, Ink.Transect, Closed: true, Thickness: 2) : null,
        }.OfType<Trace>();

    private Pin ItemPin(PlanItem item, Geo at, string label, bool crowded, bool selected, bool draggable)
    {
        var lettered = item.Complex ? string.Empty : item.Abbreviation;
        var side = !crowded && lettered.Length > 1 && lettered[0] > 'A' && lettered[0] < 'z' ? lettered : string.Empty;
        var size = selected ? CurrentPinSize : crowded ? DotSize : PinSize;
        return new Pin(new WaypointHit(item.Index), at, crowded && !selected ? string.Empty : label, side, InkOf(item), size, selected, draggable,
            !item.Ready || _colliding.Contains(item.Index));
    }

    private static Ink InkOf(PlanItem item) => (item.Index, item.Kind, item.Command) switch
    {
        (PlanRules.HomeItem, _, _) => Ink.Home,
        (_, "takeoff", _) => Ink.Takeoff,
        (_, "land", _) or (_, _, PlanRules.ReturnToLaunch) => Ink.Land,
        _ => Ink.Path,
    };

    private void Render()
    {
        _canvas.Draw(BuildScene());
        RenderChrome();
        RenderRail();
        RenderPanel();
        RebuildEditor(false);
    }
}
