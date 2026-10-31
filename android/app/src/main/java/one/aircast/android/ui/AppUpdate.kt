package one.aircast.android.ui

import android.content.Context
import android.content.Intent
import android.net.ConnectivityManager
import android.net.Uri
import android.provider.Settings
import android.text.format.DateUtils
import android.text.format.Formatter
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.annotation.DrawableRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
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
import androidx.compose.runtime.LaunchedEffect
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
import one.aircast.android.InstallFailure
import one.aircast.android.InstallOutcome
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
internal const val INSTALL_PRIMER = "Android will ask you to let Aircast install updates. Turn on Allow from this source, then press Back."
internal const val RELEASE_TAG_URL = "https://github.com/aircast-one/qgroundcontrol/releases/tag/aircast-v"
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
    val skipped: String,
    val later: Boolean,
    val updatedFrom: String,
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
        skipped = json.optText("skipped"),
        later = json.optBoolean("later", false),
        updatedFrom = json.optText("updatedFrom"),
        canInstall = json.optBoolean("canInstall"),
    )
}

internal fun updateStatus(reading: AppUpdateReading, fromPlay: Boolean, failure: InstallFailure? = null): String = when {
    fromPlay -> PLAY_UPDATES
    reading.phase == UpdatePhase.Checking -> "Checking for updates…"
    reading.phase == UpdatePhase.UpToDate -> "Aircast is up to date"
    reading.phase == UpdatePhase.Available -> "Aircast ${reading.latest} is available"
    reading.phase == UpdatePhase.Downloading -> "Downloading Aircast ${reading.latest}… ${(reading.progress * 100).roundToInt()}%"
    reading.phase == UpdatePhase.Ready -> failure?.text ?: "Aircast ${reading.latest} is ready to install"
    reading.phase == UpdatePhase.Failed -> reading.error.ifBlank { "The update check failed." }
    else -> ""
}

internal fun skippedThis(reading: AppUpdateReading): Boolean = reading.skipped.isNotEmpty() && reading.skipped == reading.latest

internal fun bannerShown(reading: AppUpdateReading, fromPlay: Boolean): Boolean =
    !fromPlay && reading.canInstall && !reading.later && !skippedThis(reading) &&
        reading.phase in setOf(UpdatePhase.Available, UpdatePhase.Downloading, UpdatePhase.Ready)

internal fun updatedShown(reading: AppUpdateReading): Boolean = reading.updatedFrom.isNotEmpty() && reading.canInstall

internal fun releaseTagUrl(version: String): String = "$RELEASE_TAG_URL$version"

internal fun installRefusal(reading: AppUpdateReading): String? = DISCONNECT_FIRST.takeUnless { reading.canInstall }

internal fun installFailureFor(outcome: InstallOutcome?, reading: AppUpdateReading): InstallFailure? =
    outcome?.takeIf { it.apk == reading.file && reading.phase == UpdatePhase.Ready && reading.canInstall }?.failure

internal fun sizedLabel(label: String, size: String?): String = size?.let { "$label ($it)" } ?: label

internal fun meteredQuestion(size: String?): String =
    size?.let { "This update is $it and you're on a metered connection. Download now?" } ?: "You're on a metered connection. Download the update now?"

internal enum class ReadyAction { Install, OpenReleasePage, None }

internal fun readyAction(failure: InstallFailure?): ReadyAction = when (failure) {
    InstallFailure.DifferentBuild -> ReadyAction.OpenReleasePage
    InstallFailure.Downgrade -> ReadyAction.None
    else -> ReadyAction.Install
}

internal fun installLabel(failure: InstallFailure?): String = if (failure == null) "Install" else "Try again"

private val MARKDOWN_LINK = Regex("""\[([^\]]+)]\([^)]*\)""")
private val MARKDOWN_MARKS = Regex("""(?m)^[ \t]{0,3}(#{1,6}[ \t]+|[-*+][ \t]+|>[ \t]*)|[*_`]+""")

internal fun plainNotes(markdown: String): String =
    markdown.replace(MARKDOWN_LINK, "$1").replace(MARKDOWN_MARKS, "").lines().map { it.trimEnd() }
        .dropWhile { it.isBlank() }.dropLastWhile { it.isBlank() }.joinToString("\n").replace(Regex("\n{3,}"), "\n\n")

internal fun checkedAtMillis(checkedAt: String): Long? = runCatching { Instant.parse(checkedAt).toEpochMilli() }.getOrNull()

