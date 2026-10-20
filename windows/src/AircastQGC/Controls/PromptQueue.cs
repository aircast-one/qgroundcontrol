using System;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using Microsoft.UI.Xaml.Controls;

namespace AircastQGC.Controls;

internal static class PromptQueue
{
    private static readonly TimeSpan Retry = TimeSpan.FromSeconds(1);

    private static Task _tail = Task.CompletedTask;

    public static async Task<ContentDialogResult> Show(ContentDialog dialog, Func<bool> due)
    {
        var previous = _tail;
        var turn = new TaskCompletionSource();
        _tail = turn.Task;
        try
        {
            await previous;
            return await Open(dialog, due);
        }
        finally
        {
            turn.SetResult();
        }
    }

    private static async Task<ContentDialogResult> Open(ContentDialog dialog, Func<bool> due)
    {
        if (!due()) return ContentDialogResult.None;
        try
        {
            return await dialog.ShowAsync();
        }
        catch (COMException)
        {
            await Task.Delay(Retry);
            return await Open(dialog, due);
        }
    }
}
