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
using Windows.ApplicationModel.DataTransfer;

namespace AircastQGC.Pages.Settings;

internal sealed record SigningKey(string Name, bool InUse, bool ActiveOnVehicle);

internal sealed record SigningKeys(bool Vehicle, bool Armed, string State, string LinkName, string ActiveKey, int MinPassphrase, IReadOnlyList<SigningKey> Keys)
{
    public bool Pending => State is "enabling" or "disabling";

    public bool AnyActive => Vehicle && ActiveKey != "None";

    public static SigningKeys? Parse(JsonObject view) => !MapUiJson.Flag(view["available"])
        ? null
        : new SigningKeys(
            MapUiJson.Flag(view["vehicle"]),
            MapUiJson.Flag(view["armed"]),
            MapUiJson.Text(view["state"]),
            MapUiJson.Text(view["linkName"]),
            MapUiJson.Text(view["activeKey"]),
            (int)(MapUiJson.Number(view["minPassphraseLength"]) ?? 8),
            MapUiJson.Objects(view["keys"])
                .Select(key => new SigningKey(MapUiJson.Text(key["name"]), MapUiJson.Flag(key["inUse"]), MapUiJson.Flag(key["activeOnVehicle"])))
                .ToList());

    public static bool IsHex(string text) => text.Length == SigningKeysPanel.HexLength && text.All(Uri.IsHexDigit);

    public static bool CanAdd(string name, bool raw, string secret, int minPassphrase) =>
        name.Length > 0 && (raw ? IsHex(secret) : secret.Length >= minPassphrase);
}

internal sealed class SigningKeysPanel : UserControl
{
    public const int HexLength = 64;
    private const string View = "view.signingKeys";
    private const string AddPassphrase = "signingKeys.addPassphrase";
    private const string AddRaw = "signingKeys.addRaw";
    private const string Generate = "signingKeys.generate";
    private const string Remove = "signingKeys.remove";
    private const string Export = "signingKeys.export";
    private const string Enable = "signing.enable";
    private const string Disable = "signing.disable";
    private static readonly TimeSpan PendingPoll = TimeSpan.FromMilliseconds(500);
    private static readonly TimeSpan ClipboardLife = TimeSpan.FromSeconds(30);

    private readonly StackPanel _body = new();
    private readonly TextBlock _refusal = FormLook.Caption("", "CriticalBrush");
    private readonly Poller _poll;
    private SigningKeys? _keys;

    public SigningKeysPanel()
    {
        Content = FormLook.Stack(0, _body, _refusal);
        _poll = Live.Changes(this, PendingPoll, () => Bridge.Get(View), view =>
        {
            _keys = SigningKeys.Parse(view);
            Render();
        });
    }

    private void Render()
    {
        if (_keys is not { } keys)
        {
            _body.Children.Clear();
            return;
        }
        FormLook.Fill(_body,
        [
            FormLook.Heading("MAVLink 2 signing"),
            FormLook.Margined(FormLook.Caption("Signing keys should only be sent to the vehicle over secure links (e.g. USB)."), new Thickness(0, 0, 0, 8)),
            FormLook.Card(new UIElement?[] { keys.Vehicle ? FormLook.Pair("Active key", keys.ActiveKey) : null }
                .Concat(keys.Keys.Select(key => KeyRow(keys, key)))
                .Append(keys.Keys.Count == 0 ? FormLook.Margined(FormLook.Text("No keys configured", "FontSizeBody", "TextSecondaryBrush"), new Thickness(16, 12, 16, 12)) : null)
                .OfType<UIElement>()),
            FormLook.Margined(FormLook.Button("Add key…", AddKey), new Thickness(0, 8, 0, 0)),
        ]);
    }

    private UIElement KeyRow(SigningKeys keys, SigningKey key)
    {
        var otherActive = keys.AnyActive && !key.ActiveOnVehicle;
        Button Action(string text, Action click, bool enabled = true)
        {
            var button = FormLook.Button(text, click, ButtonLook.Subtle);
            button.IsEnabled = enabled;
            AutomationProperties.SetName(button, $"{text} {key.Name}");
            return button;
        }
        return FormLook.Row(
            FormLook.Stack(2, FormLook.Text(key.Name), FormLook.Caption(otherActive ? "Another key is active" : "")),
            FormLook.Line(4,
                !keys.AnyActive ? Action(keys.Pending ? "Configuring…" : "Enable", () => EnableKey(key.Name), keys.Vehicle && !keys.Pending) : null,
                keys.Vehicle && key.ActiveOnVehicle ? Action(keys.Pending ? "Disabling…" : "Disable", DisableKey, !keys.Pending) : null,
                key.InUse ? null : Action("Export", () => ExportKey(key.Name)),
                key.InUse ? null : Action("Delete", () => DeleteKey(key.Name))));
    }

    private async void Change(string path, params object?[] args)
    {
        FormLook.Say(_refusal, await FormWrites.Off(() => FormWrites.Call(path, FormWrites.VehicleSubject, args)));
        _poll.Now();
    }

