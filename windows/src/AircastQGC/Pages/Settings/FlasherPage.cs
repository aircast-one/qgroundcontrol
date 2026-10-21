using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Windows.Networking.Connectivity;
using Windows.System.Display;

namespace AircastQGC.Pages.Settings;

internal static class PcNetwork
{
    public static string? Ssid()
    {
        try
        {
            var profile = NetworkInformation.GetInternetConnectionProfile();
            return profile is { IsWlanConnectionProfile: true } && profile.WlanConnectionProfileDetails.GetConnectedSsid() is { Length: > 0 } ssid ? ssid : null;
        }
        catch (Exception) { return null; }
    }

    public static bool Metered()
    {
        try
        {
            var cost = NetworkInformation.GetInternetConnectionProfile()?.GetConnectionCost();
            return cost is not null && (cost.NetworkCostType is NetworkCostType.Fixed or NetworkCostType.Variable || cost.Roaming || cost.OverDataLimit);
        }
        catch (Exception) { return false; }
    }
}

internal sealed class FlasherPage : UserControl
{
    private const string Subject = "The card writer";
    private const string StepGroup = "flasherDisk";
    private static readonly TimeSpan Every = TimeSpan.FromMilliseconds(500);
    private static readonly TimeSpan DiskEvery = TimeSpan.FromSeconds(2);
    private static readonly bool Elevated = Elevation.Elevated();
#if DEBUG
    private const bool DebugBuild = true;
#else
    private const bool DebugBuild = false;
#endif

    private readonly StackPanel _steps = new() { Orientation = Orientation.Horizontal, Spacing = 4 };
    private readonly StackPanel _cardPanel = new() { Spacing = 8 };
    private readonly StackPanel _setupPanel = new() { Spacing = 8 };
    private readonly StackPanel _writePanel = new() { Spacing = 8 };
    private readonly StackPanel _diskList = new() { Spacing = 8 };
    private readonly Button _retry;
    private readonly TextBlock _unavailable = FormLook.Text("This device cannot write cards.", "FontSizeBody", "CautionBrush");
    private readonly TextBlock _loading = FormLook.Text("Loading…", "FontSizeBody", "TextSecondaryBrush");

    private readonly TextBox _hostname = FormLook.Field("", "Drone name");
    private readonly TextBlock _hostnameNote = FormLook.Caption("");
    private readonly TextBox _ssid = FormLook.Field("", "WiFi network");
    private readonly TextBlock _ssidNote = FormLook.Caption("");
    private readonly Button _useSsid;
    private readonly PasswordBox _wifiPassword = new() { Header = "WiFi password" };
    private readonly TextBlock _wifiPasswordNote = FormLook.Caption("");
    private readonly StackPanel _wifiFields = new() { Spacing = 4 };
    private readonly CheckBox _noWifi = new() { Content = "No WiFi" };
    private readonly PasswordBox _devicePassword = new() { Header = "Password for pi" };
    private readonly TextBlock _devicePasswordNote = FormLook.Caption("");
    private readonly StackPanel _devicePasswordFields = new() { Spacing = 4 };
    private readonly TextBlock _downloadLine = FormLook.Caption("");
    private readonly TextBlock _meteredLine = FormLook.Caption("", "CautionBrush");
    private readonly Button _downloadAnyway;
    private readonly HyperlinkButton _moreToggle = new() { Content = "More options" };
    private readonly StackPanel _morePanel = new() { Spacing = 8 };
    private readonly StackPanel _releaseList = new() { Spacing = 0 };
    private readonly TextBlock _releasesError = FormLook.Caption("", "CriticalBrush");
    private readonly StackPanel _channelRow = new() { Orientation = Orientation.Horizontal, Spacing = 8 };
    private readonly RadioButtons _sshModes = new() { MaxColumns = 3 };
    private readonly TextBox _authorizedKey = FormLook.Field("", "SSH public key");
    private readonly TextBlock _authorizedKeyNote = FormLook.Caption("");
    private readonly TextBlock _keyIdentity = FormLook.Caption("");
    private readonly StackPanel _keyFields = new() { Spacing = 4 };
    private readonly PasswordBox _authKey = new() { Header = "Tailscale auth key" };
    private readonly TextBox _controlServer = FormLook.Field("", "Headscale server", "https://");
    private readonly TextBlock _controlServerNote = FormLook.Caption("");
    private readonly StackPanel _controlServerFields = new() { Spacing = 4 };
    private readonly TextBox _country = FormLook.Field("", "Country code");

