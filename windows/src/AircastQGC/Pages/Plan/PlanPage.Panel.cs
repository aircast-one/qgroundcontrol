using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;

namespace AircastQGC.Pages.Plan;

public sealed partial class PlanPage
{
    private const double ChipSize = 36;
    private const double RowMarkerSize = 36;
    private const double RailWidth = 60;

    private static Brush Token(string key) => (Brush)Application.Current.Resources[key];

    private static string InkKey(string kind, int command, int index) => (index, kind, command) switch
    {
        (PlanRules.HomeItem, _, _) => "TextTertiaryBrush",
        (_, "takeoff", _) => "SuccessBrush",
        (_, "land", _) or (_, _, PlanRules.ReturnToLaunch) => "CriticalBrush",
        _ => "MapPathBrush",
    };

    private void RenderChrome()
    {
        PlanTitle.Text = PlanRules.PlanTitle(_status.File);
        var aircraft = _status.Offline ? null : PlanCalls.VehiclePosition();
        var far = PlanRules.FarFromAircraft(aircraft, _items) is double metres
            ? PlanRules.StartsFromAircraft(metres, MapUiJson.Flag(_summary["imperial"]))
            : null;
        var warning = far ?? PlanRules.TerrainWarning(_collisionLegs.Count, _colliding.Count);
        var note = _terrainChecking && ItemCount > 0 ? "Checking the terrain…" : null;
        var plain = _status.Syncing || ItemCount == 0 ? _status.Line : PlanRules.StatsLine(ItemCount, _items, _summary);
        var warned = warning != null && !_status.Syncing;
        PlanLine.Text = warned ? warning! : (!_status.Syncing ? note : null) ?? plain;
        PlanLine.Foreground = Token(warned ? "CriticalBrush" : "TextSecondaryBrush");
        PlanLine.Visibility = PlanLine.Text.Length > 0 && PlanLine.Text != PlanTitle.Text ? Visibility.Visible : Visibility.Collapsed;
        RenderPill(warned);
        UndoButton.IsEnabled = _status.CanUndo;
    }

    private void RenderPill(bool warned)
    {
        var upload = !_status.Offline;
        var label = PlanRules.UploadLabel(_status);
        var done = upload && label == PlanRules.Uploaded;
        var enabled = upload ? _status.ContainsItems && !_status.Syncing : _status.CanSave && _status.Dirty;
        var emphasised = upload ? enabled && !done : _status.Dirty;
        PillButton.Visibility = upload || _status.ContainsItems ? Visibility.Visible : Visibility.Collapsed;
        PillText.Text = upload ? done ? "Fly" : label : _status.Dirty ? "Save" : "Saved";
        PillIcon.Glyph = (upload, done, warned, _status.Dirty) switch
        {
            (true, true, _, _) => "",
            (true, false, true, _) => "",
            (true, false, false, _) => "",
            (false, _, _, true) => "",
            _ => "",
        };
        PillButton.IsEnabled = enabled || done;
        PillButton.Background = Token(done ? "SuccessBrush" : emphasised ? "AccentBrush" : "ControlBrush");
        PillButton.Foreground = Token(done || emphasised ? "AccentOnBrush" : "TextPrimaryBrush");
        AutomationProperties(PillButton, PillText.Text);
        PillProgress.Width = _status.Syncing ? PillButton.ActualWidth * _status.Progress : 0;
    }

    private static void AutomationProperties(UIElement element, string name) =>
        Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(element, name);

    private void RenderRail()
    {
        _placeable = PlaceAt() != null;
        RailItems.Children.Clear();
        var layers = new[] { (PlanLayer.Mission, "Mission", ""), (PlanLayer.Fence, "Fence", ""), (PlanLayer.Rally, "Rally", "") }
            .Select(layer => RailButton(layer.Item3, layer.Item2, () =>
            {
                _layer = layer.Item1;
                _panelOpen = true;
                if (PlanRules.LayerOf(_selected) != layer.Item1) _selected = null;
                _editable = EditableFor(_selected);
                Render();
            }, chosen: _layer == layer.Item1));
        var tools = _layer switch
        {
            PlanLayer.Mission => MissionTools(),
            PlanLayer.Fence => FenceTools(),
            _ => [RailButton("", "Add point", () =>
            {
                if (PlaceAt() is { } at) AddRally(at);
            }, enabled: _status.AddRally && PlaceAt() != null)],
        };
        layers.Append((UIElement)new Rectangle { Height = 1, Width = 28, Margin = new Thickness(0, 4, 0, 4), Fill = Token("DividerBrush") })
            .Concat(tools)
            .ToList()
            .ForEach(RailItems.Children.Add);
    }

    private bool Addable(string kind) => PlanRules.Allows(_kinds, kind) || !_status.HomeSet;

