using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Forms;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Pages.Settings;

internal sealed record CameraDraft(int? Stored, string Title, string Name, string Source, string Url, string Tilt, bool NeedsUrl, bool ChoosingKind, string? Refusal)
{
    public static readonly CameraDraft New = new(null, "", "", "", "", "", true, true, null);

    public static CameraDraft Edit(CameraEntry camera, IReadOnlyList<CameraKind> kinds) => new(
        camera.Stored, camera.Title, camera.Name, camera.Source, camera.Url,
        camera.Tilt is double tilt ? FormText.Spell(tilt) : "",
        kinds.All(kind => kind.Raw != camera.Source || kind.NeedsUrl), false, null);

    public static double? TypedTilt(string text) => FormText.Parse(text.Replace(',', '.'));

    public bool TiltReadable => Tilt.Trim().Length == 0 || TypedTilt(Tilt) is not null;

    public bool CanSave => (Url.Trim().Length > 0 || !NeedsUrl) && TiltReadable;

    public CameraDraft Picked(CameraKind kind) => this with
    {
        Source = kind.Raw,
        NeedsUrl = kind.NeedsUrl,
        Url = kind.NeedsUrl ? (Url.Length > 0 ? Url : kind.DefaultAddress) : "",
        ChoosingKind = false,
        Refusal = null,
    };
}

internal static class CameraCommands
{
    private const string Subject = "The camera list";

    public static string? Add(string name, string source, string url, double? tilt) => FormWrites.Call("cameras.add", Subject, name, source, url, tilt);

    public static string? Update(int slot, string name, string source, string url, double? tilt) => FormWrites.Call("cameras.update", Subject, slot, name, source, url, tilt);

    public static string? Remove(int slot) => FormWrites.Call("cameras.remove", Subject, slot);

    public static string? Move(int from, int to) => FormWrites.Call("cameras.move", Subject, from, to);

    public static void Show(int slot) => CameraList.Pick(slot);

    public static string? Save(CameraDraft draft) => draft.Stored is int slot
        ? Update(slot, draft.Name, draft.Source, draft.Url, CameraDraft.TypedTilt(draft.Tilt))
        : Add(draft.Name, draft.Source, draft.Url, CameraDraft.TypedTilt(draft.Tilt));
}

internal sealed class CamerasPanel : UserControl
{
    private const string NoSources = "No video sources yet. Cameras on the drone show up here by themselves. Choose Add to add one.";
    private static readonly TimeSpan UndoWindow = TimeSpan.FromSeconds(6);

    private sealed record Removed(int Slot, string Name, string Source, string Url, double? Tilt, string Title, bool Active);

    private readonly StackPanel _body = new();
    private readonly DispatcherTimer _undo = new() { Interval = UndoWindow };
    private CameraList? _list;
    private string? _notice;
    private Removed? _removed;
    private bool _busy;

    public CamerasPanel()
    {
        Content = _body;
        _undo.Tick += (_, _) =>
        {
            _undo.Stop();
            _removed = null;
            Render();
        };
        Live.Watch(this, CameraList.View, view =>
        {
            _list = CameraList.Parse(view);
            Render();
        });
        Unloaded += (_, _) => _undo.Stop();
    }

    private bool Editable => _list?.Readable == true;

    private void Render()
    {
        var add = FormLook.Button("Add", () => Open(CameraDraft.New), ButtonLook.Standard, "");
        add.IsEnabled = Editable && !_busy;
        AutomationProperties.SetName(add, "Add a video source");
        add.HorizontalAlignment = HorizontalAlignment.Right;
        var cameras = _list?.Cameras ?? [];
        FormLook.Fill(_body,
        [
            FormLook.Margined(add, new Thickness(0, 0, 0, 12)),
            _list is { Readable: false } unreadable ? FormLook.Caption(unreadable.Reason, "CriticalBrush") : null,
            _notice is { } notice ? FormLook.Caption(notice, "CriticalBrush") : null,
            _list is not null && cameras.Count == 0 ? FormLook.Footnote(NoSources) : null,
            cameras.Count > 0 ? FormLook.Card(cameras.Select(Row)) : null,
            _removed is { } gone ? FormLook.Margined(FormLook.Card(FormLook.Row(FormLook.Text($"Removed {gone.Title}."), UndoButton(gone))), new Thickness(0, 12, 0, 0)) : null,
        ]);
    }

