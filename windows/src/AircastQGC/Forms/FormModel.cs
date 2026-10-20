using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json.Nodes;
using AircastQGC.Controls;

namespace AircastQGC.Forms;

public sealed record FormOption(string Label, string Raw, string Group);

public sealed record FormBit(string Label, long Raw);

public sealed record FormSlider(double From, double To, int Decimals, double Step);

public sealed record FormControl
{
    public static readonly IReadOnlySet<string> KnownKinds =
        new HashSet<string> { "toggle", "choice", "bitmask", "text", "number", "label", "dialog", "button" };

    public string Path { get; init; } = "";
    public string Name { get; init; } = "";
    public string Label { get; init; } = "";
    public string ShortLabel { get; init; } = "";
    public string Kind { get; init; } = "";
    public JsonNode? Value { get; init; }
    public string ValueString { get; init; } = "";
    public string Display { get; init; } = "";
    public string Units { get; init; } = "";
    public bool ReadOnly { get; init; }
    public bool Enabled { get; init; } = true;
    public string DisabledReason { get; init; } = "";
    public IReadOnlyList<FormOption> Options { get; init; } = [];
    public IReadOnlyList<FormBit> Bits { get; init; } = [];
    public bool WholeNumbers { get; init; }
    public string MinimumText { get; init; } = "";
    public string MaximumText { get; init; } = "";
    public string DefaultText { get; init; } = "";
    public bool VehicleReboot { get; init; }
    public bool AppRestart { get; init; }
    public FormSlider? Slider { get; init; }
    public string Description { get; init; } = "";
    public bool Warning { get; init; }
    public bool SmallFont { get; init; }
    public bool Indent { get; init; }
    public bool Inverted { get; init; }
    public bool RawChoice { get; init; }
    public bool FirstEntryIsAll { get; init; }
    public string Keywords { get; init; } = "";
    public string LongDescription { get; init; } = "";
    public string Problem { get; init; } = "";
    public JsonObject? Calculator { get; init; }

    public string Title => (Label.Length > 0 ? Label : Name).TrimEnd(':', ' ');

    public string Heading => ShortLabel.TrimEnd(':', ' ') is { Length: > 0 } heading ? heading : Title;

    public string Detail =>
        ShortLabel.Length > 0 && Options.All(o => o.Label != Label) && !FormText.Restates(Label, Heading)
            ? FormText.LabelCase(Label)
            : "";

    public bool IsToggle => Kind == "toggle";
    public bool IsChoice => Kind == "choice" && Options.Count > 0;
    public bool IsBitmask => Kind == "bitmask" && Bits.Count > 0;
    public bool IsText => Kind == "text";
    public bool IsNumber => Kind == "number";
    public bool IsLabel => Kind == "label";
    public bool IsButton => Kind == "button";
    public bool IsDialog => Kind == "dialog";
    public bool Understood => Kind.Length == 0 || KnownKinds.Contains(Kind);
    public bool AcceptsWrite => !ReadOnly && Enabled;
    public bool Secret => Name.EndsWith("Password", StringComparison.OrdinalIgnoreCase);

    public int ChoiceIndex => Options.Select(o => o.Label).ToList().IndexOf(Display);

    public bool On =>
        (FormJson.IsTrue(Value) || ValueString.Equals("true", StringComparison.OrdinalIgnoreCase) || ValueString == "1") != Inverted;

    public double? Number => FormText.Parse(ValueString) ?? MapUiJson.Number(Value);

    public string LivePath => Path.StartsWith("vehicleConfig(", StringComparison.Ordinal) && Name.Length > 0
        ? $"vehicle.parameterManager.getParameter(-1,{Name})"
        : Path;

    public long BitsValue => (long)(MapUiJson.Number(Value) ?? FormText.Parse(ValueString) ?? 0);

    public int Decimals => ValueString.Trim().Split('.').Skip(1).FirstOrDefault()?.TakeWhile(char.IsDigit).Count() ?? 0;

    public double Step => Slider is { Step: > 0 } slider ? slider.Step : Math.Pow(10, -Decimals);

    public string UnitsShown => FormText.Seconds.Contains(Units.Trim().ToLowerInvariant()) ? "s" : Units.Trim();

    public string ShownValue => string.Join(" ", new[]
    {
        IsChoice ? MapUiJson.SentenceCase(Display) : Display.Length > 0 ? Display : ValueString,
        IsChoice ? "" : UnitsShown,
    }.Where(part => part.Length > 0));

    public string RebootNote =>
        VehicleReboot ? "Reboot vehicle for changes to take effect."
        : AppRestart ? "Restart Aircast for this to take effect."
        : "";

    public string InertNote => !Enabled ? (DisabledReason.Length > 0 ? DisabledReason : "Has no effect yet") : "Read-only";

    public string ConstraintNote => FormText.Joined(
        MinimumText.Length > 0 ? $"Min {FormText.PlainNumber(MinimumText)}" : "",
        MaximumText.Length > 0 ? $"Max {FormText.PlainNumber(MaximumText)}" : "",
        DefaultText.Length > 0 ? $"Default {FormText.PlainNumber(DefaultText)}" : "");

