package one.aircast.android.ui

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import android.text.format.DateUtils
import android.text.format.Formatter
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import java.io.File
import java.time.Instant
import kotlin.math.roundToInt
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.AppInstall
import one.aircast.android.R
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcDerived
import one.aircast.android.installedFromPlay
import one.aircast.android.installerOf
import one.aircast.map.optText
import org.json.JSONObject

internal const val APP_UPDATE_VIEW = "view.appUpdate"
internal const val PLAY_UPDATES = "Updates come through Google Play"
internal const val DISCONNECT_FIRST = "Disconnect from the vehicle first"
private const val UPDATE_SUBJECT = "The updater"
private const val NOTES_MAX_LINES = 12

internal enum class UpdatePhase(val token: String) {
    Idle("idle"), Checking("checking"), UpToDate("upToDate"), Available("available"), Downloading("downloading"), Ready("ready"), Failed("failed");

    companion object {
        fun of(token: String): UpdatePhase = entries.firstOrNull { it.token == token } ?: Idle
    }
}

internal data class AppUpdateReading(
    val current: String,
    val channel: String,
    val phase: UpdatePhase,
    val latest: String,
    val notes: String,
    val releaseUrl: String,
    val assetName: String,
    val assetUrl: String,
    val sizeBytes: Long,
    val progress: Float,
    val file: String,
    val error: String,
    val checkedAt: String,
    val autoCheck: Boolean,
    val snoozed: Boolean,
    val canInstall: Boolean,
)

internal fun appUpdateReading(view: JSONObject?): AppUpdateReading? = view?.let { json ->
    AppUpdateReading(
        current = json.optText("current"),
        channel = json.optText("channel"),
        phase = UpdatePhase.of(json.optText("phase")),
        latest = json.optText("latest"),
        notes = json.optText("notes"),
        releaseUrl = json.optText("releaseUrl"),
        assetName = json.optText("assetName"),
        assetUrl = json.optText("assetUrl"),
        sizeBytes = json.optLong("sizeBytes"),
        progress = json.optDouble("progress", 0.0).toFloat(),
        file = json.optText("file"),
        error = json.optText("error"),
        checkedAt = json.optText("checkedAt"),
        autoCheck = json.optBoolean("autoCheck", true),
        snoozed = json.optText("snoozed").isNotEmpty(),
        canInstall = json.optBoolean("canInstall"),
    )
}

internal fun updateStatus(reading: AppUpdateReading, fromPlay: Boolean, installError: String? = null): String = when {
    fromPlay -> PLAY_UPDATES
    reading.phase == UpdatePhase.Checking -> "Checking for updates…"
    reading.phase == UpdatePhase.UpToDate -> "Aircast is up to date"
    reading.phase == UpdatePhase.Available -> "Aircast ${reading.latest} is available"
    reading.phase == UpdatePhase.Downloading -> "Downloading Aircast ${reading.latest}… ${(reading.progress * 100).roundToInt()}%"
    reading.phase == UpdatePhase.Ready -> installError ?: "Aircast ${reading.latest} is ready to install"
    reading.phase == UpdatePhase.Failed -> reading.error.ifBlank { "The update check failed." }
    else -> ""
}

internal fun bannerShown(reading: AppUpdateReading, fromPlay: Boolean): Boolean =
    !fromPlay && reading.canInstall && !reading.snoozed &&
        reading.phase in setOf(UpdatePhase.Available, UpdatePhase.Downloading, UpdatePhase.Ready)

internal fun installRefusal(reading: AppUpdateReading): String? = DISCONNECT_FIRST.takeUnless { reading.canInstall }

private val MARKDOWN_LINK = Regex("""\[([^\]]+)]\([^)]*\)""")
private val MARKDOWN_MARKS = Regex("""(?m)^[ \t]{0,3}(#{1,6}[ \t]+|[-*+][ \t]+|>[ \t]*)|[*_`]+""")

internal fun plainNotes(markdown: String): String =
    markdown.replace(MARKDOWN_LINK, "$1").replace(MARKDOWN_MARKS, "").lines().map { it.trimEnd() }
        .dropWhile { it.isBlank() }.dropLastWhile { it.isBlank() }.joinToString("\n").replace(Regex("\n{3,}"), "\n\n")

internal fun checkedAtMillis(checkedAt: String): Long? = runCatching { Instant.parse(checkedAt).toEpochMilli() }.getOrNull()

@Composable
internal fun watchedAppUpdate(): AppUpdateReading? = qgcDerived(APP_UPDATE_VIEW, ::appUpdateReading).value

private fun openUrl(context: Context, url: String) =
    context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))

