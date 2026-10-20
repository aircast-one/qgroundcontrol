using System;
using System.Collections.Immutable;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Pages.Analyze;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Automation.Peers;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Controls;

public sealed class NoticeLayer : UserControl
{
    private const string FlyStatePath = "view.flyState";
    private const double FlyToastLift = 112;
    private const double PageToastLift = 24;

    private readonly string _noticeClient = $"notices.host.{Guid.NewGuid():N}";
    private readonly string _flyClient = $"notices.fly.{Guid.NewGuid():N}";
    private readonly string _batteryClient = $"notices.battery.{Guid.NewGuid():N}";
    private readonly InfoBar _alert = new()
    {
        Severity = InfoBarSeverity.Error,
        IsClosable = true,
        IsOpen = false,
        MaxWidth = 560,
        Margin = new Thickness(16),
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Top,
    };
    private readonly TextBlock _toastText = Ui.Text("");
    private readonly Border _toast;
    private readonly DispatcherTimer _toastTime = new() { Interval = TimeSpan.FromSeconds(4) };
    private readonly DispatcherTimer _dialogRetry = new() { Interval = TimeSpan.FromSeconds(1) };
    private readonly SpeechOut _speech;
    private readonly VoiceAlerts _voice;

    private long _through = -1;
    private ImmutableDictionary<string, DateTime> _shownAt = ImmutableDictionary<string, DateTime>.Empty;
    private ImmutableQueue<string> _toasts = ImmutableQueue<string>.Empty;
    private ImmutableQueue<string> _alerts = ImmutableQueue<string>.Empty;
    private ImmutableList<AppMessage> _dialogs = ImmutableList<AppMessage>.Empty;
    private bool _asking;
    private bool _onFly = true;
    private ArmTrack _arm = ArmTrack.Idle;

    public NoticeLayer()
    {
        _speech = new SpeechOut(DispatcherQueue);
        _voice = new VoiceAlerts(_speech);
        AutomationProperties.SetLiveSetting(_toastText, AutomationLiveSetting.Polite);
        _toast = new Border
        {
            Background = Ui.Token("CardBrush"),
            BorderBrush = Ui.Token("CardStrokeBrush"),
            BorderThickness = new Thickness(1),
            CornerRadius = (CornerRadius)Application.Current.Resources["RadiusOverlay"],
            Padding = new Thickness(16, 12, 16, 12),
            MaxWidth = 560,
            Margin = new Thickness(16, 16, 16, FlyToastLift),
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Bottom,
            Visibility = Visibility.Collapsed,
            Child = _toastText,
        }.Named("Notice");
        _alert.Closed += (_, args) =>
        {
            if (args.Reason == InfoBarCloseReason.CloseButton) AlertDismissed();
        };
        _toastTime.Tick += (_, _) =>
        {
            _toastTime.Stop();
            _toast.Visibility = Visibility.Collapsed;
            NextToast();
        };
        _dialogRetry.Tick += (_, _) =>
        {
            _dialogRetry.Stop();
            AskNext();
        };
        Content = new Grid { Children = { _alert, _toast } };
        Loaded += (_, _) => Start();
        Unloaded += (_, _) => Stop();
    }

    public event EventHandler<string>? NavigateRequested;

    public bool OnFly
    {
        get => _onFly;
        set
        {
            _onFly = value;
            _toast.Margin = new Thickness(16, 16, 16, value ? FlyToastLift : PageToastLift);
            _alert.Visibility = Ui.Shown(!value);
        }
    }

    private void Toast(string text)
    {
        _toasts = _toasts.Enqueue(text);
        NextToast();
    }

    private void Start()
    {
        BridgeWatch.Watch(_noticeClient, [NoticeBatch.Path(_through)], ApplyNotices);
        BridgeWatch.Watch(_flyClient, [FlyStatePath], ApplyFlyState);
        BridgeWatch.Watch(_batteryClient, [FlightSummary.BatteryView], _voice.Battery);
        Ui.Off(this, () => Bridge.Get(NoticeBatch.Path(_through)), ApplyNotices);
        _speech.Start();
        AskNext();
    }

    private void Stop()
    {
        BridgeWatch.Stop(_noticeClient);
        BridgeWatch.Stop(_flyClient);
        BridgeWatch.Stop(_batteryClient);
        _speech.Stop();
        _toastTime.Stop();
        _dialogRetry.Stop();
    }