    private IEnumerable<UIElement> MissionTools()
    {
        var patterns = PlanRules.ScanPatterns(_kinds);
        var waypoint = RailButton("", "Waypoint", () => AddMissionItem("waypoint", "Adding a waypoint", PlaceAt(), PlanRules.InsertAfter(_selected, _items), _panelOpen),
            enabled: PlaceAt() != null);
        var pattern = RailButton("", "Pattern", () => PickPattern(patterns), enabled: patterns.Count > 0);
        var takeoff = PlanRules.Offered(_kinds, "takeoff") && PlanRules.TakeoffMissing(_items)
            ? RailButton("", "Takeoff", () => AddMissionItem("takeoff", "Adding a takeoff", PlaceAt(), PlanRules.BeforeTheRest, _panelOpen), enabled: Addable("takeoff"))
            : null;
        var landLabel = PlanRules.KindLabel(_kinds, "land");
        var land = RailButton(landLabel == "Return" ? "" : "", landLabel, () => AddMissionItem("land", "Adding a landing", PlaceAt(), PlanRules.InsertAfter(_selected, _items), _panelOpen),
            enabled: ItemCount > 0);
        var drop = PlanRules.Offered(_kinds, "drop")
            ? RailButton("", "Drop", () => AddMissionItem("drop", "Adding a drop", PlaceAt(), PlanRules.InsertAfter(_selected, _items), _panelOpen), enabled: ItemCount > 0 && Addable("drop"))
            : null;
        return new UIElement?[] { waypoint, pattern, takeoff, land, drop }.OfType<UIElement>();
    }

    private IEnumerable<UIElement> FenceTools()
    {
        if (_status.FenceRefused) return [];
        var polygon = RailButton("", "Polygon", () =>
        {
            var next = _fences.Polygons.Count;
            var window = FenceWindow();
            Bridged("Adding fence", () => window is { } corners && PlanCalls.AddPolygon(corners.TopLeft, corners.BottomRight),
                () => Select(new FenceVertexHit(next, 0), true));
        }, enabled: _status.AddFence);
        var circle = RailButton("", "Circle", () =>
        {
            var next = _fences.Circles.Count;
            var window = FenceWindow();
            Bridged("Adding circle", () => window is { } corners && PlanCalls.AddCircle(corners.TopLeft, corners.BottomRight),
                () => Select(new CircleHit(next), true));
        }, enabled: _status.AddFence);
        var breach = RailButton("", "Breach", () =>
        {
            var place = PlaceAt();
            if (_fences.Breach != null) Select(new BreachHit(), true);
            else Bridged("Adding breach return point", () => place is { } at && PlanCalls.SetBreach(at));
        }, enabled: _status.AddFence, chosen: _fences.Breach != null);
        return [polygon, circle, breach];
    }

    private (Geo TopLeft, Geo BottomRight)? FenceWindow() => PlaceAt() is { } at ? PlanRules.FenceWindow(Visible(), at) : null;

    private async void PickPattern(IReadOnlyList<MissionKind> patterns)
    {
        bool Offered(MissionKind kind) => kind.Enabled || !_status.HomeSet;
        var blocked = patterns.FirstOrDefault(kind => !Offered(kind) && kind.DisabledReason.Length > 0)?.DisabledReason;
        var chosen = await _dialogs.Pick("Which pattern?", blocked ?? "A pattern covers an area or a line with a camera run.",
            patterns.Select(kind => (kind, kind.Label, Offered(kind))).ToList());
        if (chosen == null) return;
        AddMissionItem(chosen.Id, $"Adding {chosen.Label.ToLowerInvariant()}", PlaceAt(), PlanRules.InsertAfter(_selected, _items), _panelOpen);
    }

    private Button RailButton(string glyph, string label, Action click, bool enabled = true, bool chosen = false)
    {
        var content = new StackPanel { Spacing = 2, HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };
        content.Children.Add(new FontIcon { Glyph = glyph, FontSize = 18 });
        content.Children.Add(new TextBlock { Text = label, FontSize = 10, HorizontalAlignment = HorizontalAlignment.Center, TextTrimming = TextTrimming.Clip });
        var button = new Button
        {
            Width = RailWidth - 8,
            Height = RailWidth - 8,
            Padding = new Thickness(0),
            Content = content,
            IsEnabled = enabled,
            BorderThickness = new Thickness(0),
            CornerRadius = new CornerRadius(4),
            Background = chosen ? Token("SubtleSelectedBrush") : new SolidColorBrush(Microsoft.UI.Colors.Transparent),
            Foreground = Token(chosen ? "AccentTextBrush" : "TextPrimaryBrush"),
        };
        ToolTipService.SetToolTip(button, label);
        AutomationProperties(button, label);
        button.Click += (_, _) => Settled(click);
        return button;
    }

    private PlanItem? ChosenItem() => PlanRules.MissionIndex(_selected) is int index && _selected is WaypointHit ? _items.ElementAtOrDefault(index) : null;