    private UIElement Row(CameraEntry camera)
    {
        var (status, brush) = CameraList.StatusLook(camera.Status);
        var editable = Editable && camera.Stored is not null;
        var row = FormLook.Row(
            FormLook.Lead(14, FormLook.Dot(brush), FormLook.Stack(2,
                FormLook.Text(camera.Title),
                FormLook.Caption(camera.Detail),
                FormLook.Caption(camera.Problem, "CriticalBrush"))),
            FormLook.Line(8,
                camera.Active ? FormLook.Caption("On screen") : null,
                editable ? FormLook.Glyph("", 12) : null));
        if (!editable) return row;
        var button = FormLook.Stretched(row, camera.Title, () => Open(CameraDraft.Edit(camera, _list?.Kinds ?? [])));
        AutomationProperties.SetItemStatus(button, camera.Active ? $"{status}, on screen" : status);
        return button;
    }

    private Button UndoButton(Removed gone)
    {
        var button = FormLook.Button("Undo", () =>
        {
            _undo.Stop();
            _removed = null;
            var stored = _list?.Stored.Count ?? 0;
            Change(() => Restore(gone, stored), refusal => _notice = refusal);
        }, ButtonLook.Subtle);
        button.IsEnabled = !_busy;
        return button;
    }

    private static string? Restore(Removed gone, int stored)
    {
        var refused = CameraCommands.Add(gone.Name, gone.Source, gone.Url, gone.Tilt)
            ?? (gone.Slot < stored ? CameraCommands.Move(stored, gone.Slot) : null);
        if (refused is null && gone.Active) CameraCommands.Show(gone.Slot);
        return refused;
    }

    private async void Change(Func<string?> action, Action<string?> after)
    {
        _busy = true;
        Render();
        var refusal = await FormWrites.Off(action);
        after(refusal);
        _busy = false;
        Render();
    }

    private async void Open(CameraDraft draft)
    {
        if (XamlRoot is null || _list is not { } list) return;
        var outcome = await new CameraDialog(XamlRoot, list, draft).Run();
        if (outcome is not CameraDialog.Removal removal) return;
        _notice = removal.Refusal;
        if (removal.Refusal is null && list.Cameras.FirstOrDefault(c => c.Stored == removal.Slot) is { } entry)
        {
            _removed = new Removed(removal.Slot, entry.Name, entry.Source, entry.Url, entry.Tilt, entry.Title, entry.Active);
            _undo.Start();
        }
        Render();
    }
}

internal sealed class CameraDialog
{
    public sealed record Removal(int Slot, string? Refusal);

    private const string GimbalAimed = "Aimed by a gimbal";
    private const string TiltHelp = "For a camera fixed to the frame: 0 looks straight ahead, -90 straight down. Leave empty when a gimbal aims it. Used to place home and the plan on the video.";
    private const string TiltUnreadable = "Type the tilt as a number of degrees, like -30.";

    private readonly CameraList _list;
    private readonly ContentDialog _dialog;
    private CameraDraft _draft;
    private bool _naming;
    private Removal? _removal;

    public CameraDialog(XamlRoot root, CameraList list, CameraDraft draft)
    {
        _list = list;
        _draft = draft;
        _naming = draft.Stored is not null;
        _dialog = FormLook.Dialog(root, "", new Border(), null);
        _dialog.PrimaryButtonClick += async (_, args) =>
        {
            var deferral = args.GetDeferral();
            var saving = _draft;
            var refusal = await FormWrites.Off(() => CameraCommands.Save(saving));
            args.Cancel = refusal is not null;
            if (refusal is not null)
            {
                _draft = _draft with { Refusal = refusal };
                Render();
            }
            deferral.Complete();
        };
        _dialog.SecondaryButtonClick += async (_, args) =>
        {
            if (_draft.Stored is not int slot) return;
            var deferral = args.GetDeferral();
            _removal = new Removal(slot, await FormWrites.Off(() => CameraCommands.Remove(slot)));
            deferral.Complete();
        };
        _dialog.CloseButtonClick += (_, args) =>
        {
            if (!_draft.ChoosingKind || _draft.Stored is null) return;
            args.Cancel = true;
            _draft = _draft with { ChoosingKind = false };
            Render();
        };
    }

    public async Task<Removal?> Run()
    {
        Render();
        await FormLook.Show(_dialog);
        return _removal;
    }

    private CameraKind? Kind => _list.Kinds.FirstOrDefault(kind => kind.Raw == _draft.Source);

    private string TypeTitle => Kind?.Title ?? CameraList.KindLabel(_draft.Source);

