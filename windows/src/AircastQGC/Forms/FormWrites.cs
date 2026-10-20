using System;
using System.Globalization;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;

namespace AircastQGC.Forms;

public static class FormWrites
{
    public const string VehicleSubject = "The vehicle";
    public const string NotAccepted = "That change was not accepted.";
    public const string InvalidNumber = "Invalid number";

    public static Task<T> Off<T>(Func<T> work) => Task.Run(work);

    public static Task Off(Action work) => Task.Run(work);

    public static string? Refusal(JsonObject answer, string subject = VehicleSubject) =>
        answer.Count == 0 ? $"{subject} did not answer."
        : MapUiJson.Flag(answer["ok"]) ? null
        : MapUiJson.Text(answer["reason"]).Trim() is { Length: > 0 } reason ? reason
        : $"{subject} refused.";

    public static string? Set(string path, object? value) => Written(Bridge.Set(path, FormJson.Node(value)));

    public static string? Call(string path, string subject, params object?[] args) =>
        Refusal(Bridge.Invoke(path, FormJson.Args(args)), subject);

    public static bool Ok(string path, params object?[] args) => MapUiJson.Flag(Bridge.Invoke(path, FormJson.Args(args))["ok"]);

    public static JsonNode? Result(string path, params object?[] args) => Bridge.Invoke(path, FormJson.Args(args))["result"];

    public static bool ResultTrue(string path, params object?[] args) => FormJson.IsTrue(Result(path, args));

    public static string? Validate(FormControl control, string typed) =>
        Result($"{control.Path}.validate", typed, false) is JsonValue value && value.TryGetValue<string>(out var message) && message.Trim().Length > 0
            ? message.Trim()
            : null;

    public static string? Toggle(FormControl control, bool on) => Set(control.Path, on != control.Inverted);

    public static string? Choose(FormControl control, int index) =>
        control.RawChoice && index >= 0 && index < control.Options.Count
        && long.TryParse(control.Options[index].Raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out var raw)
            ? Set(control.Path, raw)
            : Set($"{control.Path}.enumIndex", index);

    public static string? Bits(FormControl control, long raw) => Set(control.Path, raw.ToString(CultureInfo.InvariantCulture));

    public static string? Press(FormControl control) => Set(control.Path, true);

    public static string? Text(FormControl control, string typed) => Validate(control, typed) ?? Set(control.Path, typed);

    public static string? Number(FormControl control, double typed) =>
        control.WholeNumbers && typed != Math.Floor(typed) ? InvalidNumber
        : Validate(control, typed.ToString("R", CultureInfo.InvariantCulture))
          ?? Set(control.Path, control.WholeNumbers || (control.Decimals == 0 && typed == Math.Floor(typed)) ? (object)(long)typed : typed);

    private static string? Written(JsonObject answer) =>
        MapUiJson.Flag(answer["ok"]) ? null
        : MapUiJson.Text(answer["reason"]).Trim() is { Length: > 0 } reason ? reason
        : NotAccepted;
}