@Composable
private fun rememberInstall(file: String): () -> Unit {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val install = { scope.launch { withContext(Dispatchers.IO) { AppInstall.start(context, File(file)) } } }
    val allowed = { context.packageManager.canRequestPackageInstalls() }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { if (allowed()) install() }
    return {
        if (allowed()) install() else permission.launch(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${context.packageName}")))
    }
}

@Composable
internal fun UpdatesSection() {
    val context = LocalContext.current
    val fromPlay = remember { installedFromPlay(installerOf(context)) }
    val reading = watchedAppUpdate() ?: return
    val installError by AppInstall.outcome.collectAsState()
    var refusal by remember { mutableStateOf<String?>(null) }
    val install = rememberInstall(reading.file)
    val act: (String) -> Unit = { path -> offMainDetached { refusal = Qgc.refusalOf(path, subject = UPDATE_SUBJECT) } }

    SectionHeader("Updates")
    ListItem(
        headlineContent = { Text("Aircast version") },
        supportingContent = { Text("${reading.current} · ${reading.channel} channel") },
        trailingContent = {
            if (!fromPlay) TextButton(onClick = { refusal = null; act("appUpdate.check") }, enabled = reading.phase !in setOf(UpdatePhase.Checking, UpdatePhase.Downloading)) { Text("Check now") }
        },
    )
    if (fromPlay) {
        ListItem(headlineContent = { Text(PLAY_UPDATES) })
        return
    }
    ListItem(
        headlineContent = { Text("Check for updates automatically") },
        trailingContent = { Switch(checked = reading.autoCheck, onCheckedChange = { on -> offMainDetached { Qgc.invoke("appUpdate.setAutoCheck", on) } }) },
    )
    val status = refusal ?: updateStatus(reading, fromPlay, installError)
    if (status.isNotBlank()) {
        ListItem(
            headlineContent = { Text(status, color = if (refusal != null || installError != null || reading.phase == UpdatePhase.Failed) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface) },
            supportingContent = { UpdateDetail(reading) },
        )
    }
    if (reading.phase == UpdatePhase.Downloading) {
        LinearProgressIndicator(progress = { reading.progress }, modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp))
    }
    if (reading.phase in setOf(UpdatePhase.Available, UpdatePhase.Ready) && reading.notes.isNotBlank()) {
        Text(
            plainNotes(reading.notes),
            Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            maxLines = NOTES_MAX_LINES,
            overflow = TextOverflow.Ellipsis,
        )
    }
    Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp), horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.CenterVertically) {
        if (reading.releaseUrl.isNotBlank() && reading.phase != UpdatePhase.UpToDate) TextButton(onClick = { openUrl(context, reading.releaseUrl) }) { Text("Release notes") }
        when (reading.phase) {
            UpdatePhase.Available -> Button(onClick = { refusal = null; act("appUpdate.download") }) {
                Text(if (reading.sizeBytes > 0) "Download (${Formatter.formatShortFileSize(context, reading.sizeBytes)})" else "Download")
            }
            UpdatePhase.Downloading -> TextButton(onClick = { act("appUpdate.cancel") }) { Text("Cancel") }
            UpdatePhase.Ready -> Button(onClick = install, enabled = reading.canInstall) { Text("Install") }
            UpdatePhase.Failed -> {
                if (reading.assetUrl.isNotBlank()) TextButton(onClick = { openUrl(context, reading.assetUrl) }) { Text("Download in browser") }
                TextButton(onClick = { refusal = null; act("appUpdate.check") }) { Text("Try again") }
            }
            else -> Unit
        }
    }
}

@Composable
private fun UpdateDetail(reading: AppUpdateReading) {
    val context = LocalContext.current
    val lines = listOfNotNull(
        installRefusal(reading).takeIf { reading.phase == UpdatePhase.Ready },
        reading.assetName.takeIf { reading.phase == UpdatePhase.Ready && it.isNotBlank() }
            ?.let { name -> if (reading.sizeBytes > 0) "$name · ${Formatter.formatShortFileSize(context, reading.sizeBytes)}" else name },
        checkedAtMillis(reading.checkedAt)?.let { "Last checked ${DateUtils.getRelativeTimeSpanString(it)}" },
    )
    if (lines.isNotEmpty()) Text(lines.joinToString("\n"))
}

private val UPDATE_BANNER_MAX_WIDTH = 560.dp
private val UPDATE_BANNER_CORNER = 12.dp

@Composable
internal fun UpdateBanner(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val fromPlay = remember { installedFromPlay(installerOf(context)) }
    val reading = watchedAppUpdate()?.takeIf { bannerShown(it, fromPlay) } ?: return
    val installError by AppInstall.outcome.collectAsState()
    val install = rememberInstall(reading.file)
    val act: (String) -> Unit = { path -> offMainDetached { Qgc.invoke(path) } }
    Surface(
        modifier = modifier.widthIn(max = UPDATE_BANNER_MAX_WIDTH).fillMaxWidth(),
        shape = RoundedCornerShape(UPDATE_BANNER_CORNER),
        color = MaterialTheme.colorScheme.secondaryContainer,
        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
    ) {
        Row(
            Modifier.heightIn(min = 48.dp).padding(start = 12.dp, end = 4.dp, top = 4.dp, bottom = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Icon(painterResource(R.drawable.ic_download), contentDescription = null, modifier = Modifier.size(24.dp))
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(updateStatus(reading, fromPlay, installError), style = MaterialTheme.typography.titleSmall, maxLines = 2, overflow = TextOverflow.Ellipsis)
                if (reading.phase == UpdatePhase.Downloading) LinearProgressIndicator(progress = { reading.progress }, modifier = Modifier.fillMaxWidth())
            }
            when (reading.phase) {
                UpdatePhase.Downloading -> TextButton(onClick = { act("appUpdate.cancel") }) { Text("Cancel") }
                else -> {
                    TextButton(onClick = { act("appUpdate.snooze") }) { Text("Later") }
                    if (reading.phase == UpdatePhase.Ready) Button(onClick = install) { Text("Install") } else Button(onClick = { act("appUpdate.download") }) { Text("Update") }
                }
            }
        }
    }
}
