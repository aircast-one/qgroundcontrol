using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Setup;

public sealed partial class SetupPage : UserControl
{
    private static readonly TimeSpan SummaryEvery = TimeSpan.FromSeconds(2);
    private static readonly TimeSpan SearchSettle = TimeSpan.FromMilliseconds(250);
    private static readonly TimeSpan NoticeFor = TimeSpan.FromSeconds(3);
    private const double TwoPaneMinWidth = 840;
    private const double ListWidth = 360;
    private const string OverviewGlyph = "\uE90F";
    private static readonly IReadOnlyDictionary<string, IReadOnlyList<SummaryLine>> NoSummaries = new Dictionary<string, IReadOnlyList<SummaryLine>>();
    private static readonly IReadOnlyDictionary<string, IReadOnlyList<string>> NoHits = new Dictionary<string, IReadOnlyList<string>>();

    private readonly DispatcherTimer _settle = new() { Interval = SearchSettle };
    private readonly DispatcherTimer _noticeTimer = new() { Interval = NoticeFor };
    private JsonObject _setup = new();
    private JsonObject _firmware = new();
    private bool _advanced = true;
    private bool _wide = true;
    private bool _picked;
    private bool _overview;
    private (string Name, string? Section)? _opened;
    private bool _parametersOpen;
    private string _parametersSearch = "";
    private (string Page, string Search)? _requested;
    private IReadOnlyDictionary<string, IReadOnlyList<SummaryLine>> _summaries = NoSummaries;
    private string _summarySignature = "";
    private IReadOnlyDictionary<string, IReadOnlyList<string>> _sectionHits = NoHits;
    private int _hitsVersion;
    private int _parameterCount;
    private string? _countedFor;
    private bool _summaryOpened;
    private string? _detailKey;
    private string? _stateKey;

    public event EventHandler? OpenConnectionsRequested;

    public SetupPage()
    {
        InitializeComponent();
        AutomationProperties.SetName(SearchBox, "Search setup");
        FormLook.Paint(BackButton, ButtonLook.Subtle);
        BackButton.Click += (_, _) => Back();
        SizeChanged += (_, size) =>
        {
            var wide = size.NewSize.Width >= TwoPaneMinWidth;
            if (wide == _wide) return;
            _wide = wide;
            Arrange();
        };
        SearchBox.TextChanged += (_, _) =>
        {
            _settle.Stop();
            _settle.Start();
        };
        _settle.Tick += (_, _) =>
        {
            _settle.Stop();
            RefreshSectionHits();
        };
        _noticeTimer.Tick += (_, _) =>
        {
            _noticeTimer.Stop();
            NavigationNotice.IsOpen = false;
        };
        SearchBox.KeyDown += (_, e) =>
        {
            if (e.Key == Windows.System.VirtualKey.Enter && _advanced && SearchBox.Text.Trim().Length > 0) OpenParameters(SearchBox.Text.Trim());
        };
        Live.Watch(this, SetupModel.SetupPath, view =>
        {
            _setup = view;
            Render();
        });
        Live.Watch(this, SetupModel.FirmwarePath, view =>
        {
            _firmware = view;
            RenderHeader();
        });
        Live.Watch(this, SetupModel.AdvancedUiPath, view =>
        {
            _advanced = MapUiJson.Flag(view["shown"], true);
            RenderList();
        });
        _ = new Poller(this, SummaryEvery, PollSummary);
        Loaded += (_, _) => NavigationLock.Refused += OnRefused;
        Unloaded += (_, _) =>
        {
            NavigationLock.Refused -= OnRefused;
            _summaryOpened = false;
        };
    }

    public void Open(string page, string parametersSearch = "")
    {
        _requested = (page, parametersSearch);
        if (IsLoaded) Render();
    }

    private void OnRefused(object? sender, string reason)
    {
        NavigationNotice.Message = reason;
        NavigationNotice.IsOpen = true;
        _noticeTimer.Stop();
        _noticeTimer.Start();
    }

    private IReadOnlyList<SetupComponent> Components => SetupModel.Components(_setup);

