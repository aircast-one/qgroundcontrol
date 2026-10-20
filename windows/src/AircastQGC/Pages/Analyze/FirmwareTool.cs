using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;
using AircastQGC.Core;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.Windows.Storage.Pickers;

namespace AircastQGC.Pages.Analyze;

internal sealed record FirmwarePort(string Port, string Description, bool Bootloader, string BoardType);

internal sealed record FirmwareJob(string Phase, bool Busy, bool Cancellable, double Progress, IReadOnlyList<string> Messages, string Error,
    IReadOnlyList<(string Name, string Url)> Choices, int BestChoice, string UpdateAvailable, string Px4Stable, string Px4Beta)
{
    internal static FirmwareJob? Parse(JsonObject view) =>
        MapUiJson.Text(view["class"]) == "FirmwareUpgrade"
            ? new FirmwareJob(
                MapUiJson.Text(view["phase"]) is { Length: > 0 } phase ? phase : "idle",
                MapUiJson.Flag(view["busy"]),
                MapUiJson.Flag(view["cancellable"]),
                MapUiJson.Number(view["progress"]) ?? 0,
                view["messages"] is JsonArray messages ? messages.Select(MapUiJson.Text).ToList() : [],
                MapUiJson.Text(view["error"]),
                MapUiJson.Objects(view["choices"]).Select(choice => (MapUiJson.Text(choice["name"]), MapUiJson.Text(choice["url"]))).ToList(),
                MapUiJson.Number(view["bestChoice"]) is double best ? (int)best : -1,
                MapUiJson.Text(view["updateAvailable"]),
                MapUiJson.Text(view["px4StableVersion"]),
                MapUiJson.Text(view["px4BetaVersion"]))
            : null;
}

internal sealed class FirmwareTool : UserControl
{
    private const string JobPath = "view.firmwareUpgrade";
    private const string PortsPath = "view.firmwarePorts";
    private const string SettingsGroup = "settings.firmwareUpgradeSettings";
    private const string FromFile = "file";
    private const string DefaultSource = "px4:stable";
    private const int Px4Type = 12;
    private const int ApmType = 3;
    private const string MultipleDevices = "Multiple devices detected. Make sure to select the correct one from the list.";
    private const string FlashFailText = "If upgrade failed, make sure to connect directly to a powered USB port on your computer, not through a USB hub. Also make sure you are only powered via USB not battery.";
    private const string BetaWarning = "WARNING: BETA FIRMWARE. This firmware version is ONLY intended for beta testers. Although it has received FLIGHT TESTING, it represents actively changed code. Do NOT use for normal operation.";
    private const string DevWarning = "WARNING: CONTINUOUS BUILD FIRMWARE. This firmware has NOT BEEN FLIGHT TESTED. It is only intended for DEVELOPERS. Run bench tests without props first. Do NOT fly this without additional safety precautions. Follow the forums actively when using it.";

    private static readonly string[] ApmVehicles = ["copter", "heli", "plane", "rover", "sub"];
    private static readonly string[] FirmwareExtensions = ["px4", "apj", "bin", "ihx"];

    internal static readonly IReadOnlyList<(string Key, string Label)> Sources =
        new[] { (FromFile, "A firmware file"), ("px4:stable", "PX4 Pro, stable"), ("px4:beta", "PX4 Pro, beta"), ("px4:dev", "PX4 Pro, dev"), ("sik:stable", "SiK radio, stable") }
            .Concat(ApmVehicles.SelectMany(vehicle => new[] { "stable", "beta", "dev" }.Select(build =>
                ($"ardupilot:{vehicle}:{build}", $"ArduPilot {char.ToUpperInvariant(vehicle[0])}{vehicle[1..]}, {build}"))))
            .ToList();

