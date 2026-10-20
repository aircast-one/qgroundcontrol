using System;
using System.Collections.Immutable;
using System.Linq;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Windows.System;

namespace AircastQGC;

public sealed partial class MainWindow : Window
{
    private const double TileWidth = 260;
    private const double TileHeight = 150;
    private const double TileMargin = 16;
    private const int FullScreenLayer = 100;
    private const double LeftTop = 96;
    private const double BottomGap = 24;
    private const double StatsRoom = 40;
    private const double ShutterInset = 68;
    private const double FleetFloor = 120;

    private readonly FlightModeMenu _modes;
    private ImmutableStack<string> _history = ImmutableStack<string>.Empty;
    private string _page = NavPane.Fly;
    private ImmutableDictionary<string, UIElement> _pages = ImmutableDictionary<string, UIElement>.Empty;
    private bool _videoMain;
    private bool _fullScreen;
    private AppWindowPresenter? _windowed;

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
        Notices.NavigateRequested += (_, page) => Go(page);

        NoVehicle.VehiclePresenceChanged += (_, present) => ShowVehicleOverlays(present);
        NoVehicle.OpenConnectionsRequested += (_, _) => OpenLinks();

        MapButtons.ZoomInRequested += (_, _) => Map.ZoomIn();
        MapButtons.ZoomOutRequested += (_, _) => Map.ZoomOut();
        MapButtons.CenterOnVehicleRequested += (_, _) => Map.CenterOnVehicle();
        Map.MapRightTapped += (latitude, longitude, at) =>
        {
            if (MapMenu.Show(latitude, longitude, Map.TransformToVisual(FlyContent).TransformPoint(at)))
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
        GuidedCard.Failed += (_, reason) =>
        {
            if (!GuidedCard.IsOpen) Warn(reason);
        };

        VideoTile.SwapRequested += (_, _) => SwapVideo(!_videoMain);
        CameraPanel.CameraPresentChanged += (_, _) => CameraButton.Visibility = Shown(CameraPanel.CameraPresent);
        CameraButton.Click += (_, _) => ToggleCamera();
        MapPayload.Attach(Map);
        VideoLayer.SwapRequested += (_, _) => SwapVideo(!_videoMain);
        ObstacleOnMap.Attach(Map);
        Marks.ZoomFrom(FlyContent, VideoTile, VideoLayer);
        Marks.DecodingChanged += (_, _) =>
        {
            if (!Marks.Decoding) FullScreen(false);
            PaintFullScreen();
        };
        Marks.ExitFullScreenRequested += (_, _) => FullScreen(false);
        FullScreenButton.Click += (_, _) => FullScreen(true);
        FlyContent.KeyboardAcceleratorPlacementMode = KeyboardAcceleratorPlacementMode.Hidden;
        FlyContent.KeyboardAccelerators.Add(Accelerator(VirtualKey.F11, () => true, () => FullScreen(!_fullScreen)));
        FlyContent.KeyboardAccelerators.Add(Accelerator(VirtualKey.Escape, () => _fullScreen || OverlayLayout.Editing, () =>
        {
            if (_fullScreen) FullScreen(false);
            else OverlayLayout.StopEditing();
        }));
        ReadingsBar.SizeChanged += (_, _) => Clearances();
        TelemetryCard.SizeChanged += (_, _) => Clearances();
        FlyContent.Loaded += (_, _) => Clearances();
        FlyContent.SizeChanged += (_, _) => Clearances();
        GuidedStrip.SizeChanged += (_, _) => Clearances();
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
        ChecklistButton.Click += (_, _) => ToggleChecklist();
        Checklist.RegisterPropertyChangedCallback(UIElement.VisibilityProperty, (_, _) => FleetColumn.Visibility = Shown(!Checklist.IsOpen));

        var root = (FrameworkElement)Content;
        _ = new ChecklistGate(Checklist, () => GuidedCard.IsOpen || MapMenu.Target != null);
        _ = new MissionPopups(GuidedCard);
        _ = new MissionCompleteDialog(root);
        _ = new ResumeFailedPrompt(FlyContent);
        _ = new FirstRunDialog(root);
        _ = new CloseGuard(this);
    }

