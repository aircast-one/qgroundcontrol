using System;
using System.Collections.Immutable;
using System.Linq;
using System.Numerics;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Hosting;
using Microsoft.UI.Xaml.Input;
using Windows.System;

namespace AircastQGC;

public sealed partial class MainWindow : Window
{
    private const double TileMargin = 16;
    private const int FullScreenLayer = 100;
    private const double LeftTop = 96;
    private const double BottomGap = 24;
    private const double StatsRoom = 40;
    private const double ScaleRoom = 48;
    private const double PanelTop = 122;
    private const double FleetFloor = 120;
    private const double CornerRoom = 80;

    private static readonly long DoubleClickMs = new Windows.UI.ViewManagement.UISettings().DoubleClickTime;

    private readonly FlightModeMenu _modes;
    private ImmutableStack<string> _history = ImmutableStack<string>.Empty;
    private string _page = NavPane.Fly;
    private ImmutableDictionary<string, UIElement> _pages = ImmutableDictionary<string, UIElement>.Empty;
    private bool _videoMain;
    private bool _threeD = FlyViewStore.LoadThreeD();
    private bool _fullScreen;
    private AppWindowPresenter? _windowed;
    private long _swappedAt;

    public MainWindow()
    {
        Theme.Appearance.Prepare();
        InitializeComponent();
        BridgeWatch.BindDispatcher(DispatcherQueue);
        Theme.Appearance.Attach((FrameworkElement)Content, AppWindow);
        Pages.Setup.GamepadInput.Start();
        ScreenWake.Start();

        ExtendsContentIntoTitleBar = true;
        AppWindow.TitleBar.PreferredHeightOption = TitleBarHeightOption.Tall;
        AppWindow.SetIcon(System.IO.Path.Combine(AppContext.BaseDirectory, "Assets", "AppIcon.ico"));
        SetTitleBar(Shell.DragRegion);
        Shell.Loaded += (_, _) =>
        {
            FitCaption();
            Shell.XamlRoot.Changed += (_, _) => FitCaption();
        };
        FlyRequests.Connections += OpenLinks;
        FlyRequests.Settings += OpenSettings;
        FlyRequests.Setup += OpenSetup;
        FlyRequests.Guided += AskGuided;
        FlyRequests.Messages += OpenMessages;
        FlyRequests.Fly += () => Go(NavPane.Fly);

        _modes = new FlightModeMenu();
        Shell.FlightModeRequested += (_, anchor) => _modes.Show(anchor);
        _modes.StateChanged += (_, _) => Shell.PendingMode = _modes.Requested;
        _modes.Refused += (_, reason) => Warn(reason);

        Nav.NavigationRequested += (_, page) => Go(page);
        Nav.BackRequested += (_, _) => GoBack();
        Shell.PageRequested += (_, page) => Go(page);
        Shell.BackRequested += (_, _) => GoBack();
        Notices.NavigateRequested += (_, page) => Go(page);

        NoVehicle.VehiclePresenceChanged += (_, present) => ShowVehicleOverlays(present);
        NoVehicle.OpenConnectionsRequested += (_, _) => OpenLinks();
        NoVehicle.OpenFlasherRequested += (_, _) => OpenSettings(Pages.Settings.Flasher.Page);

        MapButtons.ZoomInRequested += (_, _) =>
        {
            if (_threeD) Viewer.ZoomIn();
            else Map.ZoomIn();
        };
        MapButtons.ZoomOutRequested += (_, _) =>
        {
            if (_threeD) Viewer.ZoomOut();
            else Map.ZoomOut();
        };
        MapButtons.CenterOnVehicleRequested += (_, _) =>
        {
            if (_threeD) Viewer.FollowVehicle();
            else Map.CenterOnVehicle();
        };
        MapButtons.ThreeDRequested += (_, _) => ShowThreeD(!_threeD);
        Viewer.EnabledChanged += (_, _) => ShowThreeD(_threeD && Viewer.Enabled);
        Map.MapRightTapped += (latitude, longitude, at) =>
        {
            if (MapMenu.Show(latitude, longitude, Map.TransformToVisual(MapMenu).TransformPoint(at)))
                Map.ShowClickMarker(latitude, longitude);
        };
        MapMenu.OpenChanged += (_, _) =>
        {
            if (MapMenu.Target == null) Map.HideClickMarker();
        };
        MapMenu.ConfirmRequested += (_, ask) => GuidedCard.Ask(ask);
        Map.VehicleTapped += id =>
        {
            if (_videoMain) SwapVideo(false);
            else if (FleetBridge.AskFor(id) is { } refusal) Warn(refusal);
        };
        FleetPanel.ConfirmRequested += (_, ask) => GuidedCard.Ask(ask);
        Shell.VehiclePicker.ConfirmRequested += (_, ask) =>
        {
            Go(NavPane.Fly);
            if (_page == NavPane.Fly) GuidedCard.Ask(ask);
        };
        Shell.VehiclePicker.ConnectRequested += (_, _) => OpenLinks();
        Shell.VehiclePicker.HandedOver += (_, notice) => Tell(notice);

        GuidedStrip.ActionRequested += (_, offer) => GuidedCard.Show(offer);
        GuidedStrip.CutRequested += (_, offer) =>
        {
            if (GuidedCard.Show(offer)) GuidedCard.Confirm();
        };
        GuidedCard.Failed += (_, reason) =>
        {
            if (!GuidedCard.IsOpen) Warn(reason);
        };

        VideoTile.SwapRequested += (_, _) => SwapVideo(!_videoMain);
        VideoTile.Pill = NoVideoMessage;
        AimSlot.Child = VideoTile.AimRail;
        PictureTags.ToList().ForEach(tag =>
        {
            tag.SizeChanged += (_, _) => Clearances();
            tag.RegisterPropertyChangedCallback(UIElement.VisibilityProperty, (_, _) => Clearances());
        });
        PipFrame.Click += (_, _) => SwapVideo(false);
        ReadingsBar.AttitudeChanged += (_, _) => ShowVehicleOverlays(ReadingsBar.Visibility == Visibility.Visible);
        CameraPanel.CameraPresentChanged += (_, _) => CameraButton.Visibility = Shown(CameraPanel.CameraPresent);
        CameraButton.Click += (_, _) => ToggleCamera();
        MapPayload.Attach(Map);
        ObstacleOnMap.Attach(Map);
        Marks.ZoomFrom(FlyContent, VideoTile, VideoLayer);
        VideoTile.PictureChanged += (_, _) =>
        {
            Marks.Pictured = VideoTile.Picture;
            VideoLayer.Pictured = VideoTile.Picture;
            if (!VideoTile.Picture) FullScreen(false);
            PaintFullScreen();
        };
        Marks.ExitFullScreenRequested += (_, _) => FullScreen(false);
        Marks.FullScreenToggled += (_, _) => FullScreen(!_fullScreen);
        FullScreenButton.Click += (_, _) => FullScreen(true);
        FlyContent.KeyboardAcceleratorPlacementMode = KeyboardAcceleratorPlacementMode.Hidden;
        FlyContent.KeyboardAccelerators.Add(Accelerator(VirtualKey.F11, () => true, () => FullScreen(!_fullScreen)));
        FlyContent.KeyboardAccelerators.Add(Accelerator(VirtualKey.Escape, () => _fullScreen || OverlayLayout.Editing, () =>
        {
            if (_fullScreen) FullScreen(false);
            else OverlayLayout.StopEditing();
        }));
        BottomLeft.SizeChanged += (_, _) => Clearances();
        LeftRail.SizeChanged += (_, _) => Clearances();
        TelemetryCard.SizeChanged += (_, _) => Clearances();
        MapCorner.SizeChanged += (_, _) => Clearances();
        NoVehicle.SizeChanged += (_, _) => Clearances();
        RightColumn.SizeChanged += (_, _) => Clearances();
        FlyChrome.Loaded += (_, _) => Clearances();
        FlyChrome.SizeChanged += (_, _) => Clearances();
        LeftColumn.SizeChanged += (_, _) => Clearances();
        RcLayer.RegisterPropertyChangedCallback(UIElement.VisibilityProperty, (_, _) => Clearances());
        Map.MapTapped += (_, _, at) =>
        {
            if (_videoMain) SwapVideo(false);
            else if (RoiTap.At(Map, at) is { } roi) GuidedCard.Ask(RoiTap.Ask(roi));
            else if (SetWaypoint.PinAt(Map, at) is { } sequence) GuidedCard.Ask(SetWaypoint.Ask(sequence));
        };
        MissionProgress.SkipRequested += (_, next) => GuidedCard.Ask(SetWaypoint.Ask(next));
        Gripper.ConfirmRequested += (_, ask) => GuidedCard.Ask(ask);
        GuidedStrip.PayloadRequested += (_, _) => ToggleGripper();

        Checklist.StateChanged += (_, _) => PaintChecklistButton();
        GuidedStrip.DeckChanged += (_, _) => PaintChecklistButton();
        GuidedStrip.ChecklistRequested += (_, _) => ToggleChecklist();
        Checklist.RegisterPropertyChangedCallback(UIElement.VisibilityProperty, (_, _) => FleetColumn.Visibility = Shown(!Checklist.IsOpen));

        var root = (FrameworkElement)Content;
        _ = new ChecklistGate(Checklist, () => GuidedCard.IsOpen || MapMenu.Target != null);
        _ = new MissionPopups(GuidedCard);
        _ = new MissionCompleteDialog(root);
        _ = new ResumeFailedPrompt(FlyContent);
        _ = new FirstRunDialog(root);
        _ = new CloseGuard(this);
        new FrameworkElement[] { VideoTile, Marks, VideoLayer, PipFrame }.ToList().ForEach(view => Dock(view, false));
        Map.FollowInsets = new Thickness(Map.FollowInsets.Left, Map.FollowInsets.Top + Shell.Height, Map.FollowInsets.Right, Map.FollowInsets.Bottom);
        ShowPage(NavPane.Fly);
    }