    private void RenderPanel()
    {
        var chosenItem = ChosenItem();
        var missionLayer = _layer == PlanLayer.Mission;
        StripRow.Visibility = Shown(missionLayer && ItemCount > 0 && PlanRules.LayerOf(_selected) is null or PlanLayer.Mission);
        if (StripRow.Visibility == Visibility.Visible) RenderStrip(chosenItem?.Index);
        ProfileButton.Visibility = Shown(_profile.Points.Count > 0);
        ProfileButton.IsChecked = ProfileShown();

        SelectionHeader.Visibility = Shown(_selected != null);
        if (chosenItem != null)
        {
            var conflict = _colliding.Contains(chosenItem.Index);
            SelectionTitle.Text = PlanRules.ItemTitle(chosenItem);
            SelectionDetail.Text = conflict
                ? PlanRules.TerrainConflictHere
                : chosenItem.Complex && PlanRules.Detail(chosenItem) is { Length: > 0 } detail
                    ? detail
                    : !_panelOpen ? $"Click the map to add after #{chosenItem.Sequence}" : PlanRules.AltitudeWithFrame(chosenItem) ?? string.Empty;
            SelectionDetail.Foreground = Token(conflict ? "CriticalBrush" : "TextSecondaryBrush");
        }
        else if (_selected != null)
        {
            SelectionTitle.Text = SelectionTitleOf(_selected);
            SelectionDetail.Text = SelectionDetailOf(_selected) ?? string.Empty;
            SelectionDetail.Foreground = Token("TextSecondaryBrush");
        }
        SelectionDetail.Visibility = Shown(SelectionDetail.Text.Length > 0);

        EmptyState.Visibility = Shown(_selected == null && missionLayer && ItemCount == 0);
        HomeHint.Visibility = Shown(!_status.HomeSet);
        EmptyDownloadButton.Visibility = Shown(!_status.Offline);
        EmptyOpenButton.IsEnabled = _status.CanOpen;
        TemplatesButton.IsEnabled = _status.CanNewPlan;
        LayerTitle.Visibility = Shown(_selected == null && !missionLayer);
        LayerTitle.Text = _layer == PlanLayer.Fence ? "Fence" : "Rally";

        var profiled = _selected == null && missionLayer && ProfileShown() && _profile.Drawable && ItemCount > 0;
        ProfileHost.Content = profiled
            ? TerrainChart.View(_profile, MapUiJson.Text(PlanCalls.Read("settings.flightMapSettings.elevationMapProvider.rawValue")["value"]),
                PlanRules.MissionIndex(_selected) is int index ? _items.ElementAtOrDefault(index)?.Sequence : null,
                sequence =>
                {
                    if (_items.FirstOrDefault(item => item.Sequence == sequence) is { } item) FocusItem(item.Index);
                })
            : null;
        RenderList();
    }

    private static Visibility Shown(bool on) => on ? Visibility.Visible : Visibility.Collapsed;

    private static bool ProfileShown()
    {
        var setting = PlanCalls.Read($"{PlanCalls.ShowStatusSetting}.rawValue");
        return setting["value"] is not JsonValue value || !value.TryGetValue<bool>(out var shown) || shown;
    }

    private async void OnProfileToggled(object sender, RoutedEventArgs e)
    {
        var shown = !ProfileShown();
        await PlanCalls.Queued(() => PlanCalls.Set(PlanCalls.ShowStatusSetting, shown));
        RenderPanel();
    }

    private void RenderStrip(int? chosen)
    {
        Strip.Children.Clear();
        PlanRules.Rows(_items).Where(row => row.Index != PlanRules.HomeItem).Select(row =>
        {
            var ink = Token(InkKey(row.Kind, row.Command, row.Index));
            var picked = row.Index == chosen;
            var conflict = _colliding.Contains(row.Index);
            var chip = new Button
            {
                Width = ChipSize,
                Height = ChipSize,
                CornerRadius = new CornerRadius(ChipSize / 2),
                Padding = new Thickness(0),
                Background = row.Ready ? ink : new SolidColorBrush(Microsoft.UI.Colors.Transparent),
                Foreground = row.Ready ? Token("AccentOnBrush") : Token("CautionBrush"),
                BorderBrush = conflict ? Token("CriticalBrush") : picked ? Token("TextPrimaryBrush") : row.Ready ? ink : Token("CautionBrush"),
                BorderThickness = new Thickness(conflict || picked ? 3 : 1),
                Content = new TextBlock { Text = row.Seal, FontWeight = Microsoft.UI.Text.FontWeights.SemiBold, FontSize = 12 },
            };
            AutomationProperties(chip, $"{MapUiJson.SentenceCase(row.Name)} {row.Seal}{(conflict ? ", too close to the terrain" : string.Empty)}");
            ToolTipService.SetToolTip(chip, MapUiJson.SentenceCase(row.Name));
            chip.Click += (_, _) => FocusItem(row.Index);
            var altitude = _items.ElementAtOrDefault(row.Index)?.AltitudeText ?? string.Empty;
            return (UIElement)new StackPanel
            {
                Spacing = 2,
                Children = { chip, new TextBlock { Text = altitude, FontSize = 10, HorizontalAlignment = HorizontalAlignment.Center, Foreground = Token("TextSecondaryBrush") } },
            };
        }).ToList().ForEach(Strip.Children.Add);
    }