    private readonly TextBlock _update = Ui.Text("", brush: "CautionBrush");
    private readonly TextBlock _noteTitle = Ui.Text("", semibold: true);
    private readonly TextBlock _multiple = Ui.Text(MultipleDevices, brush: "CautionBrush");
    private readonly ItemsControl _portList = Ui.Column([]);
    private readonly ComboBox _sourceBox = new() { Header = "Firmware", HorizontalAlignment = HorizontalAlignment.Stretch, FontFamily = PreflightLook.Font };
    private readonly ContentControl _chibios = new() { HorizontalContentAlignment = HorizontalAlignment.Stretch, IsTabStop = false };
    private readonly CheckBox _advancedBox = new() { Content = "Advanced settings", FontFamily = PreflightLook.Font };
    private readonly Button _bootloader;
    private readonly TextBlock _fileText = Ui.Text("");
    private readonly TextBlock _warning = Ui.Text("", brush: "CriticalBrush");
    private readonly TextBlock _flashing = Ui.Text("", wrap: false);
    private readonly Button _chooseFile;
    private readonly Button _flash;
    private readonly Button _cancel;
    private readonly ProgressBar _progress = new() { Minimum = 0, Maximum = 1 };
    private readonly TextBlock _phase = Ui.Text("");
    private readonly ItemsControl _choices = Ui.Column([], 8);
    private readonly TextBlock _error = Ui.Text("", brush: "CriticalBrush");
    private readonly TextBlock _failHelp = Ui.Text(FlashFailText);
    private readonly TextBlock _refusalText = Ui.Text("", brush: "CriticalBrush");
    private readonly TextBlock _log = Ui.Text("", "FontSizeCaption", mono: true).With(text => text.IsTextSelectionEnabled = true);
    private readonly DispatcherTimer _poll = new() { Interval = TimeSpan.FromMilliseconds(500) };

    private FirmwareJob? _job;
    private IReadOnlyList<FirmwarePort> _ports = [];
    private IReadOnlyList<JsonObject> _facts = [];
    private bool _apmVehicle;
    private string _port = "";
    private string? _file;
    private bool _advanced;
    private string _source = DefaultSource;
    private bool _sourceRestored;
    private string _refusal = "";
    private string _flashingName = "";
    private bool _painting;
    private string _portSignature = "";
    private string _sourceSignature = "";
    private string _choiceSignature = "";
    private string _chibiosSignature = "";

    public FirmwareTool()
    {
        _bootloader = Ui.Button("Flash ChibiOS bootloader", () => Ui.Off(() => Bridge.Invoke("vehicle.flashBootloader")));
        _chooseFile = Ui.Button("Choose file", ChooseFile);
        _flash = Ui.Button("Flash", Flash, Look.Accent);
        _cancel = Ui.Button("Cancel", () => Ask(() => Bridge.Invoke("firmware.cancel")));
        _sourceBox.SelectionChanged += (_, _) =>
        {
            if (_painting || _job?.Busy == true || _sourceBox.SelectedIndex < 0) return;
            ChooseSource(Offered(_advanced)[_sourceBox.SelectedIndex].Key);
        };
        _advancedBox.Click += (_, _) =>
        {
            _advanced = _advancedBox.IsChecked == true;
            _source = SourceAfterAdvanced(_source, _advanced);
            Render();
        };

        Content = new ScrollViewer
        {
            Content = new StackPanel
            {
                Padding = new Thickness(24, 0, 24, 24),
                Spacing = 12,
                Children =
                {
                    Ui.Card(new StackPanel
                    {
                        Spacing = 8,
                        Children =
                        {
                            _update,
                            _noteTitle,
                            Ui.Text("Plug in your device via USB, choose its port and either a release, downloaded once the board is identified, " +
                                "or a firmware file, then press Flash. A board running its firmware is asked to be unplugged and plugged back in " +
                                "so its bootloader starts.", brush: "TextSecondaryBrush"),
                        },
                    }),
                    _multiple,
                    _portList,
                    _sourceBox,
                    _chibios,
                    _advancedBox,
                    _bootloader.With(button => button.HorizontalAlignment = HorizontalAlignment.Left),
                    _fileText,
                    _warning,
                    _flashing,
                    new StackPanel
                    {
                        Orientation = Orientation.Horizontal,
                        Spacing = 12,
                        HorizontalAlignment = HorizontalAlignment.Right,
                        Children = { _chooseFile, _flash, _cancel },
                    },
                    _progress,
                    _phase,
                    _choices,
                    _error,
                    _failHelp,
                    _refusalText,
                    _log,
                },
            },
        };

        _poll.Tick += (_, _) => Poll();
        Loaded += (_, _) =>
        {
            Poll();
            _poll.Start();
        };
        Unloaded += (_, _) =>
        {
            _poll.Stop();
            if (_job?.Phase is "connecting" or "choosing") Lane.Run(() => Bridge.Invoke("firmware.cancel"));
        };
    }

