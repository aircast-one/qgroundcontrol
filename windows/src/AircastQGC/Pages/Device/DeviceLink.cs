using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;
using AircastQGC.Controls;
using AircastQGC.Core;
using AircastQGC.Forms;

namespace AircastQGC.Pages.Device;

internal sealed record DeviceRefusal(string Message, string Field);

internal sealed record DeviceAnswer(DeviceRefusal? Refusal, JsonObject? Result);

internal sealed record DeviceRequest(string Method, string Path, JsonObject? Body = null)
{
    public string Key => $"{Method} {Path} {Body?.ToJsonString()}";
}

internal static class DeviceLevel
{
    public const int Ready = 0;
    public const int Warn = 1;
    public const int Blocked = 2;
    public const int Neutral = 3;
}

internal static class DeviceLink
{
    public const string Page = "Aircast device";
    public const string Home = "aircraft";
    public const string Post = "POST";
    public const string Put = "PUT";
    public const string Delete = "DELETE";

    private const string ConnectPath = "device.connect";
    private const string DisconnectPath = "device.disconnect";
    private const string CallPath = "device.call";
    private const string ReadPath = "device.read";
    private const string UploadPath = "device.upload";
    private const string QrPath = "device.qr";
    private const string ReachPath = "device.reach";

    public static readonly DeviceRefusal NoAnswer = new("The device did not answer.", "");

    public static string ViewPath(string page) => $"view.device({page})";

    public static string Escaped(string text) => Uri.EscapeDataString(text);

    public static JsonArray Args(params object?[] args) =>
        new(args.Select(arg => arg is JsonNode node ? node.DeepClone() : FormJson.Node(arg)).ToArray());

    public static JsonObject Body(params (string Key, string Value)[] fields) =>
        new(fields.Select(field => KeyValuePair.Create(field.Key, (JsonNode?)JsonValue.Create(field.Value))));

    public static JsonObject Body(IEnumerable<KeyValuePair<string, string>> fields) =>
        new(fields.Select(field => KeyValuePair.Create(field.Key, (JsonNode?)JsonValue.Create(field.Value))));

    public static DeviceRefusal? Refusal(JsonObject? answer) =>
        answer is null || answer.Count == 0 ? NoAnswer
        : MapUiJson.Flag(answer["ok"]) ? null
        : new DeviceRefusal(MapUiJson.Text(answer["reason"]) is { Length: > 0 } reason ? reason : NoAnswer.Message, MapUiJson.Text(answer["field"]));

    public static Task<JsonObject> Invoke(string path, params object?[] args) => Task.Run(() => Bridge.Invoke(path, Args(args)));

    public static async Task<JsonNode?> Result(string path, params object?[] args) => (await Invoke(path, args))["result"];

    public static async Task<string?> Connect(string address) =>
        FormWrites.Refusal(await Invoke(ConnectPath, address), "The device");

    public static Task Disconnect() => Invoke(DisconnectPath);

    public static async Task<DeviceAnswer> Send(DeviceRequest request)
    {
        var answer = await Invoke(CallPath, request.Method, request.Path, request.Body);
        return new DeviceAnswer(Refusal(answer), answer["result"] as JsonObject);
    }

    public static async Task<DeviceRefusal?> Call(DeviceRequest request) => (await Send(request)).Refusal;

    public static async Task<DeviceAnswer> Upload(string method, string path, JsonArray parts)
    {
        var answer = await Invoke(UploadPath, method, path, parts);
        return new DeviceAnswer(Refusal(answer), answer["result"] as JsonObject);
    }

    public static JsonObject FilePart(string name, string file) => new() { ["name"] = name, ["file"] = file };

    public static JsonObject ValuePart(string name, string value) => new() { ["name"] = name, ["value"] = value };

    public static async Task<JsonObject?> Read(string path)
    {
        var answer = await Invoke(ReadPath, path);
        return MapUiJson.Flag(answer["ok"]) ? answer["result"] as JsonObject : null;
    }

    public static async Task<byte[]?> Qr(string text)
    {
        var answer = await Invoke(QrPath, text);
        if (!MapUiJson.Flag(answer["ok"]) || MapUiJson.Text(answer["result"]) is not { Length: > 0 } encoded) return null;
        try { return Convert.FromBase64String(encoded); }
        catch (FormatException) { return null; }
    }

    public static async Task<bool?> Reach(string url)
    {
        var answer = await Invoke(ReachPath, url);
        return MapUiJson.Flag(answer["ok"]) ? MapUiJson.Flag(answer["result"]) : null;
    }

    public static string Resolve(string origin, string path) =>
        origin.Length > 0 && Uri.TryCreate(origin, UriKind.Absolute, out var root) && Uri.TryCreate(root, path, out var full)
            ? full.ToString()
            : path;

    public static (string Path, IReadOnlyDictionary<string, string> Query) Split(string href)
    {
        var at = href.IndexOfAny(['?', '#']);
        var path = at < 0 ? href : href[..at];
        var query = href.IndexOf('?') is var q and >= 0
            ? href[(q + 1)..].Split('#')[0]
                .Split('&', StringSplitOptions.RemoveEmptyEntries)
                .Select(pair => pair.Split('=', 2))
                .GroupBy(pair => Uri.UnescapeDataString(pair[0]))
                .ToDictionary(group => group.Key, group => group.First().ElementAtOrDefault(1) is { } value ? Uri.UnescapeDataString(value.Replace('+', ' ')) : "")
            : new Dictionary<string, string>();
        return (path.Length == 0 ? "/" : path, query);
    }

    public static string Fragment(string href) => href.IndexOf('#') is var at and >= 0 ? href[(at + 1)..] : "";
}
