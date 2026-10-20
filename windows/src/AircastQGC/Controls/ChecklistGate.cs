using System;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Core;
using AircastQGC.Forms;

namespace AircastQGC.Controls;

internal sealed class ChecklistGate
{
    private const string StatePath = "vehicle.checkListState";
    private const int Passed = 1;
    private const int Failed = 2;
    private const string VehiclesPath = "view.vehicles";
    private const string EnforcePath = "view.control(settings.appSettings.enforceChecklist)";
    private static readonly TimeSpan PopupDelay = TimeSpan.FromSeconds(1);
    private static readonly TimeSpan CloseDelay = TimeSpan.FromSeconds(1);

    private readonly PreflightChecklist _checklist;
    private readonly Func<bool> _deciding;
    private int? _vehicle;
    private int? _popupShownFor;
    private bool? _stateSent;
    private bool _enforced;
    private bool _passed;
    private int _popupTry;

    public ChecklistGate(PreflightChecklist checklist, Func<bool> deciding)
    {
        _checklist = checklist;
        _deciding = deciding;
        Live.Watch(checklist, EnforcePath, view => _enforced = FormJson.Truthy(view["value"]));
        Live.Watch(checklist, VehiclesPath, ApplyVehicles);
        checklist.StateChanged += (_, _) => Settle();
    }

    private bool PopupDue => _checklist.HasVehicle && _checklist.Offered && _enforced && !_checklist.Ready;

    private static int? ActiveVehicle(JsonObject vehicles) =>
        MapUiJson.Number(vehicles["activeId"]) is { } id && id > 0 ? (int)id : null;

    private void ApplyVehicles(JsonObject vehicles)
    {
        var id = ActiveVehicle(vehicles);
        if (id == _vehicle) return;
        _vehicle = id;
        _stateSent = null;
        _popupShownFor = id is null ? null : _popupShownFor;
        _checklist.Reset();
        if (id is { } vehicle) OfferPopup(vehicle);
    }

    private void Settle()
    {
        var passed = _checklist.Ready;
        var newlyPassed = passed && !_passed;
        _passed = passed;
        if (newlyPassed && _checklist.IsOpen) CloseSoon();
        Send(passed);
    }

    private void Send(bool passed)
    {
        if (_vehicle is null || _stateSent == passed) return;
        var unsentFailure = _stateSent is null && !passed;
        _stateSent = passed;
        if (!unsentFailure) _ = FormWrites.Off(() => Bridge.Set(StatePath, passed ? Passed : Failed));
    }

    private async void CloseSoon()
    {
        await Task.Delay(CloseDelay);
        if (_checklist.Ready && _checklist.IsOpen) _checklist.Hide();
    }

    private async void OfferPopup(int vehicle)
    {
        var attempt = ++_popupTry;
        await Task.Delay(PopupDelay);
        if (attempt != _popupTry || _vehicle != vehicle || _popupShownFor == vehicle) return;
        if (_deciding())
        {
            OfferPopup(vehicle);
            return;
        }
        if (!PopupDue) return;
        _popupShownFor = vehicle;
        _checklist.Show();
    }
}