    private readonly StackPanel _progressPanel = new() { Spacing = 12 };
    private readonly TextBlock _phaseTitle = FormLook.Text("", "FontSizeSubtitle", "TextPrimaryBrush", true);
    private readonly ProgressBar _progress = new() { Maximum = 100 };
    private readonly TextBlock _progressLine = FormLook.Text("");
    private readonly StackPanel _outcomePanel = new() { Spacing = 8 };
    private readonly StackPanel _summaryPanel = new() { Spacing = 8 };

    private readonly TextBlock _note = FormLook.Text("", "FontSizeBody", "CautionBrush");
    private readonly InfoBar _admin = new() { Severity = InfoBarSeverity.Warning, IsClosable = false, IsOpen = false };
    private readonly Button _back;
    private readonly Button _continue;
    private readonly Button _cancel;
    private readonly Button _again;
    private readonly DisplayRequest _awake = new();

    private Task _queue = Task.CompletedTask;
    private FlasherState? _state;
    private FlasherStep _step = FlasherStep.Card;
    private ImmutableHashSet<string> _touched = ImmutableHashSet<string>.Empty;
    private bool _showAll;
    private bool _more;
    private bool _filled;
    private bool _opened;
    private bool _syncing;
    private bool _held;
    private bool _awakeHeld;
    private bool _metered;
    private string? _pcSsid;
    private string _refusal = "";
    private string? _autoOpened;
    private string _stepsShown = "";
    private string _disksShown = "";
    private string _releasesShown = "";
    private string _outcomeShown = "";
    private string _summaryShown = "";

    public FlasherPage()
    {
        _retry = FormLook.Button("Try the reader again", () => Choose(_state?.CardId ?? ""), ButtonLook.Subtle);
        _useSsid = FormLook.Button("", UsePcSsid, ButtonLook.Subtle, "");
        _downloadAnyway = FormLook.Button("Download now anyway", () => Command(Flasher.DownloadCommand), ButtonLook.Subtle);
        _back = FormLook.Button("Back", Back);
        _continue = FormLook.Button("Continue", Continue, ButtonLook.Accent);
        _cancel = FormLook.Button("Cancel", () => Command(Flasher.CancelCommand));
        _again = FormLook.Button("Flash another card", Again, ButtonLook.Accent);
        var restart = FormLook.Button("Restart as administrator", RestartElevated, ButtonLook.Accent, "");
        _admin.ActionButton = restart;
        _admin.Title = "Administrator rights needed";
        AutomationProperties.SetName(_admin, "Administrator rights needed");
        BuildCard();
        BuildSetup();
        BuildWrite();
        Content = Layout();
        Live.Changes(this, Every, () => Bridge.Get(Flasher.View), Apply);
        new Poller(this, DiskEvery, () => FormWrites.Off(() => FormWrites.Ok(Flasher.DisksCommand)));
        Loaded += (_, _) => Opened();
        Unloaded += (_, _) => Closed();
    }

    private UIElement Layout()
    {
        var body = new Grid
        {
            Padding = new Thickness(32, 8, 32, 24),
            ColumnDefinitions = { new ColumnDefinition { MaxWidth = SettingsBodies.DetailMaxWidth } },
            Children = { _loading, _unavailable, _cardPanel, _setupPanel, _writePanel },
        };
        var scroller = new ScrollViewer { VerticalScrollBarVisibility = ScrollBarVisibility.Auto, Content = body };
        var buttons = FormLook.Line(12, _back, _cancel, _again, _continue);
        buttons.HorizontalAlignment = HorizontalAlignment.Right;
        var footer = new Border
        {
            BorderBrush = FormLook.Brush("DividerBrush"),
            BorderThickness = new Thickness(0, 1, 0, 0),
            Padding = new Thickness(32, 12, 32, 16),
            Child = new Grid
            {
                ColumnDefinitions = { new ColumnDefinition { MaxWidth = SettingsBodies.DetailMaxWidth } },
                Children = { FormLook.Stack(8, _admin, _note, buttons) },
            },
        };
        var header = new Border
        {
            BorderBrush = FormLook.Brush("DividerBrush"),
            BorderThickness = new Thickness(0, 0, 0, 1),
            Padding = new Thickness(24, 0, 32, 8),
            Child = _steps,
        };
        Grid.SetRow(scroller, 1);
        Grid.SetRow(footer, 2);
        return new Grid
        {
            RowDefinitions = { new RowDefinition { Height = GridLength.Auto }, new RowDefinition(), new RowDefinition { Height = GridLength.Auto } },
            Children = { header, scroller, footer },
        };
    }