    private void FitCaption()
    {
        Shell.CaptionInset = AppWindow.TitleBar.RightInset / Shell.XamlRoot.RasterizationScale;
        ReturnBar.Margin = new Thickness(0, 0, Shell.CaptionInset, 0);
    }

    private void Go(string page)
    {
        if (page == _page || !Pages.NavigationLock.Allows()) return;
        _history = _history.Push(_page);
        ShowPage(page);
    }

    private void OpenLinks() => OpenSettings(Pages.Settings.SettingsPage.ConnectionsPage);

    private void OpenSettings(string page)
    {
        Go(NavPane.Settings);
        if (_page == NavPane.Settings)
            (HostedPage(NavPane.Settings) as Pages.Settings.SettingsPage)?.Open(page);
    }

    private void OpenSetup(string page)
    {
        Go(NavPane.VehicleSetup);
        if (_page == NavPane.VehicleSetup)
            (HostedPage(NavPane.VehicleSetup) as Pages.Setup.SetupPage)?.Open(page);
    }

    private void OpenMessages()
    {
        Go(NavPane.Fly);
        if (_page == NavPane.Fly) MessageBanner.OpenSheet();
    }

    private void AskGuided(string id)
    {
        Go(NavPane.Fly);
        if (_page != NavPane.Fly) return;
        var offer = GuidedOffer.List(Bridge.Get(GuidedOffer.ViewPath)).FirstOrDefault(o => o.Id == id);
        if (offer is { Ready: true }) GuidedCard.Show(offer);
        else Warn(offer?.Reason is { Length: > 0 } reason ? reason : ArmControls.Unavailable);
    }