    private void Render()
    {
        var choosing = _draft.ChoosingKind;
        _dialog.Title = choosing ? (_draft.Stored is null ? "Add video source" : "Type")
            : _draft.Stored is null ? TypeTitle : _draft.Title;
        _dialog.Content = new ScrollViewer
        {
            MaxHeight = 520,
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
            Content = choosing ? Chooser() : Form(),
        };
        _dialog.PrimaryButtonText = choosing ? "" : "Save";
        _dialog.SecondaryButtonText = !choosing && _draft.Stored is not null ? "Remove" : "";
        _dialog.CloseButtonText = "Cancel";
        _dialog.DefaultButton = choosing ? ContentDialogButton.None : ContentDialogButton.Primary;
        _dialog.IsPrimaryButtonEnabled = _draft.CanSave;
    }

    private UIElement Chooser() => FormLook.Stack(4,
        _list.Addable(_draft.Stored is null ? null : _draft.Source)
            .GroupBy(kind => kind.Group)
            .SelectMany(group => new UIElement[] { FormLook.Margined(FormLook.Caption(group.Key, "TextTertiaryBrush"), new Thickness(0, 8, 0, 0)) }
                .Concat(group.Select(KindRow)))
            .Append(FormLook.Caption(_draft.Refusal ?? "", "CriticalBrush")));

    private UIElement KindRow(CameraKind kind) => FormLook.Stretched(
        FormLook.Margined(FormLook.Stack(2, FormLook.Text(kind.Title), FormLook.Caption(kind.Description)), new Thickness(8, 6, 8, 6)),
        kind.Title,
        () => Pick(kind));

    private async void Pick(CameraKind kind)
    {
        if (_draft.Stored is not null || kind.NeedsUrl)
        {
            _draft = _draft.Picked(kind);
            Render();
            return;
        }
        var name = kind.Raw == CameraList.SyntheticSource ? CameraList.SyntheticLabel : "";
        var refusal = await FormWrites.Off(() => CameraCommands.Add(name, kind.Raw, "", null));
        if (refusal is null)
        {
            _dialog.Hide();
            return;
        }
        _draft = _draft with { Refusal = refusal };
        Render();
    }

    private UIElement Form()
    {
        var kind = Kind;
        var typeRow = FormLook.Row(FormLook.Stack(2, FormLook.Caption("Type"), FormLook.Text(TypeTitle)), FormLook.Glyph("", 12));
        typeRow.Padding = new Thickness(0, 6, 0, 6);
        var type = FormLook.Stretched(
            typeRow,
            "Change type",
            () =>
            {
                _draft = _draft with { ChoosingKind = true };
                Render();
            });
        var name = FormLook.Field(_draft.Name, "Name");
        var addName = FormLook.Button("Add a name", () =>
        {
            _naming = true;
            Render();
        }, ButtonLook.Subtle);
        var url = FormLook.Field(_draft.Url, "Address", kind?.Hint ?? "");
        url.IsSpellCheckEnabled = false;
        var urlNote = FormLook.Caption("");
        var tilt = FormLook.Field(_draft.Tilt, "Tilt (°)", GimbalAimed);
        var tiltNote = FormLook.Caption("");
        var refusal = FormLook.Caption(_draft.Refusal ?? "", "CriticalBrush");
        AutomationProperties.SetName(name, "Name");
        AutomationProperties.SetName(url, "Address");
        AutomationProperties.SetName(tilt, "Tilt");
        void Check()
        {
            _draft = _draft with { Name = name.Text, Url = url.Text, Tilt = tilt.Text };
            FormLook.Say(urlNote, _draft.Refusal ?? kind?.Description);
            urlNote.Foreground = FormLook.Brush(_draft.Refusal is null ? "TextSecondaryBrush" : "CriticalBrush");
            FormLook.Say(tiltNote, _draft.TiltReadable ? TiltHelp : TiltUnreadable);
            tiltNote.Foreground = FormLook.Brush(_draft.TiltReadable ? "TextSecondaryBrush" : "CriticalBrush");
            _dialog.IsPrimaryButtonEnabled = _draft.CanSave;
        }
        void Typed()
        {
            if (name.Text == _draft.Name && url.Text == _draft.Url && tilt.Text == _draft.Tilt) return;
            _draft = _draft with { Refusal = null };
            FormLook.Say(refusal, null);
            Check();
        }
        name.TextChanged += (_, _) => Typed();
        url.TextChanged += (_, _) => Typed();
        tilt.TextChanged += (_, _) => Typed();
        Check();
        return FormLook.Stack(12,
            type,
            _naming ? name : addName,
            _draft.NeedsUrl ? FormLook.Stack(4, url, urlNote) : FormLook.Text(kind?.Description ?? "", "FontSizeBody", "TextSecondaryBrush"),
            _draft.NeedsUrl ? null : refusal,
            FormLook.Stack(4, tilt, tiltNote));
    }
}