    private static UIElement Heading(string title, string body) => FormLook.Margined(FormLook.Stack(4,
        FormLook.Text(title, "FontSizeSubtitle", "TextPrimaryBrush", true),
        FormLook.Text(body, "FontSizeBody", "TextSecondaryBrush")), new Thickness(0, 8, 0, 8));

    private static TextBlock SubHeading(string text) =>
        FormLook.Margined(FormLook.Text(text, "FontSizeBody", "AccentTextBrush", true), new Thickness(0, 16, 0, 0));

    private void BuildCard() => FormLook.Fill(_cardPanel, [
        Heading("Insert the card", "Put the drone's SD card in a card reader and plug the reader into this computer."),
        _diskList,
        _retry,
    ]);

    private void BuildSetup()
    {
        _hostname.IsSpellCheckEnabled = false;
        _ssid.IsSpellCheckEnabled = false;
        _authorizedKey.IsSpellCheckEnabled = false;
        _authorizedKey.AcceptsReturn = true;
        _authorizedKey.TextWrapping = TextWrapping.Wrap;
        _authorizedKey.MinHeight = 72;
        _controlServer.IsSpellCheckEnabled = false;
        _keyIdentity.FontFamily = new Microsoft.UI.Xaml.Media.FontFamily("Cascadia Mono, Consolas");
        _keyIdentity.IsTextSelectionEnabled = true;
        _country.MaxLength = 2;
        _country.CharacterCasing = CharacterCasing.Upper;
        _country.Width = FormLook.NumberWidth;
        _country.HorizontalAlignment = HorizontalAlignment.Left;
        Flasher.SshModes.ToList().ForEach(mode => _sshModes.Items.Add(mode.Label));
        AutomationProperties.SetName(_sshModes, "Signing in to the drone");
        AutomationProperties.SetName(_moreToggle, "More options");
        new[] { _wifiPassword, _devicePassword, _authKey }.ToList().ForEach(box => box.HorizontalAlignment = HorizontalAlignment.Stretch);

        Typed(_hostname, "hostname");
        Typed(_ssid, "ssid");
        Typed(_authorizedKey, "authorizedKey");
        Typed(_controlServer, "controlServer");
        Typed(_country, "country");
        Hidden(_wifiPassword, "wifiPassword");
        Hidden(_devicePassword, "devicePassword");
        Hidden(_authKey, "authKey");
        FormLook.Toggled(_noWifi, on =>
        {
            if (_syncing) return;
            Edit("noWifi", on);
        });
        _sshModes.SelectionChanged += (_, _) =>
        {
            if (_syncing || _sshModes.SelectedIndex < 0) return;
            Edit("sshMode", Flasher.SshModes[_sshModes.SelectedIndex].Mode);
        };
        _moreToggle.Click += (_, _) =>
        {
            _more = !_more;
            Render();
        };

        FormLook.Fill(_wifiFields, [FormLook.Stack(4, _ssid, _ssidNote), _useSsid, FormLook.Stack(4, _wifiPassword, _wifiPasswordNote)]);
        FormLook.Fill(_devicePasswordFields, [_devicePassword, _devicePasswordNote]);
        FormLook.Fill(_keyFields, [_authorizedKey, _authorizedKeyNote, _keyIdentity]);
        FormLook.Fill(_controlServerFields, [_controlServer, _controlServerNote]);
        FormLook.Fill(_morePanel, [
            SubHeading("Aircast OS version"),
            FormLook.Card(_releaseList),
            _releasesError,
            _channelRow,
            SubHeading("Signing in to the drone"),
            _sshModes,
            _keyFields,
            SubHeading("Remote access"),
            FormLook.Stack(4, _authKey, FormLook.Caption("The drone joins your tailnet on first boot")),
            _controlServerFields,
            SubHeading("WiFi region"),
            FormLook.Stack(4, _country, FormLook.Caption("Sets which WiFi channels the drone may use")),
        ]);
        FormLook.Fill(_setupPanel, [
            Heading("Set up the drone", "These are applied on the drone's first boot, so it joins your network by itself."),
            FormLook.Stack(4, _hostname, _hostnameNote),
            _wifiFields,
            FormLook.Stack(2, _noWifi, FormLook.Caption("The drone goes online over Ethernet or cellular")),
            _devicePasswordFields,
            FormLook.Stack(4, _meteredLine, _downloadAnyway, _downloadLine),
            _moreToggle,
            _morePanel,
        ]);
    }

