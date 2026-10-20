using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Settings;

internal sealed record CloudDrone(string Id, string Name, string Seen);

internal sealed record AccountState(
    bool SignedIn,
    bool SigningIn,
    string Status,
    string UserCode,
    string VerificationUrl,
    string ApiBase,
    IReadOnlyList<CloudDrone> Drones,
    string DronesStatus);

internal static partial class AircastCloud
{
    public const string LinkName = "Aircast cloud";
    public const string Account = "account";
    public const string ApiBasePath = "account.apiBase";
    public const string SignIn = "account.signIn";
    public const string SignOut = "account.signOut";
    public const string CancelSignIn = "account.cancelSignIn";
    public const string CreateLink = "links.createAircastCloudLink";
    public const string ApiBaseError = "Enter the account server address, starting with https://";
    public const string DeviceError = "Set up from the device to fill this in";

    [GeneratedRegex(@"^https?://[^/]+")]
    private static partial Regex ApiBasePattern();

    public static AccountState? State(JsonObject view) =>
        MapUiJson.Text(view["kind"]) != "object" ? null : new AccountState(
            MapUiJson.Flag(view["signedIn"]),
            MapUiJson.Flag(view["signingIn"]),
            MapUiJson.Text(view["status"]),
            MapUiJson.Text(view["userCode"]),
            MapUiJson.Text(view["verificationUrl"]),
            MapUiJson.Text(view["apiBase"]),
            MapUiJson.Objects(view["drones"])
                .Select(drone => new CloudDrone(MapUiJson.Text(drone["id"]), MapUiJson.Text(drone["name"]), MapUiJson.Text(drone["seen"])))
                .ToList(),
            MapUiJson.Text(view["dronesStatus"]));

    public static string? AdoptedApiBase(string field, AccountState? account) =>
        account is { SignedIn: true } && account.ApiBase.Trim().Length > 0 && account.ApiBase != field && field.Trim().Length == 0
            ? account.ApiBase
            : null;

    public static string Line(AccountState state) =>
        state.SignedIn ? "Signed in" : state.Status.Trim().Length > 0 ? state.Status : "Not signed in";

    public static string Pending(AccountState state, bool launched) =>
        launched
            ? $"Your browser opened {state.VerificationUrl} — approve code {state.UserCode} there."
            : $"Open {state.VerificationUrl} in your browser and approve code {state.UserCode} there.";

    public static bool ApiBaseValid(string apiBase) => ApiBasePattern().IsMatch(apiBase.Trim());

    public static bool DeviceValid(string deviceId) => deviceId.Trim().Length > 0;

    public static bool Valid(string apiBase, string deviceId) => ApiBaseValid(apiBase) && DeviceValid(deviceId);

    public static bool Offered() => State(Bridge.Get(Account)) is not null;

    public static bool Create(string name, string apiBase, string deviceId)
    {
        FormWrites.Set(ApiBasePath, apiBase);
        return FormWrites.ResultTrue(CreateLink, name, apiBase, deviceId);
    }

    public static bool StartSignIn(string apiBase)
    {
        FormWrites.Set(ApiBasePath, apiBase);
        return FormWrites.Ok(SignIn);
    }

    public static async Task<bool> Launch(string url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri)) return false;
        try { return await Windows.System.Launcher.LaunchUriAsync(uri); }
        catch (Exception) { return false; }
    }
}

internal sealed class AircastCloudFields : UserControl
{
    private static readonly TimeSpan Every = TimeSpan.FromSeconds(1);

    private readonly TextBox _apiBase;
    private readonly TextBox _device;
    private readonly TextBlock _apiError = FormLook.Caption("", "CriticalBrush");
    private readonly TextBlock _deviceError = FormLook.Caption("", "CriticalBrush");
    private readonly StackPanel _drones = new() { Spacing = 4 };
    private readonly TextBlock _dronesStatus = FormLook.Caption("");
    private readonly TextBlock _line = FormLook.Text("");
    private readonly TextBlock _pending = FormLook.Caption("");
    private readonly Button _signIn;
    private readonly Button _signOut;
    private readonly Button _cancel;
    private readonly Border _accountCard;
    private readonly Poller _poll;
    private AccountState? _account;
    private string _opened = "";
    private bool _launched;
    private bool _showErrors;