    private void Poll() =>
        Ui.Off(this, () => (
            Job: FirmwareJob.Parse(Bridge.Get(JobPath)),
            Ports: MapUiJson.Objects(Bridge.Get(PortsPath)["ports"])
                .Where(entry => MapUiJson.Text(entry["port"]).Length > 0)
                .Select(entry => new FirmwarePort(MapUiJson.Text(entry["port"]), MapUiJson.Text(entry["description"]),
                    MapUiJson.Flag(entry["bootloader"]), MapUiJson.Text(entry["boardType"])))
                .ToList(),
            Facts: MapUiJson.Objects(Bridge.Group(SettingsGroup)["facts"]),
            Apm: Truthy(Bridge.Get("vehicle.apmFirmware")["value"])), read =>
        {
            _job = read.Job;
            var portsChanged = !read.Ports.SequenceEqual(_ports);
            _port = portsChanged && read.Job?.Busy != true
                ? Preselected(read.Ports, _port) ?? (read.Ports.Any(entry => entry.Port == _port) ? _port : "")
                : _port;
            _ports = read.Ports;
            _facts = read.Facts;
            _apmVehicle = read.Apm;
            RestoreSource();
            Render();
        });

    private static bool Truthy(JsonNode? node) => node switch
    {
        JsonValue value when value.TryGetValue<bool>(out var flag) => flag,
        JsonValue value when value.TryGetValue<double>(out var number) => number != 0,
        JsonValue value when value.TryGetValue<string>(out var text) => text.Equals("true", StringComparison.OrdinalIgnoreCase)
            || (double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed) && parsed != 0),
        _ => false,
    };

    internal static string? Preselected(IReadOnlyList<FirmwarePort> ports, string current) =>
        ports.FirstOrDefault(entry => entry.Port == current && entry.BoardType.Length > 0)?.Port
        ?? (ports.FirstOrDefault(entry => entry.BoardType == "Pixhawk") ?? ports.FirstOrDefault(entry => entry.BoardType == "SiK Radio"))?.Port;

    private int? Setting(string name) =>
        _facts.FirstOrDefault(fact => MapUiJson.Text(fact["name"]) == name) is { } fact && MapUiJson.Number(fact["value"]) is double value
            ? (int)value
            : null;

    private void RestoreSource()
    {
        if (_sourceRestored || _facts.Count == 0) return;
        _source = Setting("defaultFirmwareType") == ApmType
            ? $"ardupilot:{ApmVehicles.ElementAtOrDefault(Setting("apmVehicleType") ?? 0) ?? ApmVehicles[0]}:stable"
            : DefaultSource;
        _sourceRestored = true;
    }

    internal static IReadOnlyList<(string Name, int Value)> SourceSettings(string source)
    {
        var parts = source.Split(':');
        var vehicle = Array.IndexOf(ApmVehicles, parts.ElementAtOrDefault(1));
        return parts[0] switch
        {
            "px4" => [("defaultFirmwareType", Px4Type)],
            "ardupilot" when vehicle >= 0 => [("defaultFirmwareType", ApmType), ("apmVehicleType", vehicle)],
            "ardupilot" => [("defaultFirmwareType", ApmType)],
            _ => [],
        };
    }

    private void ChooseSource(string chosen)
    {
        _source = chosen;
        _sourceRestored = true;
        var settings = SourceSettings(chosen);
        Ui.Off(() => settings.ToList().ForEach(setting =>
            Bridge.Set($"{SettingsGroup}.{setting.Name}", JsonValue.Create(setting.Value))));
        Render();
    }

    internal static IReadOnlyList<(string Key, string Label)> Offered(bool advanced) =>
        advanced ? Sources : Sources.Where(source => source.Key.EndsWith(":stable", StringComparison.Ordinal)).ToList();

    internal static string SourceAfterAdvanced(string source, bool advanced) =>
        advanced || source.EndsWith(":stable", StringComparison.Ordinal) ? source
        : source.LastIndexOf(':') is var cut && cut > 0 ? $"{source[..cut]}:stable"
        : DefaultSource;

    private static string? Warning(string source) =>
        source.EndsWith(":beta", StringComparison.Ordinal) ? BetaWarning
        : source.EndsWith(":dev", StringComparison.Ordinal) ? DevWarning
        : null;

    private static string FlashingLabel(IReadOnlyList<FirmwarePort> ports, string port) =>
        $"Flashing - {(ports.FirstOrDefault(entry => entry.Port == port)?.Description is { Length: > 0 } description ? description : port)}";

    private string SourceLabel(string key, string label) => key switch
    {
        "px4:stable" when _job?.Px4Stable is { Length: > 0 } stable => $"PX4 Pro {stable}",
        "px4:beta" when _job?.Px4Beta is { Length: > 0 } beta => $"PX4 Pro {beta}",
        _ => label,
    };

    private string? Choice => _source == FromFile ? _file : _source;

    private async void ChooseFile()
    {
        var path = await Ui.PickFile(this, window => new FileOpenPicker(window)
        {
            SuggestedStartLocation = PickerLocationId.Downloads,
            FileTypeFilter = { ".px4", ".apj", ".bin", ".ihx" },
        });
        if (path == null) return;
        var accepted = FirmwareExtensions.Any(extension => path.EndsWith($".{extension}", StringComparison.OrdinalIgnoreCase));
        _refusal = accepted ? "" : "Choose a .px4, .apj, .bin or .ihx firmware file.";
        _file = accepted ? path : _file;
        Render();
    }

    private void Flash()
    {
        if (Choice is not { } chosen || _port.Length == 0) return;
        var port = _port;
        _flashingName = FlashingLabel(_ports, port);
        Ask(() => Bridge.Invoke("firmware.flash", new JsonArray(JsonValue.Create(port), JsonValue.Create(chosen))));
        Render();
    }

    private void Ask(Func<JsonObject> call) =>
        Ui.Off(this, () => Ui.Refusal(call()) ?? "", refusal =>
        {
            _refusal = refusal;
            Render();
        });

    private void Render()
    {
        _painting = true;
        var busy = _job?.Busy == true;

        _update.Text = _job?.UpdateAvailable ?? "";
        _update.Visibility = Ui.Shown(_update.Text.Length > 0);
        _noteTitle.Text = _ports.Count == 0 ? "No USB serial devices attached" : "Flash a board over USB";
        _multiple.Visibility = Ui.Shown(!busy && _ports.Count(entry => entry.BoardType.Length > 0) > 1);

        var portSignature = string.Join("|", _ports.Select(entry => $"{entry}")) + $"#{_port}#{busy}";
        if (portSignature != _portSignature)
        {
            _portSignature = portSignature;
            _portList.Fill(_ports.Select(PortRow));
        }

        var offered = Offered(_advanced);
        var labels = offered.Select(source => SourceLabel(source.Key, source.Label)).ToList();
        var sourceSignature = string.Join("|", labels);
        if (sourceSignature != _sourceSignature)
        {
            _sourceSignature = sourceSignature;
            _sourceBox.ItemsSource = labels;
        }
        _sourceBox.SelectedIndex = offered.Select(source => source.Key).ToList().IndexOf(_source);
        _sourceBox.IsEnabled = !busy;

        RenderChibios(busy);

        _advancedBox.IsChecked = _advanced;
        _advancedBox.IsEnabled = !busy;
        _bootloader.Visibility = Ui.Shown(_advanced && _apmVehicle);
        _bootloader.IsEnabled = !busy;
        _fileText.Visibility = Ui.Shown(_source == FromFile);
        _fileText.Text = _file is { } file ? System.IO.Path.GetFileName(file) : "No firmware file chosen";
        _warning.Text = Warning(_source) ?? "";
        _warning.Visibility = Ui.Shown(_warning.Text.Length > 0);
        _flashing.Visibility = Ui.Shown(busy);
        _flashing.Text = _flashingName.Length > 0 ? _flashingName : FlashingLabel(_ports, _port);

        _chooseFile.Visibility = Ui.Shown(_source == FromFile);
        _chooseFile.IsEnabled = !busy;
        _flash.IsEnabled = !busy && _port.Length > 0 && Choice != null;
        _cancel.Visibility = Ui.Shown(busy);
        _cancel.IsEnabled = _job?.Cancellable == true;

        _progress.Visibility = Ui.Shown(busy);
        _progress.Value = Math.Clamp(_job?.Progress ?? 0, 0, 1);
        _phase.Text = PhaseText(_job?.Phase ?? "");
        _phase.Visibility = Ui.Shown(_phase.Text.Length > 0);

        var choices = _job is { Phase: "choosing" } job ? job.Choices : [];
        var choiceSignature = string.Join("|", choices.Select(choice => choice.Url)) + $"#{_job?.BestChoice}";
        if (choiceSignature != _choiceSignature)
        {
            _choiceSignature = choiceSignature;
            _choices.Fill(choices.Select((choice, at) => (UIElement)Ui.Button(choice.Name,
                    () => Ask(() => Bridge.Invoke("firmware.choose", new JsonArray(JsonValue.Create(choice.Url)))),
                    at == _job?.BestChoice ? Look.Accent : Look.Standard)
                .With(button => button.HorizontalAlignment = HorizontalAlignment.Stretch)));
        }

        _error.Text = _job?.Error ?? "";
        _error.Visibility = Ui.Shown(_error.Text.Length > 0);
        _failHelp.Visibility = Ui.Shown(_job?.Phase == "failed");
        _refusalText.Text = _refusal;
        _refusalText.Visibility = Ui.Shown(_refusal.Length > 0);
        _log.Text = string.Join("\n", _job?.Messages ?? []);
        _log.Visibility = Ui.Shown(_log.Text.Length > 0);
        _painting = false;
    }

    private void RenderChibios(bool busy)
    {
        var fact = _source.StartsWith("ardupilot:", StringComparison.Ordinal)
            ? _facts.FirstOrDefault(entry => MapUiJson.Text(entry["name"]) == "apmChibiOS")
            : null;
        var signature = fact?.ToJsonString() + $"#{busy}";
        if (signature == _chibiosSignature) return;
        _chibiosSignature = signature;
        _chibios.Content = fact is null ? null : FactField(fact, busy);
    }

    private static UIElement FactField(JsonObject fact, bool busy)
    {
        var name = MapUiJson.Text(fact["name"]);
        var title = MapUiJson.Text(fact["shortDescription"]) is { Length: > 0 } description ? description : name;
        var choices = MapUiJson.Texts(fact["enumStrings"]);
        var editable = !busy && !MapUiJson.Flag(fact["readOnly"]);
        return choices.Count > 0 ? EnumField(name, title, choices, Ui.Int(fact["enumIndex"], -1), editable) : FlagField(name, title, Truthy(fact["value"]), editable);
    }

    private static UIElement EnumField(string name, string title, IReadOnlyList<string> choices, int index, bool editable)
    {
        var box = new ComboBox
        {
            Header = title,
            ItemsSource = choices,
            SelectedIndex = index,
            IsEnabled = editable,
            HorizontalAlignment = HorizontalAlignment.Stretch,
        };
        box.SelectionChanged += (_, _) =>
        {
            var chosen = box.SelectedIndex;
            if (chosen >= 0) Ui.Off(() => Bridge.Set($"{SettingsGroup}.{name}.enumIndex", JsonValue.Create(chosen)));
        };
        return box;
    }

    private static UIElement FlagField(string name, string title, bool on, bool editable)
    {
        var flag = new CheckBox { Content = title, IsChecked = on, IsEnabled = editable };
        flag.Click += (_, _) =>
        {
            var checkedNow = flag.IsChecked == true;
            Ui.Off(() => Bridge.Set($"{SettingsGroup}.{name}", JsonValue.Create(checkedNow)));
        };
        return flag;
    }

    private static string PhaseText(string phase) => phase switch
    {
        "connecting" => "Waiting for the bootloader",
        "choosing" => "Choose the firmware build",
        "erasing" => "Erasing",
        "programming" => "Programming",
        "verifying" => "Verifying",
        "complete" => "Upgrade complete",
        "failed" => "Upgrade failed",
        _ => "",
    };

    private UIElement PortRow(FirmwarePort entry)
    {
        var radio = new RadioButton
        {
            GroupName = "firmwarePort",
            IsChecked = entry.Port == _port,
            IsEnabled = _job?.Busy != true,
            Content = new StackPanel
            {
                Children =
                {
                    Ui.Text(entry.Description.Length > 0 ? entry.Description : entry.Port),
                    Ui.Caption(entry.Bootloader
                        ? "In its bootloader"
                        : string.Join(" · ", new[] { entry.BoardType, entry.Port }.Where(part => part.Length > 0))),
                },
            },
            Padding = new Thickness(8, 6, 8, 6),
        }.Named(entry.Description.Length > 0 ? entry.Description : entry.Port);
        radio.Click += (_, _) =>
        {
            _port = entry.Port;
            Render();
        };
        return radio;
    }
}