    private string SelectionTitleOf(Hit hit) => hit switch
    {
        FenceVertexHit => "Fence corner",
        CircleHit => "Circular fence",
        SurveyVertexHit s => $"{MapUiJson.SentenceCase(PatternName(s.Item))} corner",
        RallyHit r => $"Rally point {r.Index + 1}",
        BreachHit => "Breach return point",
        _ => "Selected",
    };

    private string? SelectionDetailOf(Hit hit) => hit switch
    {
        CircleHit c => _fences.Circles.FirstOrDefault(circle => circle.Index == c.Index)?.Detail,
        FenceVertexHit f => _fences.Polygons.FirstOrDefault(polygon => polygon.Index == f.Polygon) is { } polygon
            ? $"corner {f.Vertex + 1} of {polygon.Vertices.Count}"
            : null,
        _ => null,
    };

    private string PatternName(int index) => _items.ElementAtOrDefault(index)?.Name is { Length: > 0 } name ? name : "pattern";

    private void OnDone(object sender, RoutedEventArgs e) => Select(null, false);

    private void RenderList()
    {
        ListHost.Children.Clear();
        var rows = _layer switch
        {
            PlanLayer.Mission when _items.Any(item => item.Index != PlanRules.HomeItem) => ItemList(),
            PlanLayer.Fence when _selected == null => FenceList(),
            PlanLayer.Rally when _selected == null => RallyList(),
            _ => [],
        };
        rows.ToList().ForEach(ListHost.Children.Add);
    }

    private IEnumerable<UIElement> ItemList()
    {
        var rows = PlanRules.Rows(_items);
        var summary = PlanRules.SummaryText(_summary);
        var heading = new StackPanel
        {
            Spacing = 2,
            Margin = new Thickness(0, 8, 0, 4),
            Children = { PlanFacts.Heading(PlanRules.ItemCountText(rows.Count(row => row.Index != PlanRules.HomeItem))) },
        };
        if (summary.Length > 0) heading.Children.Add(PlanFacts.Note(summary));
        return rows.Select(row => (UIElement)ItemRowView(row, (_selected as WaypointHit)?.Index == row.Index)).Prepend(heading);
    }

    private UIElement ItemRowView(ItemRow row, bool picked)
    {
        var ink = Token(InkKey(row.Kind, row.Command, row.Index));
        var marker = new Grid
        {
            Width = RowMarkerSize,
            Height = RowMarkerSize,
            Children =
            {
                new Ellipse { Fill = row.Ready ? ink : null, Stroke = row.Ready ? null : Token("CautionBrush"), StrokeThickness = 1 },
                new TextBlock
                {
                    Text = row.Number,
                    FontSize = 12,
                    FontWeight = Microsoft.UI.Text.FontWeights.SemiBold,
                    Foreground = row.Ready ? Token("AccentOnBrush") : Token("CautionBrush"),
                    HorizontalAlignment = HorizontalAlignment.Center,
                    VerticalAlignment = VerticalAlignment.Center,
                },
            },
        };
        var text = new StackPanel { Spacing = 2, VerticalAlignment = VerticalAlignment.Center, Children = { PlanFacts.Text(MapUiJson.SentenceCase(row.Name)) } };
        if (row.Detail.Length > 0) text.Children.Add(PlanFacts.Note(row.Detail));
        var grid = new Grid
        {
            ColumnSpacing = 12,
            ColumnDefinitions = { new ColumnDefinition { Width = GridLength.Auto }, new ColumnDefinition(), new ColumnDefinition { Width = GridLength.Auto } },
            Children = { marker, text },
        };
        Grid.SetColumn(text, 1);
        if (picked)
        {
            var selected = PlanFacts.Note("Selected");
            selected.VerticalAlignment = VerticalAlignment.Center;
            Grid.SetColumn(selected, 2);
            grid.Children.Add(selected);
        }
        var button = new Button
        {
            Content = grid,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            Padding = new Thickness(8),
            BorderThickness = new Thickness(0),
            Background = picked ? Token("SubtleSelectedBrush") : new SolidColorBrush(Microsoft.UI.Colors.Transparent),
        };
        AutomationProperties(button, $"{row.Number} {MapUiJson.SentenceCase(row.Name)}");
        button.Click += (_, _) => FocusItem(row.Index);
        return button;
    }