    public bool BitSet(FormBit bit) => (BitsValue & bit.Raw) != 0;

    public bool BitEnabled(int index) =>
        !(FirstEntryIsAll && index > 0 && Bits.Count > 0 && (BitsValue & Bits[0].Raw) != 0);

    public long BitsToggled(int index) =>
        (BitsValue & Bits[index].Raw) != 0 ? BitsValue & ~Bits[index].Raw
        : FirstEntryIsAll && index == 0 ? Bits.Skip(1).Aggregate(BitsValue, (raw, other) => raw & ~other.Raw) | Bits[0].Raw
        : BitsValue | Bits[index].Raw;

    public string BitsSummary => Summary(Bits.Where(BitSet).Select(b => b.Label).ToList(), Bits.Count);

    private static string Summary(IReadOnlyList<string> set, int all) =>
        set.Count == 0 ? "None" : set.Count == all ? "All" : string.Join(", ", set);

    public static FormControl? Parse(JsonObject json) =>
        MapUiJson.Text(json["label"]).Length == 0 && MapUiJson.Text(json["name"]).Length == 0
            ? null
            : new FormControl
            {
                Path = MapUiJson.Text(json["path"]),
                Name = MapUiJson.Text(json["name"]),
                Label = MapUiJson.Text(json["label"]),
                ShortLabel = MapUiJson.Text(json["shortLabel"]),
                Kind = MapUiJson.Text(json["control"]),
                Value = json["value"]?.DeepClone(),
                ValueString = FormJson.Shown(json["valueString"]),
                Display = FormJson.Shown(json["display"]),
                Units = MapUiJson.Text(json["units"]),
                ReadOnly = MapUiJson.Flag(json["readOnly"]),
                Enabled = MapUiJson.Flag(json["enabled"], true),
                DisabledReason = MapUiJson.Text(json["disabledReason"]),
                Options = MapUiJson.Objects(json["options"])
                    .Select(o => new FormOption(MapUiJson.Text(o["label"]), FormJson.Shown(o["raw"]), MapUiJson.Text(o["group"])))
                    .ToList(),
                Bits = MapUiJson.Text(json["control"]) != "bitmask"
                    ? []
                    : MapUiJson.Objects(json["bits"])
                        .Select(b => (Label: MapUiJson.Text(b["label"]), Raw: FormText.Parse(FormJson.Shown(b["raw"]))))
                        .Where(b => b.Raw is not null)
                        .Select(b => new FormBit(b.Label, (long)b.Raw!.Value))
                        .ToList(),
                WholeNumbers = MapUiJson.Flag(json["wholeNumbersOnly"]),
                MinimumText = FormJson.Shown(json["minimumText"]),
                MaximumText = FormJson.Shown(json["maximumText"]),
                DefaultText = FormJson.Shown(json["defaultText"]),
                VehicleReboot = MapUiJson.Flag(json["vehicleRebootRequired"]),
                AppRestart = MapUiJson.Flag(json["applicationRestartRequired"]),
                Slider = json["slider"] is JsonObject slider ? FormJson.Slider(slider) : null,
                Description = MapUiJson.Text(json["description"]),
                Warning = MapUiJson.Flag(json["warning"]),
                SmallFont = MapUiJson.Flag(json["smallFont"]),
                Indent = MapUiJson.Flag(json["indent"]),
                Inverted = MapUiJson.Flag(json["inverted"]),
                RawChoice = MapUiJson.Flag(json["rawChoice"]),
                FirstEntryIsAll = MapUiJson.Flag(json["firstEntryIsAll"]),
                Keywords = MapUiJson.Text(json["keywords"]),
                LongDescription = MapUiJson.Text(json["longDescription"]),
                Problem = MapUiJson.Text(json["problem"]),
                Calculator = json["calculator"] as JsonObject,
            };
}

public sealed record FormBlock(string Title, IReadOnlyList<FormControl> Controls)
{
    public const string Advanced = "Advanced";

    public string? SharedRebootNote => Shared(Controls.Select(c => c.RebootNote).Where(n => n.Length > 0).ToList());

    public string? SharedInertNote => Shared(Controls.Where(c => !c.Enabled).Select(c => c.InertNote).ToList());

    private static string? Shared(IReadOnlyList<string> notes) =>
        notes.Count > 1 && notes.Distinct().Count() == 1 ? notes[0] : null;
}

public sealed record FormSection(
    string Title,
    string Group,
    string Note,
    string Image,
    IReadOnlyList<string> Keywords,
    IReadOnlyList<FormBlock> Blocks)
{
    public IEnumerable<FormControl> Controls => Blocks.SelectMany(b => b.Controls);
}