@Composable
internal fun watchedAppUpdate(): AppUpdateReading? = qgcDerived(APP_UPDATE_VIEW, ::appUpdateReading).value

@Composable
private fun rememberInstallFailure(reading: AppUpdateReading): InstallFailure? {
    val outcome by AppInstall.outcome.collectAsState()
    LaunchedEffect(reading.phase, reading.file, reading.canInstall) {
        AppInstall.forget { installFailureFor(it, reading) != null }
    }
    return installFailureFor(outcome, reading)
}

private fun openUrl(context: Context, url: String) =
    context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))

private fun sizeText(context: Context, bytes: Long): String? = bytes.takeIf { it > 0 }?.let { Formatter.formatShortFileSize(context, it) }

private fun onMeteredNetwork(context: Context): Boolean = context.getSystemService(ConnectivityManager::class.java)?.isActiveNetworkMetered == true

@Composable
private fun rememberInstall(file: String): () -> Unit {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var priming by remember { mutableStateOf(false) }
    val install = { scope.launch { withContext(Dispatchers.IO) { AppInstall.start(context, File(file)) } } }
    val allowed = { context.packageManager.canRequestPackageInstalls() }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { if (allowed()) install() }
    val prime = { priming = true }
    if (priming) {
        AlertDialog(
            onDismissRequest = { priming = false },
            text = { Text(INSTALL_PRIMER) },
            confirmButton = {
                TextButton(onClick = {
                    priming = false
                    permission.launch(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${context.packageName}")))
                }) { Text("Continue") }
            },
            dismissButton = { TextButton(onClick = { priming = false }) { Text("Cancel") } },
        )
    }
    return { if (allowed()) install() else prime() }
}

@Composable
private fun rememberDownload(reading: AppUpdateReading, act: (String) -> Unit): () -> Unit {
    val context = LocalContext.current
    var asking by remember { mutableStateOf(false) }
    val download = { act("appUpdate.download") }
    val ask = { asking = true }
    if (asking) {
        AlertDialog(
            onDismissRequest = { asking = false },
            text = { Text(meteredQuestion(sizeText(context, reading.sizeBytes))) },
            confirmButton = { TextButton(onClick = { asking = false; download() }) { Text("Download") } },
            dismissButton = { TextButton(onClick = { asking = false }) { Text("Not now") } },
        )
    }
    return { if (onMeteredNetwork(context)) ask() else download() }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun UpdatesSection() {
    val context = LocalContext.current
    val fromPlay = remember { installedFromPlay(installerOf(context)) }
    val reading = watchedAppUpdate() ?: return
    val failure = rememberInstallFailure(reading)
    var refusal by remember { mutableStateOf<String?>(null) }
    val act: (String) -> Unit = { path -> refusal = null; offMainDetached { refusal = Qgc.refusalOf(path, subject = UPDATE_SUBJECT) } }
    val install = rememberInstall(reading.file)
    val download = rememberDownload(reading, act)
    val size = sizeText(context, reading.sizeBytes)

    SectionHeader("Updates")
    ListItem(
        headlineContent = { Text("Aircast version") },
        supportingContent = { Text("${reading.current} · ${reading.channel} channel") },
        trailingContent = {
            if (!fromPlay) TextButton(onClick = { act("appUpdate.check") }, enabled = reading.phase !in setOf(UpdatePhase.Checking, UpdatePhase.Downloading)) { Text("Check now") }
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
    val status = refusal ?: updateStatus(reading, fromPlay, failure)
    if (status.isNotBlank()) {
        ListItem(
            headlineContent = { Text(status, color = if (refusal != null || failure != null || reading.phase == UpdatePhase.Failed) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface) },
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
    val offersRelease = reading.phase == UpdatePhase.Ready && readyAction(failure) == ReadyAction.OpenReleasePage
    FlowRow(
        Modifier.fillMaxWidth().padding(horizontal = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
        verticalArrangement = Arrangement.Center,
    ) {
        if (reading.releaseUrl.isNotBlank() && reading.phase != UpdatePhase.UpToDate && !offersRelease) TextButton(onClick = { openUrl(context, reading.releaseUrl) }) { Text("Release notes") }
        if (reading.phase in setOf(UpdatePhase.Available, UpdatePhase.Ready) && !skippedThis(reading)) TextButton(onClick = { act("appUpdate.skip") }) { Text("Skip this version") }
        when (reading.phase) {
            UpdatePhase.Available -> Button(onClick = download) { Text(sizedLabel("Download", size)) }
            UpdatePhase.Downloading -> TextButton(onClick = { act("appUpdate.cancel") }) { Text("Cancel") }
            UpdatePhase.Ready -> ReadyButton(reading, failure, install)
            UpdatePhase.Failed -> {
                if (reading.assetUrl.isNotBlank()) TextButton(onClick = { openUrl(context, reading.assetUrl) }) { Text("Download in browser") }
                TextButton(onClick = { act("appUpdate.check") }) { Text("Try again") }
            }
            else -> Unit
        }
    }
}

@Composable
private fun ReadyButton(reading: AppUpdateReading, failure: InstallFailure?, install: () -> Unit) {
    val context = LocalContext.current
    when (readyAction(failure)) {
        ReadyAction.OpenReleasePage -> Button(onClick = { openUrl(context, reading.releaseUrl.ifBlank { releaseTagUrl(reading.latest) }) }) { Text("Open release page") }
        ReadyAction.Install -> Button(onClick = install, enabled = reading.canInstall) { Text(installLabel(failure)) }
        ReadyAction.None -> Unit
    }
}

@Composable
private fun UpdateDetail(reading: AppUpdateReading) {
    val context = LocalContext.current
    val lines = listOfNotNull(
        installRefusal(reading).takeIf { reading.phase == UpdatePhase.Ready },
        reading.assetName.takeIf { reading.phase == UpdatePhase.Ready && it.isNotBlank() }
            ?.let { name -> sizeText(context, reading.sizeBytes)?.let { "$name · $it" } ?: name },
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
    val reading = watchedAppUpdate() ?: return
    val failure = rememberInstallFailure(reading)
    if (updatedShown(reading)) UpdatedBanner(reading, modifier)
    if (bannerShown(reading, fromPlay)) OfferBanner(reading, failure, fromPlay, modifier)
}

@Composable
private fun UpdatedBanner(reading: AppUpdateReading, modifier: Modifier) {
    val context = LocalContext.current
    BannerCard(R.drawable.ic_check_circle, "Updated to Aircast ${reading.current}", modifier) {
        TextButton(onClick = { offMainDetached { Qgc.invoke("appUpdate.dismissUpdated") } }) { Text("Dismiss") }
        Button(onClick = { openUrl(context, releaseTagUrl(reading.current)) }) { Text("What's new") }
    }
}

@Composable
private fun OfferBanner(reading: AppUpdateReading, failure: InstallFailure?, fromPlay: Boolean, modifier: Modifier) {
    val context = LocalContext.current
    val act: (String) -> Unit = { path -> offMainDetached { Qgc.invoke(path) } }
    val install = rememberInstall(reading.file)
    val download = rememberDownload(reading, act)
    BannerCard(R.drawable.ic_download, updateStatus(reading, fromPlay, failure), modifier, progress = reading.progress.takeIf { reading.phase == UpdatePhase.Downloading }) {
        when (reading.phase) {
            UpdatePhase.Downloading -> TextButton(onClick = { act("appUpdate.cancel") }) { Text("Cancel") }
            UpdatePhase.Ready -> {
                TextButton(onClick = { act("appUpdate.later") }) { Text("Later") }
                ReadyButton(reading, failure, install)
            }
            else -> {
                TextButton(onClick = { act("appUpdate.later") }) { Text("Later") }
                Button(onClick = download) { Text(sizedLabel("Update", sizeText(context, reading.sizeBytes))) }
            }
        }
    }
}

@Composable
private fun BannerCard(@DrawableRes icon: Int, text: String, modifier: Modifier, progress: Float? = null, actions: @Composable RowScope.() -> Unit) {
    Surface(
        modifier = modifier.widthIn(max = UPDATE_BANNER_MAX_WIDTH).fillMaxWidth(),
        shape = RoundedCornerShape(UPDATE_BANNER_CORNER),
        color = MaterialTheme.colorScheme.secondaryContainer,
        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
    ) {
        Column(Modifier.padding(start = 12.dp, end = 8.dp, top = 12.dp, bottom = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Icon(painterResource(icon), contentDescription = null, modifier = Modifier.size(24.dp))
                Text(text, style = MaterialTheme.typography.titleSmall)
            }
            progress?.let { LinearProgressIndicator(progress = { it }, modifier = Modifier.fillMaxWidth().padding(start = 36.dp)) }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End), verticalAlignment = Alignment.CenterVertically, content = actions)
        }
    }
}