    private SetupPageInfo? PageOf(SetupComponent component) => SetupModel.Page(_setup, component);

    private void Render()
    {
        if (!SetupModel.Connected(_setup))
        {
            _opened = null;
            _parametersOpen = false;
            _overview = false;
            _picked = false;
            _summaries = NoSummaries;
            _summarySignature = "";
            _summaryOpened = false;
            _parameterCount = 0;
            _countedFor = null;
            ShowState("none", NoVehicle);
            return;
        }
        if (SetupModel.Wait(_setup) is { } wait)
        {
            ShowState($"wait|{wait}", () => Waiting(wait));
            return;
        }
        _stateKey = null;
        StateScroller.Visibility = Visibility.Collapsed;
        TwoPane.Visibility = Visibility.Visible;
        ApplyRequest();
        CountParameters();
        RenderHeader();
        EnsureOpened();
        RenderList();
        RenderDetail();
        Arrange();
    }

    private void Arrange()
    {
        var list = _wide || !_picked;
        var detail = _wide || _picked;
        ListPane.Visibility = FormLook.Shown(list);
        DetailPane.Visibility = FormLook.Shown(detail);
        ListColumn.Width = _wide ? new GridLength(ListWidth) : list ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
        DetailColumn.Width = detail ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
        BackButton.Visibility = FormLook.Shown(!_wide);
    }

    private void Back()
    {
        if (!NavigationLock.Allows()) return;
        _picked = false;
        Arrange();
    }

    private void ShowState(string key, Func<UIElement> build)
    {
        if (key == _stateKey) return;
        _stateKey = key;
        _detailKey = null;
        DetailHost.Content = null;
        TwoPane.Visibility = Visibility.Collapsed;
        StateHost.Content = build();
        StateScroller.Visibility = Visibility.Visible;
    }

    private UIElement NoVehicle()
    {
        var connect = FormLook.Button("Set up connection", () => OpenConnectionsRequested?.Invoke(this, EventArgs.Empty), ButtonLook.Accent);
        connect.HorizontalAlignment = HorizontalAlignment.Center;
        return FormLook.Stack(0, FormLook.Empty("", SetupModel.LookingTitle, SetupModel.LookingHint), connect);
    }

    private UIElement Waiting(ParameterWait wait)
    {
        var body = FormLook.Margined(FormLook.Text(wait.Body, "FontSizeBody", "TextSecondaryBrush"), new Thickness(24, 0, 24, 16));
        body.TextAlignment = TextAlignment.Center;
        var download = wait.DownloadOffered
            ? FormLook.Button("Download parameters", () => SetupUi.Detached(() => FormWrites.Ok(SetupModel.ParameterRefresh)), ButtonLook.Accent)
            : null;
        if (download is not null) download.HorizontalAlignment = HorizontalAlignment.Center;
        return FormLook.Margined(FormLook.Stack(0, SetupUi.Notice(wait.Title), body, download), new Thickness(0, 48, 0, 0));
    }

    private void ApplyRequest()
    {
        if (_requested is not { } request || Components.Count == 0) return;
        _requested = null;
        if (request.Page.Length == 0)
        {
            _parametersOpen = false;
            _opened = null;
            _overview = true;
            _picked = false;
            return;
        }
        if (request.Page == SetupModel.ParametersPage)
        {
            _parametersSearch = request.Search;
            _opened = null;
            _parametersOpen = true;
            _picked = true;
            return;
        }
        if (Components.FirstOrDefault(c => c.Name == request.Page || c.Page == request.Page) is { } component)
        {
            _parametersOpen = false;
            _opened = (component.Name, null);
            _picked = true;
        }
    }

    private void CountParameters()
    {
        var vehicle = MapUiJson.Number(_setup["vehicleId"])?.ToString() ?? "";
        if (!SetupModel.ParametersReady(_setup) || _countedFor == vehicle) return;
        _countedFor = vehicle;
        SetupUi.Run(() => ParameterModel.Keys().Count, count =>
        {
            _parameterCount = count;
            RenderList();
        });
    }