public static class FormParse
{
    public static IReadOnlyList<FormSection> SettingsSections(JsonObject page, Func<string, bool>? drawnByHost = null) =>
        MapUiJson.Objects(page["sections"])
            .Select(section => (Section: section, Group: MapUiJson.Text(section["group"])))
            .Select(s => new FormSection(
                MapUiJson.Text(s.Section["title"]),
                s.Group,
                MapUiJson.Text(s.Section["note"]),
                "",
                [],
                drawnByHost?.Invoke(s.Group) == true
                    ? [new FormBlock("", [])]
                    : MapUiJson.Objects(s.Section["subsections"])
                        .Select(sub => new FormBlock(MapUiJson.Text(sub["title"]), Controls(sub["controls"])))
                        .Where(block => block.Controls.Count > 0)
                        .ToList()))
            .Where(section => section.Blocks.Count > 0)
            .ToList();

    public static IReadOnlyList<FormSection> SetupSections(JsonObject page) =>
        MapUiJson.Objects(page["sections"])
            .Select(section => (Section: section, Controls: Controls(section["controls"])))
            .Where(s => s.Controls.Count > 0)
            .Select(s => new FormSection(
                MapUiJson.Text(s.Section["title"]),
                "",
                FormText.Spaced(MapUiJson.Text(s.Section["note"]), ReadOnlyNote(s.Controls)),
                MapUiJson.Text(s.Section["image"]),
                MapUiJson.Texts(s.Section["keywords"]),
                [new FormBlock("", s.Controls)]))
            .ToList();

    public static string ReadOnlyNote(IReadOnlyList<FormControl> controls) =>
        LockedNote(controls.Where(c => c.ReadOnly).Select(c => c.Name).ToList(), controls.Count);

    private static string LockedNote(IReadOnlyList<string> locked, int all) =>
        locked.Count == 0 ? ""
        : locked.Count == all ? "This firmware reports all of these as read-only, so they are shown for reference."
        : $"This firmware reports {string.Join(", ", locked)} as read-only, so they are shown but cannot be changed here.";

    private static IReadOnlyList<FormControl> Controls(JsonNode? node) =>
        MapUiJson.Objects(node).Select(FormControl.Parse).OfType<FormControl>().ToList();
}

public static class FormText
{
    internal static readonly IReadOnlySet<string> Seconds = new HashSet<string> { "s", "sec", "secs", "second", "seconds" };

    public static double? Parse(string text) =>
        double.TryParse(text.Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out var number) && double.IsFinite(number)
            ? number
            : null;

    public static string PlainNumber(string text) =>
        Parse(text) is not { } number || Math.Abs(number) >= 1e15 ? text
        : number == Math.Floor(number) ? ((long)number).ToString(CultureInfo.InvariantCulture)
        : ((decimal)number).ToString("0.############", CultureInfo.InvariantCulture);

    public static string Spell(double number) => PlainNumber(number.ToString("R", CultureInfo.InvariantCulture));

    public static string LabelCase(string note) =>
        !note.TrimEnd().EndsWith('.') && note.Split(' ').Length <= 4 ? MapUiJson.SentenceCase(note) : note;

    public static bool Restates(string note, string label) =>
        Overlaps(note.TrimEnd('.').ToLowerInvariant(), label.ToLowerInvariant());

    public static string Joined(params string[] parts) => string.Join(" · ", parts.Where(p => p.Length > 0));

    public static string Spaced(params string[] parts) => string.Join(" ", parts.Where(p => p.Length > 0));

    private static bool Overlaps(string note, string label) =>
        note.Length > 0 && label.Length > 0
        && (label.StartsWith(note) || note.StartsWith(label) || note.EndsWith(" " + label) || label.EndsWith(" " + note));
}

public static class FormJson
{
    public static string Shown(JsonNode? node) => node switch
    {
        JsonValue value when value.TryGetValue<string>(out var text) => text,
        JsonValue value when value.TryGetValue<bool>(out var flag) => flag ? "true" : "false",
        JsonValue value when value.TryGetValue<double>(out var number) => FormText.Spell(number),
        _ => "",
    };

    public static bool IsTrue(JsonNode? node) => node is JsonValue value && value.TryGetValue<bool>(out var flag) && flag;

    public static bool Truthy(JsonNode? node) => node switch
    {
        JsonValue value when value.TryGetValue<bool>(out var flag) => flag,
        JsonValue value when value.TryGetValue<double>(out var number) => number != 0,
        JsonValue value when value.TryGetValue<string>(out var text) =>
            text.Equals("true", StringComparison.OrdinalIgnoreCase) || (FormText.Parse(text) is { } n && n != 0),
        _ => false,
    };

    public static FormSlider? Slider(JsonObject json) =>
        MapUiJson.Number(json["from"]) is { } from && MapUiJson.Number(json["to"]) is { } to && to > from
            ? new FormSlider(from, to, (int)(MapUiJson.Number(json["decimals"]) ?? 2), MapUiJson.Number(json["step"]) ?? 0)
            : null;

    public static JsonNode? Node(object? value) => value switch
    {
        null => null,
        JsonNode node => node,
        bool flag => JsonValue.Create(flag),
        int number => JsonValue.Create(number),
        long number => JsonValue.Create(number),
        double number => JsonValue.Create(number),
        string text => JsonValue.Create(text),
        _ => JsonValue.Create(Convert.ToString(value, CultureInfo.InvariantCulture)),
    };

    public static JsonArray Args(params object?[] args) => new(args.Select(Node).ToArray());
}