    private void GoBack()
    {
        if (_history.IsEmpty || !Pages.NavigationLock.Allows()) return;
        _history = _history.Pop(out var previous);
        ShowPage(previous);
    }

    private void ShowPage(string page)
    {
        var fly = page == NavPane.Fly;
        if (!fly) FullScreen(false);
        _page = page;
        Nav.SelectedPage = page;
        Nav.CanGoBack = !_history.IsEmpty;
        Nav.Visibility = Shown(!fly);
        Shell.CurrentPage = page;
        Shell.CanGoBack = !_history.IsEmpty;
        Shell.OnFly = fly;
        Theme.Appearance.OverVideo(fly);
        var hosted = fly ? null : HostedPage(page);
        FlyContent.Visibility = Shown(fly);
        FlyChrome.Visibility = Shown(fly);
        PageHost.Content = hosted;
        PageHost.Visibility = Shown(hosted != null);
        PagePlaceholder.Visibility = Shown(page != NavPane.Fly && hosted == null);
        PageTitle.Text = page;
        Notices.OnFly = page == NavPane.Fly;
    }

    private UIElement? HostedPage(string page)
    {
        if (_pages.TryGetValue(page, out var built)) return built;
        var created = Pages.PageRegistry.Create(page);
        if (created is Pages.Plan.PlanPage plan) plan.FlyRequested += (_, _) => Go(NavPane.Fly);
        if (created is Pages.Setup.SetupPage setup) setup.OpenConnectionsRequested += (_, _) => OpenLinks();
        if (created != null) _pages = _pages.SetItem(page, created);
        return created;
    }

    private void ShowVehicleOverlays(bool present)
    {
        ReadingsBar.Visibility = Shown(present);
        TelemetryCard.Visibility = Shown(present && ReadingsBar.AttitudeShown);
        GuidedStrip.Visibility = Shown(present);
        if (!present && Checklist.IsOpen) Checklist.Hide();
        Clearances();
    }