    private async Task PollSummary()
    {
        if (!SetupModel.Connected(_setup)) return;
        var first = !_summaryOpened;
        _summaryOpened = true;
        var read = await Task.Run(() =>
        {
            if (first) FormWrites.Ok(SetupModel.PageOpened, SetupModel.SummaryPage);
            return Bridge.Get(SetupModel.SummaryPath);
        });
        var signature = read.ToJsonString();
        if (signature == _summarySignature || !SetupModel.Connected(_setup)) return;
        _summarySignature = signature;
        _summaries = SetupModel.Summaries(read);
        RenderList();
    }

    private void RenderHeader()
    {
        var readiness = SetupModel.Readiness(_setup);
        var vehicle = MapUiJson.Text(_firmware["vehicleType"]);
        FormLook.Say(SubtitleText, SetupModel.Subtitle(vehicle.Trim().Length > 0 ? vehicle : "Vehicle", MapUiJson.Text(_firmware["summary"])));
        FormLook.Say(VehicleText, readiness.VehicleId is { } id ? $"Vehicle {id}" : null);
        FormLook.Say(ReadinessText, SetupModel.ReadinessNote(readiness, Components.Any(c => c.NeedsAttention)));
    }

    private void EnsureOpened()
    {
        if (_parametersOpen || _overview) return;
        if (_opened is { } open && Components.Any(c => c.Name == open.Name)) return;
        _opened = SetupModel.FirstToOpen(Components, c => SetupModel.CanOpen(c, PageOf(c))) is { } first ? (first.Name, null) : null;
    }

    private void Select(SetupComponent component, string? section)
    {
        if (!NavigationLock.Allows()) return;
        _parametersOpen = false;
        _overview = false;
        _opened = (component.Name, section);
        _picked = true;
        RenderList();
        RenderDetail();
        Arrange();
    }

    private void OpenParameters(string search)
    {
        if (!NavigationLock.Allows()) return;
        _parametersSearch = search;
        _opened = null;
        _overview = false;
        _parametersOpen = true;
        _picked = true;
        RenderList();
        RenderDetail();
        Arrange();
    }

    private IReadOnlyList<string> HitsOf(SetupComponent component) =>
        _sectionHits.TryGetValue(component.Name, out var hits) ? hits : [];

    private async void RefreshSectionHits()
    {
        var version = ++_hitsVersion;
        var search = SearchBox.Text;
        var pages = search.Trim().Length == 0
            ? []
            : Components.Where(c => PageOf(c)?.ParameterSections == true && !SetupModel.OwnScreen(c)).Select(c => (c.Name, Page: PageOf(c)!.Name)).ToList();
        var hits = await Task.Run(() => pages.ToDictionary(
            page => page.Name,
            page => (IReadOnlyList<string>)FormParse.SetupSections(Bridge.Get($"view.setup({page.Page})"))
                .Where(section => SetupModel.SectionMatches(section, search))
                .Select(section => section.Title)
                .ToList()));
        if (version != _hitsVersion) return;
        _sectionHits = hits;
        RenderList();
    }

    private void RenderList()
    {
        if (!SetupModel.Connected(_setup) || TwoPane.Visibility != Visibility.Visible) return;
        var components = Components;
        var search = SearchBox.Text;
        bool Hit(SetupComponent c) => SetupModel.Matches(c.Name, search) || HitsOf(c).Count > 0;
        bool Opens(SetupComponent c) => SetupModel.CanOpen(c, PageOf(c));
        var attention = components.Where(c => c.NeedsAttention && Hit(c)).ToList();
        var remaining = components.Where(c => !c.NeedsAttention && Hit(c)).ToList();
        var usable = remaining.Where(Opens).ToList();
        var desktop = remaining.Where(c => !Opens(c)).ToList();
        var parameters = _advanced && SetupModel.Matches(SetupModel.ParametersPage, search);
        FormLook.Fill(ComponentList, new IEnumerable<UIElement?>[]
        {
            attention.Count > 0 ? [Header("Needs attention")] : [],
            attention.SelectMany(AttentionRows),
            components.Count == 0 ? [EmptyList()] : [],
            usable.Count > 0 ? [Header("Ready")] : [],
            usable.SelectMany(ReadyRows),
            desktop.Count > 0 ? [Header("Advanced")] : [],
            desktop.SelectMany(ReadyRows),
            parameters ? [desktop.Count == 0 ? Header("Advanced") : null, ParametersRow()] : [],
        }.SelectMany(rows => rows));
    }

