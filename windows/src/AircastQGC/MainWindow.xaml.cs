using System.Collections.Immutable;
using System.Linq;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC;

public sealed partial class MainWindow : Window
{
    private const double TileWidth = 260;
    private const double TileHeight = 150;
    private const double TileMargin = 16;

    private readonly FlightModeMenu _modes;
    private ImmutableStack<string> _history = ImmutableStack<string>.Empty;
    private string _page = NavPane.Fly;
    private ImmutableDictionary<string, UIElement> _pages = ImmutableDictionary<string, UIElement>.Empty;
    private bool _videoMain;

    public MainWindow()
    {
        InitializeComponent();
        BridgeWatch.BindDispatcher(DispatcherQueue);
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
        Map.MapTapped += (_, _, at) =>
        {
            if (_videoMain) SwapVideo(false);
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
        _ = new ResumeFailedPrompt(root);
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
        _videoMain = videoMain;
        VideoTile.Expanded = videoMain;
        Dock(VideoTile, videoMain);
        Dock(Map, !videoMain);
        Canvas.SetZIndex(VideoTile, videoMain ? -1 : 0);
        VideoStats.Expanded = videoMain;
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