    private void BuildWrite()
    {
        AutomationProperties.SetName(_progress, "Writing progress");
        FormLook.Fill(_progressPanel, [
            _phaseTitle,
            _progress,
            _progressLine,
            FormLook.Text("Keep the card reader plugged in until this finishes.", "FontSizeBody", "TextSecondaryBrush"),
        ]);
        FormLook.Fill(_writePanel, [FormLook.Margined(_progressPanel, new Thickness(0, 8, 0, 0)), _outcomePanel, _summaryPanel]);
    }

    private void Typed(TextBox box, string key) => box.TextChanged += (_, _) =>
    {
        if (_syncing) return;
        Edit(key, box.Text);
    };

    private void Hidden(PasswordBox box, string key)
    {
        AutomationProperties.SetName(box, box.Header as string ?? key);
        box.PasswordChanged += (_, _) =>
        {
            if (_syncing) return;
            Edit(key, box.Password);
        };
    }

    private async void Opened()
    {
        NavigationLock.Refused += OnRefused;
        var (ssid, metered) = await FormWrites.Off(() => (PcNetwork.Ssid(), PcNetwork.Metered()));
        _pcSsid = ssid;
        _metered = metered;
        _refusal = await Queue(() => FormWrites.Call(Flasher.OpenCommand, Subject)) ?? "";
        _opened = true;
        Apply(await FormWrites.Off(() => Bridge.Get(Flasher.View)));
    }

    private void Closed()
    {
        NavigationLock.Refused -= OnRefused;
        NavigationLock.Release(Flasher.BusyHold);
        _held = false;
        if (_awakeHeld) _awake.RequestRelease();
        _awakeHeld = false;
    }

    private void OnRefused(object? sender, string reason)
    {
        if (reason != Flasher.BusyHold) return;
        _refusal = reason;
        Render();
    }

    private Task<string?> Queue(Func<string?> work)
    {
        var next = _queue.ContinueWith(_ => work(), TaskScheduler.Default);
        _queue = next;
        return next;
    }

    private async void Command(string path, params object?[] args)
    {
        _refusal = await Queue(() => FormWrites.Call(path, Subject, args)) ?? "";
        Render();
    }

    private void Edit(string key, object value)
    {
        _touched = _touched.Add(key);
        _ = Queue(() => FormWrites.Call(Flasher.FormCommand, Subject, new JsonObject { [key] = FormJson.Node(value) }));
        Render();
    }

    private void Choose(string id)
    {
        if (id.Length == 0) return;
        Command(Flasher.ChooseDiskCommand, id);
    }

    private void UsePcSsid()
    {
        if (_pcSsid is not { } ssid) return;
        Synced(() => _ssid.Text = ssid);
        Edit("ssid", ssid);
    }

    private void Synced(Action change)
    {
        _syncing = true;
        try { change(); }
        finally { _syncing = false; }
    }

    private void Fill(FlasherForm form) => Synced(() =>
    {
        _hostname.Text = form.Hostname;
        _ssid.Text = form.Ssid;
        _wifiPassword.Password = form.WifiPassword;
        _noWifi.IsChecked = form.NoWifi;
        _devicePassword.Password = form.DevicePassword;
        _authorizedKey.Text = form.AuthorizedKey;
        _authKey.Password = form.AuthKey;
        _controlServer.Text = form.ControlServer;
        _country.Text = form.Country;
        _sshModes.SelectedIndex = Math.Max(0, Flasher.SshModes.ToList().FindIndex(mode => mode.Mode == form.SshMode));
    });

    private void FirstFill(FlasherForm read)
    {
        var country = read.Country.Trim().Length > 0 ? read.Country : RegionInfo.CurrentRegion.TwoLetterISORegionName;
        var sshMode = Flasher.DefaultSshMode(read) ?? read.SshMode;
        var ssid = read.Ssid.Trim().Length == 0 && !read.NoWifi && _pcSsid is { } pc ? pc : read.Ssid;
        var changes = new JsonObject
        {
            ["country"] = country != read.Country ? country : null,
            ["sshMode"] = sshMode != read.SshMode ? sshMode : null,
            ["ssid"] = ssid != read.Ssid ? ssid : null,
        };
        var patch = new JsonObject(changes.Where(c => c.Value is not null).Select(c => KeyValuePair.Create(c.Key, c.Value?.DeepClone())));
        if (patch.Count > 0) _ = Queue(() => FormWrites.Call(Flasher.FormCommand, Subject, patch));
        Fill(read with { Country = country, SshMode = sshMode, Ssid = ssid });
        _filled = true;
    }