    private FrameworkElement[] PictureTags => [VideoTile.CameraTag, VideoLayer.AimTag];

    private double RailTop() =>
        PictureTags
            .Where(tag => tag is { Visibility: Visibility.Visible, ActualHeight: > 0 })
            .Select(tag => tag.TransformToVisual(FlyChrome).TransformPoint(new Windows.Foundation.Point(0, tag.ActualHeight)).Y + TileMargin)
            .Append(TileMargin)
            .Max();

    private void Clearances()
    {
        var bar = BottomLeft.ActualHeight > 0 ? BottomLeft.ActualHeight + BottomGap : TileMargin;
        var rail = LeftRail.ActualWidth > 0 ? LeftRail.ActualWidth + TileMargin : 0;
        var top = RailTop();
        LeftRail.Margin = new Thickness(TileMargin, top, 0, bar);
        GuidedStrip.Room = FlyChrome.ActualHeight - top - bar;
        GuidedCard.Margin = new Thickness(TileMargin + rail, TileMargin, TileMargin, bar);
        LeftColumn.Margin = new Thickness(TileMargin + rail, LeftTop, TileMargin, bar + StatsRoom);
        FleetScroll.MaxHeight = RcLayer.Visibility == Visibility.Visible
            ? Math.Max(FleetFloor, (FlyChrome.ActualHeight - LeftTop - bar - StatsRoom) / 2)
            : double.PositiveInfinity;
        Checklist.Margin = new Thickness(TileMargin + rail, TileMargin, TileMargin, bar);
        var attitude = TelemetryCard.Visibility == Visibility.Visible;
        var corner = attitude ? TelemetryCard.ActualHeight + BottomGap : TileMargin;
        BottomLeft.Margin = new Thickness(BottomLeft.Margin.Left, BottomLeft.Margin.Top,
            attitude ? TelemetryCard.ActualWidth + TileMargin * 2 : CornerRoom, BottomLeft.Margin.Bottom);
        Map.ScaleInCorner = !_videoMain;
        Map.ScaleHidden = _videoMain;
        Map.ScaleMargin = new Thickness(TileMargin, TileMargin, TileMargin, _videoMain ? TileMargin : corner);
        MapCorner.Margin = new Thickness(TileMargin, TileMargin, TileMargin, corner + ScaleRoom);
        LayoutBar.Margin = new Thickness(TileMargin, TileMargin, TileMargin, bar);
        var card = NoVehicle.Visibility == Visibility.Visible && NoVehicle.Content is FrameworkElement shown ? shown.ActualHeight : 0;
        CenterMessage.VerticalAlignment = card > 0 ? VerticalAlignment.Bottom : VerticalAlignment.Center;
        CenterMessage.Margin = new Thickness(TileMargin, TileMargin, TileMargin, card > 0 ? (FlyChrome.ActualHeight + card) / 2 + TileMargin : TileMargin);
        VideoStats.Margin = new Thickness(TileMargin, TileMargin, TileMargin, bar);
        var right = MapCorner.Visibility == Visibility.Visible ? MapCorner.Margin.Bottom + MapCorner.ActualHeight + TileMargin : corner;
        RightColumn.Margin = new Thickness(TileMargin, TileMargin, TileMargin, right);
        var panels = new Thickness(TileMargin, PanelTop, RightColumn.ActualWidth + TileMargin * 2, right);
        CameraPanel.Margin = panels;
        Gripper.Margin = panels;
    }

    private void Warn(string reason)
    {
        Notice.Severity = InfoBarSeverity.Error;
        Notice.Message = reason;
        Notice.IsOpen = true;
    }

    private void Tell(string notice)
    {
        Notice.Severity = InfoBarSeverity.Warning;
        Notice.Message = notice;
        Notice.IsOpen = true;
    }