    private UIElement EmptyList() =>
        SetupModel.ParametersIncomplete(_setup) is { } incomplete
            ? SetupUi.Notice(incomplete)
            : FormLook.Empty("", SetupModel.NothingToConfigure, SetupModel.NothingToConfigureText);

    private static UIElement Header(string text) =>
        FormLook.Margined(FormLook.Text(text, "FontSizeCaption", "TextTertiaryBrush", true), new Thickness(12, 16, 12, 4));

    private bool Selected(SetupComponent component) => !_parametersOpen && _opened?.Name == component.Name;

    private IReadOnlyList<SummaryLine> SummaryOf(SetupComponent component) =>
        _summaries.TryGetValue(component.Name, out var lines) ? lines : [];

    private string SubtitleOf(SetupComponent component) =>
        SummaryOf(component).Count == 0 ? SetupModel.Note(component.ClassName) : "";

    private IEnumerable<UIElement> AttentionRows(SetupComponent component) =>
        HitRows(component).Prepend(SetupUi.Row(
            MapUiJson.SentenceCase(component.Name),
            component.BlockedReason is { } blocked ? $"Not while {blocked}" : SetupModel.AttentionAction(component.ClassName),
            component.BlockedReason is null ? SetupState.NeedsAttention : SetupState.Unavailable,
            SetupModel.CanOpen(component, PageOf(component)) ? () => Select(component, null) : null,
            SummaryOf(component),
            SetupModel.Glyph(component.Known, component.ClassName),
            SubtitleOf(component),
            Selected(component)));

    private IEnumerable<UIElement> ReadyRows(SetupComponent component)
    {
        var openable = SetupModel.CanOpen(component, PageOf(component));
        var blocked = component.BlockedReason;
        return HitRows(component).Prepend(SetupUi.Row(
            MapUiJson.SentenceCase(component.Name),
            blocked is not null ? $"Not while {blocked}"
            : component.NeedsAttention ? SetupModel.AttentionAction(component.ClassName)
            : !openable ? "Not available here"
            : "",
            blocked is not null ? SetupState.Unavailable
            : component.NeedsAttention ? SetupState.NeedsAttention
            : !openable ? SetupState.Unavailable
            : SetupState.Neutral,
            openable ? () => Select(component, null) : null,
            SummaryOf(component),
            SetupModel.Glyph(component.Known, component.ClassName),
            SubtitleOf(component),
            Selected(component)));
    }

    private IEnumerable<UIElement> HitRows(SetupComponent component) =>
        HitsOf(component).Select(title =>
        {
            var button = FormLook.Button(MapUiJson.SentenceCase(title), () => Select(component, title), ButtonLook.Subtle);
            button.Margin = new Thickness(56, 0, 0, 0);
            button.HorizontalAlignment = HorizontalAlignment.Left;
            return (UIElement)button;
        });

    private UIElement ParametersRow() => SetupUi.Row(
        SetupModel.ParametersPage,
        "",
        SetupState.Neutral,
        () => OpenParameters(""),
        null,
        "",
        SetupModel.ParameterCountText(_parameterCount) ?? "Every setting the vehicle has",
        _parametersOpen);