    private void Apply(JsonObject view)
    {
        if (!_opened) return;
        var state = Flasher.State(view);
        _state = state;
        if (state is null)
        {
            Render();
            return;
        }
        if (Flasher.StepFor(state) is { } forced) _step = forced;
        if (!_filled) FirstFill(state.Form);
        _autoOpened = state.Disks.Count == 0 ? null : _autoOpened;
        if (Flasher.ReaderToOpen(state) is { } reader && reader != _autoOpened)
        {
            _autoOpened = reader;
            Choose(reader);
        }
        Render();
    }

    private void Render()
    {
        var state = _state;
        _loading.Visibility = FormLook.Shown(state is null);
        _unavailable.Visibility = FormLook.Shown(state is { Available: false });
        var shown = state is { Available: true };
        _cardPanel.Visibility = FormLook.Shown(shown && _step == FlasherStep.Card);
        _setupPanel.Visibility = FormLook.Shown(shown && _step == FlasherStep.Setup);
        _writePanel.Visibility = FormLook.Shown(shown && _step == FlasherStep.Write);
        RenderSteps(state);
        if (state is not { Available: true })
        {
            RenderFooter(null);
            return;
        }
        RenderCard(state);
        RenderSetup(state);
        RenderWrite(state);
        RenderFooter(state);
        Hold(state);
    }

    private void Hold(FlasherState state)
    {
        if (state.Busy != _held)
        {
            if (state.Busy) NavigationLock.Hold(Flasher.BusyHold);
            else NavigationLock.Release(Flasher.BusyHold);
            _held = state.Busy;
        }
        var awake = Flasher.KeepsAwake(state);
        if (awake == _awakeHeld) return;
        if (awake) _awake.RequestActive();
        else _awake.RequestRelease();
        _awakeHeld = awake;
    }

    private FlasherStep? BackStep() => Flasher.PreviousStep(_step, _state?.Busy == true, _state?.Finished == true);

    private void RenderSteps(FlasherState? state)
    {
        var canBack = BackStep() is not null;
        var signature = $"{_step}|{canBack}";
        if (signature == _stepsShown) return;
        _stepsShown = signature;
        FormLook.Fill(_steps, Flasher.StepTitles.Select((title, at) => (UIElement)StepButton((FlasherStep)at, title, canBack)));
    }

    private Button StepButton(FlasherStep step, string title, bool canBack)
    {
        var done = step < _step;
        var current = step == _step;
        var mark = done
            ? (UIElement)FormLook.Glyph("", 14, "AccentTextBrush")
            : FormLook.Text($"{(int)step + 1}", "FontSizeBody", current ? "AccentTextBrush" : "TextTertiaryBrush", current);
        var label = FormLook.Text(title, "FontSizeBody", current || done ? "AccentTextBrush" : "TextTertiaryBrush", current);
        var button = new Button
        {
            Content = FormLook.Line(8, mark, label),
            Padding = new Thickness(12, 8, 12, 8),
            MinHeight = FormLook.Size("SizeControl"),
            CornerRadius = FormLook.Radius("RadiusControl"),
            IsEnabled = done && canBack,
        };
        FormLook.Paint(button, ButtonLook.Subtle);
        AutomationProperties.SetName(button, $"Step {(int)step + 1} {title}{(done ? ", done" : current ? ", current" : "")}");
        button.Click += (_, _) =>
        {
            _refusal = "";
            _step = step;
            Render();
        };
        return button;
    }

    private void RenderCard(FlasherState state)
    {
        _retry.Visibility = FormLook.Shown(state.CardState == "failed" && state.CardId.Length > 0 && !state.NeedsAdmin);
        var signature = string.Join("|", state.Disks.Select(d => $"{d.Id}={d.Name}").Append($"{state.CardState}:{state.CardId}:{state.CardText}"));
        if (signature == _disksShown) return;
        _disksShown = signature;
        FormLook.Fill(_diskList, [state.Disks.Count == 0
            ? FormLook.Card(FormLook.Row(
                FormLook.Stack(2, FormLook.Text("Waiting for a card reader…"), FormLook.Caption("You can continue and plug it in before writing")),
                new ProgressRing { IsActive = true, Width = 16, Height = 16 }))
            : FormLook.Card(state.Disks.Select(disk => DiskRow(state, disk)))]);
    }