    private void ApplyNotices(JsonObject view)
    {
        if (NoticeBatch.Parse(view) is not { } batch || batch.Through <= _through) return;
        _through = batch.Through;
        BridgeWatch.Watch(_noticeClient, [NoticeBatch.Path(_through)], ApplyNotices);
        Ui.Off(() => Bridge.Invoke(NoticeBatch.Acknowledge, new JsonArray(batch.Through)));
        if (batch.Destination is { } destination) NavigateRequested?.Invoke(this, NoticeBatch.PageOf(destination));
        var now = DateTime.UtcNow;
        var banners = NoticeBatch.Quiet(batch.Banners, _shownAt, now);
        _shownAt = _shownAt.SetItems(banners.Select(banner => new System.Collections.Generic.KeyValuePair<string, DateTime>(banner, now)));
        _dialogs = _dialogs.AddRange(batch.Dialogs).Distinct().ToImmutableList();
        if (!_onFly && NoticeBatch.Critical(banners.Where(batch.ErrorBanners.Contains).ToList()) is { } critical)
        {
            _alerts = _alerts.Enqueue(critical);
            ShowAlert();
        }
        _toasts = banners.Where(banner => !batch.ErrorBanners.Contains(banner)).Aggregate(_toasts, (queue, banner) => queue.Enqueue(banner));
        NextToast();
        AskNext();
    }

    private void ApplyFlyState(JsonObject view)
    {
        _voice.FlyState(view);
        if (MapUiJson.Text(view["class"]) != "FlyState") return;
        var armedNow = MapUiJson.Flag(view["connected"]) && MapUiJson.Flag(view["armed"]);
        var before = _arm;
        _arm = before.Next(armedNow, MapUiJson.Text(view["state"]));
        if (armedNow && !before.WasArmed)
            Ui.Off(this, FlightSummary.BatteryPercentNow, percent => _arm = _arm with { ArmedBattery = percent });
        if (FlightSummary.DisarmNotice(before.WasArmed, before.FlewWhileArmed, armedNow) is not { } said) return;
        Ui.Off(this, () => before.FlewWhileArmed ? FlightSummary.LandedNow(before.ArmedBattery) : "",
            summary => Toast(string.Join(" · ", new[] { said, summary }.Where(part => part.Length > 0))));
    }

    private void NextToast()
    {
        if (_toastTime.IsEnabled || _toasts.IsEmpty) return;
        _toasts = _toasts.Dequeue(out var text);
        _toastText.Text = text;
        _toast.Visibility = Visibility.Visible;
        _toastTime.Start();
    }

    private void ShowAlert()
    {
        _alert.IsOpen = !_alerts.IsEmpty;
        _alert.Message = _alerts.IsEmpty ? "" : _alerts.Peek();
    }

    private void AlertDismissed()
    {
        if (_alerts.IsEmpty) return;
        _alerts = _alerts.Dequeue();
        Ui.Off(() => Bridge.Invoke(NoticeBatch.ResetErrors));
        ShowAlert();
    }

    private async void AskNext()
    {
        if (_asking || _dialogs.IsEmpty || XamlRoot is not { } root) return;
        if (VisualTreeHelper.GetOpenPopupsForXamlRoot(root).Any(popup => popup.Child is ContentDialog))
        {
            _dialogRetry.Start();
            return;
        }
        var message = _dialogs[0];
        _asking = true;
        var answer = await Ask(root, message);
        _asking = false;
        if (answer is not bool confirmed)
        {
            _dialogRetry.Start();
            return;
        }
        _dialogs = _dialogs.Remove(message);
        if (confirmed) Act(message);
        AskNext();
    }

    private static async Task<bool?> Ask(XamlRoot root, AppMessage message)
    {
        try
        {
            return await new ContentDialog
            {
                XamlRoot = root,
                Title = message.Title,
                Content = new TextBlock { Text = message.Text, TextWrapping = TextWrapping.Wrap },
                PrimaryButtonText = message.Dismiss == null ? "" : message.Confirm,
                CloseButtonText = message.Dismiss ?? message.Confirm,
                DefaultButton = message.Dismiss == null || message.Action == AppMessage.RebootVehicle ? ContentDialogButton.Close : ContentDialogButton.Primary,
            }.ShowAsync() == ContentDialogResult.Primary;
        }
        catch (COMException)
        {
            return null;
        }
    }

    private void Act(AppMessage message)
    {
        if (message.Action == AppMessage.OpenSetup) NavigateRequested?.Invoke(this, NavPane.VehicleSetup);
        if (message.InvokePath is { } path) Ui.Off(() => Bridge.Invoke(path));
    }
}