    private void FitCaption() =>
        Shell.CaptionInset = AppWindow.TitleBar.RightInset / Shell.XamlRoot.RasterizationScale;

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
        if (page != NavPane.Fly) FullScreen(false);
        _page = page;
        Nav.SelectedPage = page;
        Nav.CanGoBack = !_history.IsEmpty;
        var hosted = page == NavPane.Fly ? null : HostedPage(page);
        FlyContent.Visibility = Shown(page == NavPane.Fly);
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
        TelemetryCard.Visibility = Shown(present);
        GuidedStrip.Visibility = Shown(present);
        if (!present && Checklist.IsOpen) Checklist.Hide();
        Clearances();
    }

    private void Clearances()
    {
        var bar = ReadingsBar.Visibility == Visibility.Visible ? ReadingsBar.ActualHeight + BottomGap : TileMargin;
        var guided = GuidedStrip.Visibility == Visibility.Visible && GuidedStrip.ActualWidth > 0
            ? GuidedStrip.TransformToVisual(FlyContent).TransformBounds(new Windows.Foundation.Rect(0, 0, GuidedStrip.ActualWidth, GuidedStrip.ActualHeight))
            : Windows.Foundation.Rect.Empty;
        var underGuided = !guided.IsEmpty && guided.Left < TileMargin + LeftColumn.ActualWidth + TileMargin;
        var leftTop = underGuided ? Math.Max(LeftTop, guided.Bottom + TileMargin) : LeftTop;
        LeftColumn.Margin = new Thickness(TileMargin, leftTop, TileMargin, bar + StatsRoom);
        FleetScroll.MaxHeight = RcLayer.Visibility == Visibility.Visible
            ? Math.Max(FleetFloor, (FlyContent.ActualHeight - leftTop - bar - StatsRoom) / 2)
            : double.PositiveInfinity;
        Checklist.Margin = new Thickness(TileMargin, TileMargin, TileMargin, bar);
        LayoutBar.Margin = new Thickness(TileMargin, TileMargin, TileMargin, bar);
        VideoStats.Margin = new Thickness(TileMargin, TileMargin, TileMargin, bar);
        var top = TileMargin + TileHeight + TileMargin;
        var right = TelemetryCard.Visibility == Visibility.Visible ? TelemetryCard.ActualHeight + BottomGap : TileMargin;
        RightColumn.Margin = new Thickness(TileMargin, top, TileMargin, right);
        Shutter.Margin = new Thickness(TileMargin, top, ShutterInset, right);
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
        new FrameworkElement[] { Map, MapPayload, ObstacleOnMap }.ToList().ForEach(view => Dock(view, !videoMain));
        ObstacleOnMap.ShowText = !videoMain;
        VideoStats.Expanded = videoMain;
        PaintFullScreen();
    }

    private void FullScreen(bool on)
    {
        var shown = on && _videoMain && Marks.Decoding && _page == NavPane.Fly;
        if (shown == _fullScreen) return;
        _fullScreen = shown;
        _windowed = shown ? AppWindow.Presenter : _windowed;
        if (shown) AppWindow.SetPresenter(AppWindowPresenterKind.FullScreen);
        else if (_windowed is not null) AppWindow.SetPresenter(_windowed);
        Shell.Visibility = Shown(!shown);
        Nav.Visibility = Shown(!shown);
        Grid.SetRowSpan(FlyContent, shown ? 2 : 1);
        Canvas.SetZIndex(FlyContent, shown ? 1 : 0);
        new FrameworkElement[] { VideoTile, Marks, VideoLayer }.Select((view, at) => (view, at)).ToList()
            .ForEach(entry => Canvas.SetZIndex(entry.view, shown ? FullScreenLayer + entry.at : _videoMain ? -1 : 0));
        Marks.FullScreen = shown;
        PaintFullScreen();
    }

    private void PaintFullScreen() =>
        FullScreenButton.Visibility = Shown(_videoMain && Marks.Decoding && !_fullScreen);

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

    private static void Dock(FrameworkElement view, bool full)
    {
        view.Width = full ? double.NaN : TileWidth;
        view.Height = full ? double.NaN : TileHeight;
        view.Margin = new Thickness(full ? 0 : TileMargin);
        view.HorizontalAlignment = full ? HorizontalAlignment.Stretch : HorizontalAlignment.Right;
        view.VerticalAlignment = full ? VerticalAlignment.Stretch : VerticalAlignment.Top;
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

    private void PaintChecklistButton()
    {
        ChecklistButton.IsEnabled = Checklist.HasVehicle;
        ToolTipService.SetToolTip(ChecklistButton,
            Checklist.Ready ? "Pre-flight checklist · Ready" : $"Pre-flight checklist · {Checklist.Progress}");
    }

    private static Visibility Shown(bool on) => on ? Visibility.Visible : Visibility.Collapsed;
}