    private void RenderDetail()
    {
        var open = _opened is { } wanted ? Components.FirstOrDefault(c => c.Name == wanted.Name) : null;
        var page = open is null ? null : PageOf(open);
        var kind = open is null ? DetailKind.Desktop : SetupModel.Kind(open, page);
        var section = _opened?.Section;
        var vehicle = MapUiJson.Number(_setup["vehicleId"]);
        var key = _parametersOpen ? $"{vehicle}|parameters|{_parametersSearch}"
            : open is null ? $"none|{string.Join(",", Components.Where(c => c.NeedsAttention).Select(c => c.Name))}"
            : $"{vehicle}|{open.Name}|{kind}|{open.Prerequisite}|{section}|{page?.Screen}";
        var blocked = _parametersOpen ? null : open?.BlockedReason;
        if (key != _detailKey)
        {
            _detailKey = key;
            DetailTitle.Text = _parametersOpen ? SetupModel.ParametersPage : open is null ? "" : MapUiJson.SentenceCase(open.Name);
            DetailHost.Content = _parametersOpen ? new ParametersPanel(_parametersSearch)
                : open is null ? Overview()
                : Body(open, kind, section);
            if (open is not null && !_parametersOpen && SetupModel.ReportsOpening(open.Prerequisite, page))
            {
                var name = page?.Name ?? open.Name;
                SetupUi.Detached(() => FormWrites.Ok(SetupModel.PageOpened, name));
            }
        }
        FormLook.Say(BlockedText, blocked is null ? null : SetupModel.DisabledWhile(blocked));
        DetailHost.IsEnabled = blocked is null;
        DetailHost.Opacity = blocked is null ? 1 : 0.5;
    }

    private UIElement Overview()
    {
        var attention = Components.Where(c => c.NeedsAttention).Select(c => MapUiJson.SentenceCase(c.Name)).ToList();
        return FormLook.Empty(OverviewGlyph, SetupModel.AircraftSetup, attention.Count == 0
            ? "Choose a component on the left."
            : $"Needs attention: {string.Join(", ", attention)}. Choose a component on the left.");
    }

    private UIElement Body(SetupComponent component, DetailKind kind, string? section) => kind switch
    {
        DetailKind.Prerequisite => Prerequisite(component),
        DetailKind.Sensors => new SensorsPanel(),
        DetailKind.Radio => new RadioPanel(),
        DetailKind.RemoteSupport => new RemoteSupportPanel(),
        DetailKind.SubMotors => new SubMotorsPanel(),
        DetailKind.Motors => new MotorsPanel(),
        DetailKind.FlightModes => new FlightModesPanel(),
        DetailKind.Px4Tuning => new TuningPanel(),
        DetailKind.Px4Airframe => new Px4AirframePanel(),
        DetailKind.Actuators => new ActuatorsPanel(),
        DetailKind.ApmServos => new ServosPanel(),
        DetailKind.ApmFollow => new FollowPanel(),
        DetailKind.Scripting => new ScriptingPanel(),
        DetailKind.Joystick => new JoystickPanel(),
        DetailKind.EspBridge => new EspBridgePanel(),
        DetailKind.Syslink => new SyslinkPanel(),
        DetailKind.SubFrame => new SubFramePanel(),
        DetailKind.ApmAirframe => new ApmAirframePanel(),
        DetailKind.OpticalFlow => new OpticalFlowPanel(),
        DetailKind.NotSupported => SetupUi.Notice("Not supported"),
        DetailKind.Form => SetupForms.Page(component.Page ?? component.Name, section, component.Known == "power"),
        _ => SetupUi.Notice($"{component.Name} cannot be set up in this app yet."),
    };

    private UIElement Prerequisite(SetupComponent component)
    {
        var first = component.Prerequisite ?? "";
        var firstComponent = Components.FirstOrDefault(c => c.Name == first);
        var button = FormLook.Button($"Set up {MapUiJson.SentenceCase(first)}", () =>
        {
            if (Components.FirstOrDefault(c => c.Name == first) is { } target) Select(target, null);
        }, ButtonLook.Accent);
        button.HorizontalAlignment = HorizontalAlignment.Center;
        return FormLook.Stack(0,
            FormLook.Empty(SetupModel.Glyph(firstComponent?.Known, firstComponent?.ClassName ?? ""), $"{MapUiJson.SentenceCase(first)} first", SetupModel.PrerequisiteText(first, component.Name)),
            button);
    }
}