    public AircastCloudFields(string apiBase, string deviceId, Action<string> onApiBase, Action<string> onDeviceId)
    {
        _apiBase = FormLook.Field(apiBase, "Account server", "https://api.aircast.one");
        _device = FormLook.Field(deviceId, "Device");
        _apiBase.IsSpellCheckEnabled = false;
        _device.IsSpellCheckEnabled = false;
        _apiBase.TextChanged += (_, _) =>
        {
            onApiBase(_apiBase.Text);
            Validate();
        };
        _device.TextChanged += (_, _) =>
        {
            onDeviceId(_device.Text);
            Validate();
            RenderDrones();
        };
        _signIn = FormLook.Button("Sign in", () =>
        {
            var typed = _apiBase.Text;
            Run(() => AircastCloud.StartSignIn(typed));
        });
        _signOut = FormLook.Button("Sign out", () => Run(() => FormWrites.Ok(AircastCloud.SignOut)));
        _cancel = FormLook.Button("Cancel sign-in", () => Run(() => FormWrites.Ok(AircastCloud.CancelSignIn)));
        _accountCard = FormLook.Card(FormLook.Row(_line, FormLook.Line(0, _signIn, _signOut, _cancel)));
        Content = FormLook.Stack(6, _apiBase, _apiError, _drones, _dronesStatus, _device, _deviceError, _accountCard, _pending);
        _poll = Live.Changes(this, Every, () => Bridge.Get(AircastCloud.Account), view => Apply(AircastCloud.State(view)));
        Render();
    }

    public void ShowErrors()
    {
        _showErrors = true;
        Validate();
    }

    private async void Run(Func<bool> work)
    {
        await FormWrites.Off(work);
        _poll.Now();
    }

    private void Apply(AccountState? account)
    {
        _account = account;
        if (AircastCloud.AdoptedApiBase(_apiBase.Text, account) is { } adopted) _apiBase.Text = adopted;
        OpenBrowser(account);
        Render();
    }

    private async void OpenBrowser(AccountState? account)
    {
        if (account is not { SigningIn: true } || account.VerificationUrl.Trim().Length == 0 || account.VerificationUrl == _opened) return;
        _opened = account.VerificationUrl;
        _launched = false;
        _launched = await AircastCloud.Launch(account.VerificationUrl);
        Render();
    }

    private void Validate()
    {
        FormLook.Say(_apiError, _showErrors && !AircastCloud.ApiBaseValid(_apiBase.Text) ? AircastCloud.ApiBaseError : null);
        FormLook.Say(_deviceError, _showErrors && !AircastCloud.DeviceValid(_device.Text) ? AircastCloud.DeviceError : null);
    }

    private void Render()
    {
        var account = _account;
        RenderDrones();
        FormLook.Say(_dronesStatus, account is { SignedIn: true } ? account.DronesStatus : null);
        _accountCard.Visibility = FormLook.Shown(account is not null);
        FormLook.Say(_line, account is null ? null : $"Account  {AircastCloud.Line(account)}");
        FormLook.Say(_pending, account is { SigningIn: true } && account.VerificationUrl.Length > 0 ? AircastCloud.Pending(account, _launched) : null);
        _signOut.Visibility = FormLook.Shown(account is { SignedIn: true });
        _cancel.Visibility = FormLook.Shown(account is { SignedIn: false, SigningIn: true });
        _signIn.Visibility = FormLook.Shown(account is { SignedIn: false, SigningIn: false });
    }

    private void RenderDrones()
    {
        var drones = _account is { SignedIn: true } account ? account.Drones : [];
        var chosen = _device.Text.Trim();
        FormLook.Fill(_drones, drones.Count == 0
            ? []
            : drones.Select(drone => Choice(drone, chosen)).Prepend(FormLook.Text("Your drones", "FontSizeBody", "TextPrimaryBrush", true)));
        _drones.Visibility = FormLook.Shown(drones.Count > 0);
    }

    private UIElement Choice(CloudDrone drone, string chosen)
    {
        var choice = new RadioButton
        {
            Content = FormLook.Stack(0, FormLook.Text(drone.Name), FormLook.Caption(drone.Seen)),
            IsChecked = drone.Id == chosen,
            GroupName = "cloudDrone",
        };
        AutomationProperties.SetName(choice, drone.Name);
        choice.Checked += (_, _) =>
        {
            if (_device.Text.Trim() != drone.Id) _device.Text = drone.Id;
        };
        return choice;
    }
}