    private IEnumerable<UIElement> FenceList()
    {
        if (_status.FenceRefused) return [PlanFacts.Note(PlanRules.GeofenceRefused)];
        var empty = _fences.Polygons.Count == 0 && _fences.Circles.Count == 0 ? new UIElement[] { PlanFacts.Note(PlanRules.NoGeofence) } : [];
        var polygons = _fences.Polygons.Count > 0
            ? _fences.Polygons.Select(polygon => FenceRow(polygon.Title, $"{(polygon.Inclusion ? "Inclusion" : "Exclusion")}{(polygon.Detail.Length > 0 ? $" · {polygon.Detail}" : string.Empty)}",
                    string.Empty, polygon.Inclusion,
                    () => Select(new FenceVertexHit(polygon.Index, 0), true),
                    keep => Bridged($"Changing {polygon.Title.ToLowerInvariant()}", () => PlanCalls.SetInclusion(polygon.Index, keep)),
                    () => Bridged($"Removing {polygon.Title.ToLowerInvariant()}", () => PlanCalls.DeletePolygon(polygon.Index))))
                .Prepend(PlanFacts.Heading("Polygon fences"))
            : [];
        var circles = _fences.Circles.Count > 0
            ? _fences.Circles.Select(circle => FenceRow(circle.Title, circle.Inclusion ? "Inclusion" : "Exclusion",
                    string.Join(' ', new[] { PlanRules.RadiusText(circle.Radius), circle.Units }.Where(text => text.Length > 0)), null,
                    () => Select(new CircleHit(circle.Index), true),
                    null,
                    () => Bridged($"Removing {circle.Title.ToLowerInvariant()}", () => PlanCalls.DeleteCircle(circle.Index))))
                .Prepend(PlanFacts.Heading("Circular fences"))
            : [];
        return empty.Concat(polygons).Concat(circles);
    }

    private IEnumerable<UIElement> RallyList()
    {
        if (_status.RallyRefused) return [PlanFacts.Note(PlanRules.RallyRefused)];
        if (_fences.Rally.Count == 0) return [PlanFacts.Note(PlanRules.NoRally)];
        return _fences.Rally.Select(point =>
        {
            var height = point.Altitude is double altitude ? $"{PlanRules.Plain(altitude)} {(point.Units.Length > 0 ? point.Units : "m")}" : null;
            var where = string.Format(CultureInfo.InvariantCulture, "{0:F6}, {1:F6}", point.At.Latitude, point.At.Longitude);
            return FenceRow($"Rally {point.Index + 1}", string.Join(" · ", new[] { height, where }.OfType<string>()), string.Empty, null,
                () => Select(new RallyHit(point.Index), true), null, () => RemoveRally(point.Index));
        }).Prepend(PlanFacts.Heading("Rally points"));
    }