    private async void EnableKey(string name)
    {
        if (await FormLook.Confirm(XamlRoot, "Send signing key",
                $"This will transmit key '{name}' to the vehicle over '{_keys?.LinkName}'. Only proceed if this link is secure (USB or trusted local network).",
                "OK"))
            Change(Enable, name);
    }

    private async void DisableKey()
    {
        if (_keys?.Armed != true)
        {
            Change(Disable);
            return;
        }
        await FormLook.Tell(XamlRoot, "Disable signing while armed?",
            "Vehicle is armed. ArduPilot will refuse to disable signing while armed and PX4 will not accept the disable packet without a valid signature. " +
            "The disable attempt will likely time out and leave the link in an inconsistent state.\n\nDisarm the vehicle first.");
    }

    private async void DeleteKey(string name)
    {
        if (await FormLook.Confirm(XamlRoot, "Delete signing key",
                $"Are you sure you want to delete '{name}'?\n\nIf a vehicle still has this key configured, you will no longer be able to communicate with it over a signed connection. " +
                "Raw or generated keys cannot be recovered — Export the hex first if you may need it later.",
                "OK"))
            Change(Remove, name);
    }

    private async void ExportKey(string name)
    {
        if (await FormWrites.Off(() => MapUiJson.Text(FormWrites.Result(Export, name))) is not { Length: > 0 } hex) return;
        var package = new DataPackage();
        package.SetText(hex);
        Clipboard.SetContentWithOptions(package, new ClipboardContentOptions { IsAllowedInHistory = false, IsRoamable = false });
        await FormLook.Tell(XamlRoot, $"Export key: {name}", "Key copied to clipboard. Store it securely — it will be cleared from the clipboard in 30 seconds.");
        await Task.Delay(ClipboardLife);
        Clipboard.Clear();
    }

    private async void AddKey()
    {
        if (XamlRoot is null || _keys is not { } keys) return;
        var name = FormLook.Field("", "Key name", "Enter a friendly name");
        var kind = new RadioButtons { ItemsSource = new[] { "Passphrase", "Raw key (hex)" }, SelectedIndex = 0, MaxColumns = 2 };
        var passphrase = new PasswordBox { PlaceholderText = $"Enter passphrase (min {keys.MinPassphrase} chars)" };
        var hex = FormLook.Field("", "", $"{HexLength} hex characters");
        hex.MaxLength = HexLength;
        hex.IsSpellCheckEnabled = false;
        var hint = FormLook.Caption("", "CriticalBrush");
        var error = FormLook.Caption("", "CriticalBrush");
        var rawRow = FormLook.Line(8, hex, FormLook.Button("Generate", async () =>
            hex.Text = await FormWrites.Off(() => MapUiJson.Text(FormWrites.Result(Generate)))));
        hex.Width = 320;
        AutomationProperties.SetName(kind, "Key type");
        AutomationProperties.SetName(passphrase, "Passphrase");
        AutomationProperties.SetName(hex, "Raw key");
        var dialog = FormLook.Dialog(XamlRoot, "Add signing key", FormLook.Stack(8, name, kind, passphrase, rawRow, hint, error), "OK");
        bool Raw() => kind.SelectedIndex == 1;
        string Secret() => Raw() ? hex.Text : passphrase.Password;
        void Check()
        {
            passphrase.Visibility = FormLook.Shown(!Raw());
            rawRow.Visibility = FormLook.Shown(Raw());
            var secret = Secret();
            FormLook.Say(hint,
                Raw() ? (secret.Length > 0 && secret.Length != HexLength ? $"{secret.Length}/{HexLength} hex characters" : null)
                : secret.Length > 0 && secret.Length < keys.MinPassphrase ? $"Passphrase too short ({secret.Length}/{keys.MinPassphrase})" : null);
            dialog.IsPrimaryButtonEnabled = SigningKeys.CanAdd(name.Text, Raw(), secret, keys.MinPassphrase);
        }
        name.TextChanged += (_, _) => Check();
        kind.SelectionChanged += (_, _) => Check();
        passphrase.PasswordChanged += (_, _) => Check();
        hex.TextChanged += (_, _) =>
        {
            var kept = new string(hex.Text.Where(Uri.IsHexDigit).ToArray());
            if (kept != hex.Text) hex.Text = kept;
            Check();
        };
        dialog.PrimaryButtonClick += async (_, args) =>
        {
            var deferral = args.GetDeferral();
            var (typed, raw, secret) = (name.Text, Raw(), Secret());
            var refused = await FormWrites.Off(() => FormWrites.Call(raw ? AddRaw : AddPassphrase, FormWrites.VehicleSubject, typed, secret));
            FormLook.Say(error, refused);
            args.Cancel = refused is not null;
            deferral.Complete();
        };
        Check();
        await FormLook.Show(dialog);
        _poll.Now();
    }
}
