using System.Collections.Immutable;
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
    private bool _videoMain;

    public MainWindow()
    {
        InitializeComponent();
        BridgeWatch.BindDispatcher(DispatcherQueue);

        ExtendsContentIntoTitleBar = true;
        AppWindow.TitleBar.PreferredHeightOption = TitleBarHeightOption.Tall;
        SetTitleBar(Shell.DragRegion);
        Shell.Loaded += (_, _) => Shell.CaptionInset = AppWindow.TitleBar.RightInset / Shell.XamlRoot.RasterizationScale;

        _modes = new FlightModeMenu();
        Shell.FlightModeRequested += (_, anchor) => _modes.Show(anchor);
        _modes.StateChanged += (_, _) => Shell.PendingMode = _modes.Requested;

        Nav.NavigationRequested += (_, page) => Go(page);
        Nav.BackRequested += (_, _) => GoBack();

        NoVehicle.VehiclePresenceChanged += (_, present) => ShowVehicleOverlays(present);
        NoVehicle.OpenConnectionsRequested += (_, _) => Go(NavPane.Settings);

        MapButtons.ZoomInRequested += (_, _) => Map.ZoomIn();
        MapButtons.ZoomOutRequested += (_, _) => Map.ZoomOut();
        MapButtons.CenterOnVehicleRequested += (_, _) => Map.CenterOnVehicle();
        Map.MapRightTapped += (latitude, longitude, at) =>
            MapMenu.Show(latitude, longitude, Map.TransformToVisual(FlyContent).TransformPoint(at));

        GuidedStrip.ActionRequested += (_, offer) => GuidedCard.Show(offer);
        GuidedCard.Failed += (_, reason) => Warn(reason);

        VideoTile.SwapRequested += (_, _) => SwapVideo(!_videoMain);
        CameraPanel.CameraPresentChanged += (_, _) => CameraButton.Visibility = Shown(CameraPanel.CameraPresent);
        CameraButton.Click += (_, _) => ToggleCamera();

        Checklist.StateChanged += (_, _) => PaintChecklistButton();
        ChecklistButton.Click += (_, _) => ToggleChecklist();
    }

    private void Go(string page)
    {
        if (page == _page) return;
        _history = _history.Push(_page);
        ShowPage(page);
    }

    private void GoBack()
    {
        if (_history.IsEmpty) return;
        _history = _history.Pop(out var previous);
        ShowPage(previous);
    }

    private void ShowPage(string page)
    {
        _page = page;
        Nav.SelectedPage = page;
        Nav.CanGoBack = !_history.IsEmpty;
        FlyContent.Visibility = Shown(page == NavPane.Fly);
        PagePlaceholder.Visibility = Shown(page != NavPane.Fly);
        PageTitle.Text = page;
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
        if (GuidedCard.IsOpen) return;
        Notice.Message = reason;
        Notice.IsOpen = true;
    }

    private void SwapVideo(bool videoMain)
    {
        _videoMain = videoMain;
        VideoTile.Expanded = videoMain;
        Dock(VideoTile, videoMain);
        Dock(Map, !videoMain);
        Canvas.SetZIndex(VideoTile, videoMain ? -1 : 0);
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
        else CameraPanel.Show();
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