    private void SwapVideo(bool videoMain)
    {
        if (Environment.TickCount64 - _swappedAt < DoubleClickMs)
        {
            FullScreen(true);
            return;
        }
        _swappedAt = Environment.TickCount64;
        if (!videoMain) FullScreen(false);
        _videoMain = videoMain;
        VideoTile.Expanded = videoMain;
        Marks.Expanded = videoMain;
        VideoLayer.Expanded = videoMain;
        new FrameworkElement[] { VideoTile, Marks, VideoLayer }.ToList().ForEach(view =>
        {
            Dock(view, videoMain);
            Canvas.SetZIndex(view, videoMain ? -1 : 0);
        });
        new FrameworkElement[] { Map, MapPayload, ObstacleOnMap }.ToList().ForEach(view =>
        {
            Dock(view, !videoMain);
            RoundPip(view, videoMain);
        });
        ObstacleOnMap.ShowText = !videoMain;
        PipFrame.Visibility = Shown(videoMain);
        MapCorner.Visibility = Shown(!videoMain);
        Clearances();
        VideoStats.Expanded = videoMain;
        Tape.OverVideo = videoMain;
        PaintFullScreen();
        ShowThreeD(_threeD);
    }

    private void ShowThreeD(bool on)
    {
        if (on != _threeD) FlyViewStore.SaveThreeD(on);
        _threeD = on;
        Viewer.Visibility = Shown(on && !_videoMain);
        MapButtons.ThreeD(Viewer.Enabled, on);
    }

    private void FullScreen(bool on)
    {
        var shown = on && _videoMain && VideoTile.Picture && _page == NavPane.Fly;
        if (shown == _fullScreen) return;
        _fullScreen = shown;
        _windowed = shown ? AppWindow.Presenter : _windowed;
        if (shown) AppWindow.SetPresenter(AppWindowPresenterKind.FullScreen);
        else if (_windowed is not null) AppWindow.SetPresenter(_windowed);
        Shell.Visibility = Shown(!shown);
        TopScrim.Visibility = Shown(!shown);
        Grid.SetRowSpan(FlyContent, shown ? 3 : 2);
        Canvas.SetZIndex(FlyContent, shown ? 1 : 0);
        new FrameworkElement[] { VideoTile, Marks, VideoLayer }.Select((view, at) => (view, at)).ToList()
            .ForEach(entry => Canvas.SetZIndex(entry.view, shown ? FullScreenLayer + entry.at : _videoMain ? -1 : 0));
        Marks.FullScreen = shown;
        PipFrame.Visibility = Shown(_videoMain && !shown);
        PaintFullScreen();
    }

    private void PaintFullScreen() =>
        FullScreenButton.Visibility = Shown(_videoMain && VideoTile.Picture && !_fullScreen);

    private static KeyboardAccelerator Accelerator(VirtualKey key, Func<bool> owns, Action act)
    {
        var accelerator = new KeyboardAccelerator { Key = key };
        accelerator.Invoked += (_, e) =>
        {
            e.Handled = owns();
            act();
        };
        return accelerator;
    }

    private void Dock(FrameworkElement view, bool full)
    {
        view.Width = full ? double.NaN : PipSlot.Width;
        view.Height = full ? double.NaN : PipSlot.Height;
        view.Margin = full ? new Thickness(0) : new Thickness(BottomLeft.Margin.Left, 0, 0, BottomLeft.Margin.Bottom);
        view.HorizontalAlignment = full ? HorizontalAlignment.Stretch : HorizontalAlignment.Left;
        view.VerticalAlignment = full ? VerticalAlignment.Stretch : VerticalAlignment.Bottom;
    }

    private void RoundPip(UIElement view, bool docked)
    {
        var visual = ElementCompositionPreview.GetElementVisual(view);
        var shape = visual.Compositor.CreateRoundedRectangleGeometry();
        shape.Size = new Vector2((float)PipSlot.Width, (float)PipSlot.Height);
        shape.CornerRadius = new Vector2((float)((CornerRadius)Application.Current.Resources["RadiusOverlay"]).TopLeft);
        visual.Clip = docked ? visual.Compositor.CreateGeometricClip(shape) : null;
    }

    private void ToggleCamera()
    {
        if (CameraPanel.Visibility == Visibility.Visible) CameraPanel.Hide();
        else
        {
            Gripper.Hide();
            CameraPanel.Show();
        }
    }

    private void ToggleGripper()
    {
        if (Gripper.IsOpen)
        {
            Gripper.Hide();
            return;
        }
        CameraPanel.Hide();
        Gripper.Show();
    }

    private void ToggleChecklist()
    {
        if (Checklist.IsOpen) Checklist.Hide();
        else Checklist.Show();
    }

    private void PaintChecklistButton() =>
        GuidedStrip.ShowChecklist(Checklist.HasVehicle && !GuidedStrip.OnFlyingDeck,
            Checklist.Ready ? "Pre-flight checklist · Ready" : $"Pre-flight checklist · {Checklist.Progress}");

    private static Visibility Shown(bool on) => on ? Visibility.Visible : Visibility.Collapsed;
}