    private UIElement DiskRow(FlasherState state, FlasherDisk disk)
    {
        var chosen = disk.Id == state.CardId;
        var detail = !chosen ? "" : state.CardState == "ready" ? "Ready to write" : state.CardText;
        var radio = new RadioButton
        {
            GroupName = StepGroup,
            IsChecked = chosen,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            Content = FormLook.Stack(2,
                FormLook.Text(disk.Name),
                FormLook.Caption(detail, state.CardState == "failed" ? "CriticalBrush" : "TextSecondaryBrush")),
        };
        AutomationProperties.SetName(radio, disk.Name);
        radio.Checked += (_, _) => Choose(disk.Id);
        var ready = chosen && state.CardState == "ready";
        var busy = chosen && state.CardState is "opening" or "waiting";
        return FormLook.Row(radio,
            ready ? FormLook.Glyph("", 16, "SuccessBrush")
            : busy ? new ProgressRing { IsActive = true, Width = 16, Height = 16 }
            : (FrameworkElement?)null);
    }

    private void RenderSetup(FlasherState state)
    {
        var problems = Flasher.VisibleProblems(state.Problems, _touched, _showAll);
        var noWifi = _noWifi.IsChecked == true;
        var mode = _sshModes.SelectedIndex >= 0 ? Flasher.SshModes[_sshModes.SelectedIndex].Mode : state.Form.SshMode;
        var hostname = _hostname.Text.Trim();
        Note(_hostnameNote, problems.GetValueOrDefault("hostname"), hostname.Length > 0 ? $"It answers at {hostname}.local" : null);
        Note(_ssidNote, problems.GetValueOrDefault("ssid"), _pcSsid is { } pc && _ssid.Text == pc ? "This computer's current network" : null);
        Note(_wifiPasswordNote, problems.GetValueOrDefault("wifiPassword"), null);
        Note(_devicePasswordNote, problems.GetValueOrDefault("devicePassword"), "At least 8 characters. You sign in to the drone as pi with it.");
        Note(_authorizedKeyNote, problems.GetValueOrDefault("authorizedKey"), null);
        Note(_controlServerNote, problems.GetValueOrDefault("controlServer"), "Leave empty for Tailscale");
        FormLook.Say(_keyIdentity, state.KeyIdentity);
        _wifiFields.Visibility = FormLook.Shown(!noWifi);
        var offer = _pcSsid is { } other && other != _ssid.Text ? other : null;
        _useSsid.Visibility = FormLook.Shown(offer is not null);
        if (offer is not null && _useSsid.Content is StackPanel { Children: [_, TextBlock words] }) words.Text = $"Use {offer}";
        AutomationProperties.SetName(_useSsid, offer is null ? "Use this computer's network" : $"Use {offer}");
        _devicePasswordFields.Visibility = FormLook.Shown(mode == Flasher.PasswordMode);
        _keyFields.Visibility = FormLook.Shown(mode == Flasher.KeyOnlyMode);
        _controlServerFields.Visibility = FormLook.Shown(_authKey.Password.Trim().Length > 0);
        var metered = Flasher.MeteredNote(state.Release, state.DownloadState, _metered);
        FormLook.Say(_meteredLine, metered);
        _downloadAnyway.Visibility = FormLook.Shown(metered is not null);
        FormLook.Say(_downloadLine, metered is null ? state.DownloadText : null);
        _moreToggle.Content = _more ? "Fewer options" : "More options";
        AutomationProperties.SetName(_moreToggle, _more ? "Fewer options" : "More options");
        _morePanel.Visibility = FormLook.Shown(_more);
        FormLook.Say(_releasesError, state.ReleasesState == "failed" ? state.ReleasesError : null);
        RenderReleases(state);
    }

    private static void Note(TextBlock note, string? problem, string? help)
    {
        FormLook.Say(note, problem ?? help);
        note.Foreground = FormLook.Brush(problem is null ? "TextSecondaryBrush" : "CriticalBrush");
    }

    private void RenderReleases(FlasherState state)
    {
        var channels = Flasher.ShowsChannels(DebugBuild, state.Channel);
        var signature = string.Join("|", state.Releases.Select(r => $"{r.Version}:{r.Recommended}").Append($"{state.Selected}:{state.ReleasesState}:{state.Channel}:{channels}"));
        if (signature == _releasesShown) return;
        _releasesShown = signature;
        FormLook.Fill(_releaseList, state.Releases.Count == 0
            ? [FormLook.Row(FormLook.Text(state.ReleasesState == "loading" ? "Loading Aircast OS versions…" : "No Aircast OS versions are listed."), null)]
            : state.Releases.Select((release, at) => ReleaseRow(state, release, at)));
        _channelRow.Visibility = FormLook.Shown(channels);
        FormLook.Fill(_channelRow, state.Channels.Select(channel => (UIElement)ChannelChip(state, channel)));
    }