    private UIElement FenceRow(string title, string detail, string radius, bool? inclusion, Action select, Action<bool>? include, Action remove)
    {
        var text = new StackPanel { Spacing = 2, VerticalAlignment = VerticalAlignment.Center, Children = { PlanFacts.Text(title) } };
        if (detail.Length > 0) text.Children.Add(PlanFacts.Note(detail));
        var open = new Button
        {
            Content = text,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Left,
            BorderThickness = new Thickness(0),
            Background = new SolidColorBrush(Microsoft.UI.Colors.Transparent),
        };
        AutomationProperties(open, title);
        open.Click += (_, _) => select();
        var trailing = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 4, VerticalAlignment = VerticalAlignment.Center };
        if (radius.Length > 0) trailing.Children.Add(PlanFacts.Text(radius, "TextSecondaryBrush"));
        if (include != null && inclusion is bool keep)
        {
            var toggle = new ToggleSwitch { IsOn = keep, OnContent = string.Empty, OffContent = string.Empty, MinWidth = 0 };
            ToolTipService.SetToolTip(toggle, "Keep the vehicle inside");
            AutomationProperties(toggle, "Keep the vehicle inside");
            toggle.Toggled += (_, _) => include(toggle.IsOn);
            trailing.Children.Add(toggle);
        }
        var delete = new Button { Content = new FontIcon { Glyph = "", FontSize = 12 }, Padding = new Thickness(8), Background = new SolidColorBrush(Microsoft.UI.Colors.Transparent), BorderThickness = new Thickness(0) };
        AutomationProperties(delete, $"Remove {title}");
        ToolTipService.SetToolTip(delete, $"Remove {title}");
        delete.Click += (_, _) => remove();
        trailing.Children.Add(delete);
        return PlanFacts.Pair(open, trailing);
    }

    private void RebuildEditor(bool force)
    {
        var (identity, stamp) = EditorKey();
        var focused = XamlRoot != null && FocusManager.GetFocusedElement(XamlRoot) is DependencyObject element && Within(element, EditorHost);
        if (!force && identity == _editorIdentity && (stamp == _editorStamp || focused)) return;
        _editorIdentity = identity;
        _editorStamp = stamp;
        EditorHost.Content = _selected switch
        {
            WaypointHit w when _items.ElementAtOrDefault(w.Index) is { } item => WaypointEditor(item),
            SurveyVertexHit s => SurveyVertexTools(s),
            FenceVertexHit f => FenceTools(f),
            CircleHit c => CircleTools(c),
            RallyHit r => RallyTools(r),
            BreachHit => BreachTools(),
            null when _layer == PlanLayer.Mission => RouteSettings(),
            _ => null,
        };
    }

    private static bool Within(DependencyObject element, DependencyObject host) =>
        element == host || (VisualTreeHelper.GetParent(element) is { } parent && Within(parent, host));

    private (string Identity, string Stamp) EditorKey()
    {
        var item = ChosenItem();
        return _selected switch
        {
            WaypointHit w => ($"w{w.Index}", item == null ? string.Empty : $"{item.Name}|{item.Command}|{item.Altitude}|{item.AltitudeMode}|{item.FrameText}|{item.SpeedText}|{item.Ready}"),
            SurveyVertexHit s => ($"s{s.Item}.{s.Vertex}", string.Empty),
            FenceVertexHit f => ($"f{f.Polygon}.{f.Vertex}", $"{_fences.Polygons.FirstOrDefault(p => p.Index == f.Polygon)?.Inclusion}"),
            CircleHit c => ($"c{c.Index}", $"{_fences.Circles.FirstOrDefault(circle => circle.Index == c.Index)}"),
            RallyHit r => ($"r{r.Index}", $"{_fences.Rally.FirstOrDefault(point => point.Index == r.Index)}"),
            BreachHit => ("b", $"{_fences.Breach}"),
            null => ($"route{_layer}", $"{_status.Defaults.ToJsonString()}|{_status.GlobalFrame}|{_status.ContainsItems}"),
            _ => (string.Empty, string.Empty),
        };
    }

    private UIElement WaypointEditor(PlanItem item)
    {
        var tools = item.Shape != null ? SurveyTools(item) : null;
        var editor = new ItemEditor(item.Index, () => _items.ElementAtOrDefault(item.Index), () => _status, MapCentre, _dialogs, Changed,
            item.Index > PlanRules.HomeItem ? () => RemoveItem(item) : null);
        return tools == null ? editor : new StackPanel { Spacing = 12, Children = { tools, editor } };
    }

    private UIElement SurveyTools(PlanItem item)
    {
        var size = PlanFacts.Action("Size to view", () =>
        {
            var visible = Visible();
            var corners = PlanShapes.Inset(visible, 0.8);
            Bridged("Sizing the area", () => item.Shape is { } shape && corners.Count == shape.Vertices.Count
                && corners.Select((at, vertex) => PlanCalls.AdjustVertex(PlanCalls.ShapePath(item), vertex, at)).ToList().All(ok => ok),
                done: "Area sized to the view");
        });
        size.IsEnabled = item.Shape is { } outline && outline.Vertices.Count == 4;
        return new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, Children = { size } };
    }

    private UIElement ShapeButtons(string path, bool line, Action reselect)
    {
        var visible = Visible();
        var buttons = (line
                ? new[] { ("Line", (Func<IReadOnlyList<Geo>>)(() => PlanShapes.Line(visible))) }
                : new[] { ("Rectangle", (Func<IReadOnlyList<Geo>>)(() => PlanShapes.Rectangle(visible))), ("Circle", () => PlanShapes.Circle(visible)) })
            .Select(shape =>
            {
                var button = PlanFacts.Action(shape.Item1, () =>
                {
                    var vertices = shape.Item2();
                    Bridged($"Drawing {shape.Item1.ToLowerInvariant()}", () => PlanShapes.Replace(path, vertices, line), reselect);
                });
                button.IsEnabled = visible.Count == 4;
                return (UIElement)button;
            });
        var panel = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        buttons.ToList().ForEach(panel.Children.Add);
        return panel;
    }

    private UIElement SurveyVertexTools(SurveyVertexHit hit)
    {
        var item = _items.ElementAtOrDefault(hit.Item);
        var panel = new StackPanel { Spacing = 8 };
        if (item?.Shape is not { } shape) return panel;
        var path = PlanCalls.ShapePath(item);
        var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        if (_editable?.CanRemoveVertex == true)
            row.Children.Add(PlanFacts.Action("Remove vertex", () => Bridged("Removing corner", () => PlanCalls.RemoveVertex(path, hit.Vertex), () => Select(new WaypointHit(item.Index), true))));
        row.Children.Add(PlanFacts.Action("Edit position", () => EditVertex(shape.Vertices.ElementAtOrDefault(hit.Vertex), at => PlanCalls.AdjustVertex(path, hit.Vertex, at))));
        if (_editable?.Caption is { Length: > 0 } caption) panel.Children.Add(PlanFacts.Note(caption));
        panel.Children.Add(row);
        panel.Children.Add(ShapeButtons(path, shape.Line, () => Select(new WaypointHit(item.Index), true)));
        panel.Children.Add(PlanFacts.Action($"Delete {PatternName(item.Index)}", async () =>
        {
            var refusal = await PlanCalls.Refused(() => PlanCalls.Remove(item.Index));
            if (refusal != null) Say(refusal);
            Select(null, false);
            Changed();
        }, critical: true));
        return panel;
    }

    private async void EditVertex(Geo? at, Func<Geo, bool> move)
    {
        if (at is not { } from) return;
        if (await _dialogs.EditPosition(from, PlanCalls.HasVehicle(), "Edit vertex position") is { } moved)
            Bridged("Moving corner", () => move(moved), done: "Moved a corner");
    }

    private UIElement FenceTools(FenceVertexHit hit)
    {
        var polygon = _fences.Polygons.FirstOrDefault(p => p.Index == hit.Polygon);
        var panel = new StackPanel { Spacing = 8 };
        if (polygon == null) return panel;
        var path = PlanCalls.FencePath(polygon.Index);
        panel.Children.Add(PlanFacts.Action(polygon.Inclusion ? "Make keep-out" : "Make keep-in",
            () => Bridged(polygon.Inclusion ? "Making it keep-out" : "Making it keep-in", () => PlanCalls.SetInclusion(polygon.Index, !polygon.Inclusion))));
        if (_editable?.Caption is { Length: > 0 } caption) panel.Children.Add(PlanFacts.Note(caption));
        panel.Children.Add(ShapeButtons(path, false, () => Select(new FenceVertexHit(polygon.Index, 0), true)));
        var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        row.Children.Add(PlanFacts.Action("Edit position", () => EditVertex(polygon.Vertices.ElementAtOrDefault(hit.Vertex), at => PlanCalls.AdjustVertex(path, hit.Vertex, at))));
        if (_editable?.CanRemoveVertex == true)
            row.Children.Add(PlanFacts.Action("Remove vertex", () => Bridged("Removing corner", () => PlanCalls.RemoveVertex(path, hit.Vertex), () => Select(null, false))));
        panel.Children.Add(row);
        panel.Children.Add(PlanFacts.Action("Delete fence", () => Bridged("Removing fence", () => PlanCalls.DeletePolygon(polygon.Index), () => Select(null, false)), critical: true));
        return panel;
    }

    private UIElement CircleTools(CircleHit hit)
    {
        var circle = _fences.Circles.FirstOrDefault(c => c.Index == hit.Index);
        var panel = new StackPanel { Spacing = 8 };
        if (circle == null) return panel;
        var radius = PlanFacts.Entry(PlanRules.RadiusText(circle.Radius), typed =>
        {
            if (PlanRules.TypedRadius(typed, circle) is double wanted) Bridged("Changing the radius", () => PlanCalls.SetCircleRadius(circle.Index, wanted));
            else Say("Not a radius this fence accepts");
        }, 110);
        panel.Children.Add(PlanFacts.Pair(PlanFacts.Text("Radius"), new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6, Children = { radius, PlanFacts.Note(circle.Units) } }));
        var bigger = PlanFacts.Action("Bigger", () =>
        {
            if (PlanRules.Grown(circle) is double wanted) Bridged(null, () => PlanCalls.SetCircleRadius(circle.Index, wanted));
        });
        bigger.IsEnabled = PlanRules.Grown(circle) != null;
        var smaller = PlanFacts.Action("Smaller", () =>
        {
            if (PlanRules.Shrunk(circle) is double wanted) Bridged(null, () => PlanCalls.SetCircleRadius(circle.Index, wanted));
        });
        smaller.IsEnabled = PlanRules.Shrunk(circle) != null;
        var keep = PlanFacts.Action(circle.Inclusion ? "Make keep-out" : "Make keep-in",
            () => Bridged(circle.Inclusion ? "Making it keep-out" : "Making it keep-in", () => PlanCalls.SetCircleInclusion(circle.Index, !circle.Inclusion)));
        panel.Children.Add(new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, Children = { bigger, smaller, keep } });
        panel.Children.Add(PlanFacts.Action("Delete circle", () => Bridged("Removing circle", () => PlanCalls.DeleteCircle(circle.Index), () => Select(null, false)), critical: true));
        return panel;
    }

    private UIElement RallyTools(RallyHit hit)
    {
        var point = _fences.Rally.FirstOrDefault(p => p.Index == hit.Index);
        var panel = new StackPanel { Spacing = 8 };
        if (point == null) return panel;
        TextBox Coordinate(string label, double value, double limit, Func<double, Geo> moved) =>
            PlanFacts.Entry(value.ToString("F7", CultureInfo.InvariantCulture), typed =>
            {
                if (PlanRules.TypedCoordinate(typed, limit) is double entered)
                    Bridged("Moving rally point", () => PlanCalls.MoveRally(point.Index, moved(entered), point.AltitudeMetres));
                else Say($"Not a {label}");
            });
        panel.Children.Add(PlanFacts.Pair(PlanFacts.Text("Latitude"), Coordinate("Latitude", point.At.Latitude, 90, entered => new Geo(entered, point.At.Longitude))));
        panel.Children.Add(PlanFacts.Pair(PlanFacts.Text("Longitude"), Coordinate("Longitude", point.At.Longitude, 180, entered => new Geo(point.At.Latitude, entered))));
        if (point.Altitude is double altitude && point.AltitudePath.Length > 0)
        {
            var box = PlanFacts.Entry(PlanRules.Plain(Math.Round(altitude, 2)), typed =>
            {
                if (PlanRules.TypedNumber(typed) is double shown) Bridged("Setting altitude", () => PlanCalls.SetNumber(point.AltitudePath, shown));
                else Say("Not an altitude");
            }, 110);
            panel.Children.Add(PlanFacts.Pair(PlanFacts.Text($"Alt {(point.Units.Length > 0 ? point.Units : "m")}"), box));
        }
        panel.Children.Add(PlanFacts.Action("Delete rally point", () => RemoveRally(point.Index), critical: true));
        return panel;
    }

    private UIElement BreachTools()
    {
        var panel = new StackPanel { Spacing = 8 };
        if (_fences.Breach is not { } breach) return panel;
        if (breach.AltitudePath.Length > 0)
        {
            var box = PlanFacts.Entry(breach.Altitude is double altitude ? PlanRules.Plain(Math.Round(altitude, 2)) : string.Empty, typed =>
            {
                if (PlanRules.TypedNumber(typed) is double shown) Bridged("Setting breach return altitude", () => PlanCalls.SetNumber(breach.AltitudePath, shown));
                else Say("Not an altitude");
            }, 110);
            panel.Children.Add(PlanFacts.Pair(PlanFacts.Text("Altitude"), new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6, Children = { box, PlanFacts.Note(breach.Units) } }));
        }
        panel.Children.Add(PlanFacts.Note(string.Format(CultureInfo.InvariantCulture, "{0:F6}, {1:F6}", breach.At.Latitude, breach.At.Longitude)));
        panel.Children.Add(PlanFacts.Action("Remove breach return point", () => Bridged("Removing breach return point", () => PlanCalls.SetBreach(null), () => Select(null, false)), critical: true));
        return panel;
    }

    private UIElement? RouteSettings()
    {
        var defaults = _status.Defaults;
        var panel = new StackPanel { Spacing = 8 };
        if (defaults["altitude"] is JsonObject altitude && MapUiJson.Text(altitude["path"]) is { Length: > 0 } path)
            panel.Children.Add(PlanFacts.Stepper("Altitude", MapUiJson.Number(altitude["value"]) ?? PlanRules.TypedNumber(MapUiJson.Text(altitude["valueString"])),
                MapUiJson.Text(altitude["units"]), 1, "New waypoints use this",
                wanted => RouteWrite(() => PlanCalls.Set(path, wanted))));
        PlanFacts.AltitudeFrame(_status.GlobalFrame, raw => RouteWrite(() => PlanCalls.Set(PlanCalls.GlobalAltitudeMode, raw))).ToList().ForEach(panel.Children.Add);
        if (defaults["flightSpeed"] is JsonObject speed && MapUiJson.Flag(speed["available"]))
        {
            var specified = MapUiJson.Flag(speed["specified"]);
            var specify = MapUiJson.Text(speed["specifyPath"]);
            var speedPath = MapUiJson.Text(speed["path"]);
            var auto = specified ? PlanFacts.Action("Auto", () => RouteWrite(() => PlanCalls.Set(specify, false))) : null;
            panel.Children.Add(PlanFacts.Stepper("Speed", MapUiJson.Number(speed["value"]), MapUiJson.Text(speed["units"]), 0.5,
                specified ? null : "Vehicle default",
                wanted => RouteWrite(() => PlanCalls.Set(specify, true) is { Ok: false } refused ? refused : PlanCalls.Set(speedPath, wanted)), auto, suggested: !specified));
        }
        if (panel.Children.Count == 0) return null;
        panel.Children.Insert(0, PlanFacts.Heading("Route"));
        return panel;
    }

    private async void RouteWrite(Func<Outcome> write)
    {
        var outcome = await PlanCalls.Queued(write);
        if (!outcome.Ok) Say(outcome.Reason);
        RebuildEditor(true);
    }
}
