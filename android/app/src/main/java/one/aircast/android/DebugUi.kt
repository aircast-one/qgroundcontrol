package one.aircast.android

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.ActivityInfo
import kotlinx.coroutines.flow.MutableSharedFlow
import one.aircast.android.ui.FlyView
import one.aircast.android.ui.REQUESTABLE_SHEETS

internal const val DEBUG_UI_ACTION = "one.aircast.android.DEBUG_UI"
private const val DEBUG_COMMAND_BUFFER = 16

internal sealed interface DebugCommand {
    data class ShowTab(val tab: Tab) : DebugCommand
    data class ShowFlyView(val view: FlyView) : DebugCommand
    data class Orient(val orientation: Int) : DebugCommand
    data class EditLayout(val on: Boolean) : DebugCommand
    data object ResetLayout : DebugCommand
    data class Open(val target: String) : DebugCommand
}

private val ORIENTATIONS = mapOf(
    "landscape" to ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE,
    "portrait" to ActivityInfo.SCREEN_ORIENTATION_PORTRAIT,
    "auto" to ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED,
)

private val SWITCHES = mapOf("on" to true, "off" to false)

internal const val DEBUG_COMMANDS = "state, tab, fly-view, orientation, layout-edit, layout-reset, open"

private fun <T> named(value: String?, choices: Map<String, T>, what: String): Result<T> =
    choices.entries.firstOrNull { it.key.equals(value, ignoreCase = true) }?.let { Result.success(it.value) }
        ?: Result.failure(IllegalArgumentException("$what must be one of ${choices.keys.joinToString(", ")}"))

internal fun debugCommand(cmd: String?, value: String?): Result<DebugCommand> = when (cmd) {
    "tab" -> named(value, Tab.entries.associateBy { it.name }, "tab").map { DebugCommand.ShowTab(it) }
    "fly-view" -> named(value, FlyView.entries.associateBy { it.name }, "fly-view").map { DebugCommand.ShowFlyView(it) }
    "orientation" -> named(value, ORIENTATIONS, "orientation").map { DebugCommand.Orient(it) }
    "layout-edit" -> named(value, SWITCHES, "layout-edit").map { DebugCommand.EditLayout(it) }
    "layout-reset" -> Result.success(DebugCommand.ResetLayout)
    "open" -> value?.takeIf { it.isNotBlank() }?.let { Result.success(DebugCommand.Open(it)) }
        ?: Result.failure(IllegalArgumentException("open needs a sheet (${REQUESTABLE_SHEETS.joinToString(", ")}) or a flight action id, e.g. takeoff, rtl, land"))
    else -> Result.failure(IllegalArgumentException("cmd must be one of $DEBUG_COMMANDS"))
}

internal object DebugUi {
    val commands = MutableSharedFlow<DebugCommand>(extraBufferCapacity = DEBUG_COMMAND_BUFFER)

    @Volatile
    var state: String = "{}"
}

internal class DebugUiReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val cmd = intent.getStringExtra("cmd")
        resultData = if (cmd == "state") {
            DebugUi.state
        } else {
            debugCommand(cmd, intent.getStringExtra("value")).fold(
                onSuccess = { command -> if (DebugUi.commands.tryEmit(command)) "ok" else "error: busy, retry" },
                onFailure = { "error: ${it.message}" },
            )
        }
    }
}