    private UIElement ReleaseRow(FlasherState state, FlasherRelease release, int at)
    {
        var radio = new RadioButton
        {
            GroupName = "flasherRelease",
            IsChecked = release.Version == state.Selected,
            Content = FormLook.Stack(2, FormLook.Text(release.Label), FormLook.Caption(Flasher.ReleaseLine(release))),
        };
        AutomationProperties.SetName(radio, release.Label);
        radio.Checked += (_, _) => Command(Flasher.SelectCommand, release.Version);
        return new Border
        {
            BorderBrush = FormLook.Brush("DividerBrush"),
            BorderThickness = new Thickness(0, at == 0 ? 0 : 1, 0, 0),
            Padding = new Thickness(16, 8, 16, 8),
            Child = radio,
        };
    }

    private Button ChannelChip(FlasherState state, string channel)
    {
        var shown = CultureInfo.InvariantCulture.TextInfo.ToTitleCase(channel);
        var chip = FormLook.Button(shown, () => Command(Flasher.ChannelCommand, channel), channel == state.Channel ? ButtonLook.Accent : ButtonLook.Standard);
        AutomationProperties.SetName(chip, $"{shown} channel");
        return chip;
    }

    private void RenderWrite(FlasherState state)
    {
        _progressPanel.Visibility = FormLook.Shown(state.Busy);
        _outcomePanel.Visibility = FormLook.Shown(!state.Busy && state.Finished);
        _summaryPanel.Visibility = FormLook.Shown(!state.Busy && !state.Finished);
        FormLook.Say(_phaseTitle, Flasher.PhaseLabel(state.Phase));
        _progress.Value = state.Percent;
        _progress.IsIndeterminate = state.Busy && state.Percent <= 0;
        FormLook.Say(_progressLine, string.Join(" · ", new[] { $"{(int)state.Percent}%", state.SpeedText, state.RemainingText }.Where(t => t.Length > 0)));
        RenderOutcome(state);
        RenderSummary(state);
    }

    private void RenderOutcome(FlasherState state)
    {
        var signature = $"{state.Phase}|{state.JobError}|{state.Hostname}|{_ssid.Text}|{_noWifi.IsChecked}";
        if (signature == _outcomeShown) return;
        _outcomeShown = signature;
        FormLook.Fill(_outcomePanel, state.Phase switch
        {
            "done" =>
            [
                FormLook.Line(12, FormLook.Badge("", true), Heading("Card ready", "The card is written and checked. It's safe to unplug the reader.")),
                SubHeading("What happens next"),
                .. Flasher.NextSteps(_ssid.Text.Trim(), state.Hostname, _noWifi.IsChecked == true)
                    .Select((line, at) => (UIElement)FormLook.Text($"{at + 1}. {line}")),
            ],
            "failed" =>
            [
                Heading("Writing failed", "The card is not usable as it is. Fix the problem below and write it again."),
                new InfoBar { Severity = InfoBarSeverity.Error, IsOpen = true, IsClosable = false, Title = "What went wrong", Message = state.JobError },
            ],
            "cancelled" => [Heading("Cancelled", "The card is incomplete. Write it again before putting it in the drone.")],
            _ => [],
        });
    }

    private void RenderSummary(FlasherState state)
    {
        var metered = Flasher.MeteredNote(state.Release, state.DownloadState, _metered);
        var signature = string.Join("|", state.Summary.Select(r => $"{r.Label}={r.Value}:{r.Warn}")
            .Append($"{state.CardState}:{state.CardText}:{state.DownloadState}:{state.DownloadText}:{metered}"));
        if (signature == _summaryShown) return;
        _summaryShown = signature;
        var rows = state.Summary
            .Select(row => SummaryRow(row.Label, row.Value, row.Warn))
            .Append(SummaryRow("Card", state.CardText.Length > 0 ? state.CardText : "Not connected", state.CardState != "ready"));
        FormLook.Fill(_summaryPanel, [
            Heading("Check and write", "Everything on the card is replaced."),
            FormLook.Card(rows),
            state.DownloadState != "ready" && state.DownloadText.Length > 0 ? FormLook.Caption(state.DownloadText) : null,
            metered is null ? null : FormLook.Caption($"{metered} It downloads when you write the card.", "CautionBrush"),
        ]);
    }

    private UIElement SummaryRow(string label, string value, bool warn)
    {
        var change = FormLook.Button("Change", () => ChangeRow(label), ButtonLook.Subtle);
        AutomationProperties.SetName(change, $"Change {label}");
        var line = new Grid
        {
            ColumnSpacing = 16,
            ColumnDefinitions = { new ColumnDefinition { Width = new GridLength(160) }, new ColumnDefinition() },
        };
        var shown = FormLook.Text(value, "FontSizeBody", warn ? "CautionBrush" : "TextPrimaryBrush");
        Grid.SetColumn(shown, 1);
        line.Children.Add(FormLook.Text(label, "FontSizeBody", "TextSecondaryBrush"));
        line.Children.Add(shown);
        return FormLook.Row(line, change);
    }

    private void ChangeRow(string label)
    {
        _refusal = "";
        _more = _more || Flasher.MoreRows.Contains(label);
        _step = label == "Card" ? FlasherStep.Card : FlasherStep.Setup;
        Render();
    }

    private void RenderFooter(FlasherState? state)
    {
        var busy = state?.Busy == true;
        var finished = state?.Finished == true;
        var admin = state is { NeedsAdmin: true } && !busy;
        _admin.IsOpen = admin;
        _admin.Message = Elevated
            ? "Windows refused to let Aircast write to the card even as administrator. A policy on this computer may block writing to removable drives."
            : "Windows lets only programs running as administrator write to a card. Everything else in Aircast keeps working as it is.";
        if (_admin.ActionButton is Button restart) restart.Visibility = FormLook.Shown(!Elevated);
        var note = _refusal.Length > 0 ? _refusal
            : state is null ? ""
            : _step == FlasherStep.Setup && _showAll ? Flasher.ProblemNote(state)
            : _step == FlasherStep.Write && !busy && !finished && !(admin && state.Blocked == Flasher.NeedsAdmin) ? state.Blocked
            : "";
        FormLook.Say(_note, note);
        _cancel.Visibility = FormLook.Shown(busy);
        _again.Visibility = FormLook.Shown(!busy && finished);
        if (_again.Content is TextBlock againText) againText.Text = Flasher.Again(state?.Phase ?? "");
        AutomationProperties.SetName(_again, Flasher.Again(state?.Phase ?? ""));
        _back.Visibility = FormLook.Shown(state is not null && !busy && !finished && _step != FlasherStep.Card);
        _continue.Visibility = FormLook.Shown(state is { Available: true } && !busy && !finished);
        var writing = _step == FlasherStep.Write;
        if (_continue.Content is TextBlock continueText) continueText.Text = writing ? "Write card" : "Continue";
        AutomationProperties.SetName(_continue, writing ? "Write card" : "Continue");
        _continue.IsEnabled = !writing || state?.CanStart == true;
    }

    private void Back()
    {
        _refusal = "";
        _step = BackStep() ?? _step;
        Render();
    }

    private async void Continue()
    {
        if (_state is not { } state) return;
        _refusal = "";
        switch (_step)
        {
            case FlasherStep.Card:
                _step = FlasherStep.Setup;
                if (!_metered && state.DownloadState is "idle" or "failed" or "") Command(Flasher.DownloadCommand);
                break;
            case FlasherStep.Setup:
                _showAll = true;
                _more = _more || Flasher.ProblemInMoreOptions(state);
                _step = Flasher.SetupComplete(state.Problems) ? FlasherStep.Write : _step;
                break;
            case FlasherStep.Write:
                if (await Confirmed(state)) Command(Flasher.StartCommand);
                break;
        }
        Render();
    }

    private async Task<bool> Confirmed(FlasherState state) => await FormLook.Confirm(XamlRoot, "Write the card?",
        $"Everything on {(state.CardText.Length > 0 ? state.CardText : "the card")} is erased and replaced with {state.Release?.Label ?? "Aircast OS"}. This cannot be undone.",
        "Erase and write");

    private async void Again()
    {
        var finished = _state?.Phase ?? "";
        var read = await Queue(() => FormWrites.Call(Flasher.AgainCommand, Subject)).ContinueWith(_ => Flasher.State(Bridge.Get(Flasher.View)), TaskScheduler.Default);
        if (read is not null) Fill(read.Form);
        _touched = ImmutableHashSet<string>.Empty;
        _showAll = false;
        _refusal = "";
        _step = finished == "done" ? FlasherStep.Card : FlasherStep.Write;
        _state = read ?? _state;
        Render();
    }

    private async void RestartElevated()
    {
        if (!await FormLook.Confirm(XamlRoot, "Restart as administrator",
                "Aircast closes and opens again with administrator rights so it can write the card. Vehicle links close and connect again after the restart.",
                "Restart")) return;
        _refusal = Elevation.Restart() ?? "";
        Render();
    }
}
