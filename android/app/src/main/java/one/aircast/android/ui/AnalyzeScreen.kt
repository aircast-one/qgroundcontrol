package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.annotation.DrawableRes
import one.aircast.android.R

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath

enum class AnalyzeSection(val title: String) { FlightData("Flight data"), Live("Live"), Vehicle("Vehicle") }

enum class AnalyzePage(
    val label: String,
    val description: String,
    val section: AnalyzeSection,
    @DrawableRes val icon: Int,
) {
    LogDownload(
        "Flight logs",
        "Download flight logs from the vehicle",
        AnalyzeSection.FlightData,
        R.drawable.ic_download,
    ),
    GeoTag(
        "Geotag images",
        "Match photographs to where the vehicle was when it took them",
        AnalyzeSection.FlightData,
        R.drawable.ic_photo_camera,
    ),
    Vibration(
        "Vibration",
        "Accelerometer vibration levels and clipping",
        AnalyzeSection.Live,
        R.drawable.ic_vibration,
    ),
    Inspector(
        "MAVLink inspector",
        "Live message rates and field values",
        AnalyzeSection.Live,
        R.drawable.ic_analytics,
    ),
    Console(
        "Console",
        "Shell over the vehicle link",
        AnalyzeSection.Live,
        R.drawable.ic_terminal,
    ),
    Messages(
        "Messages",
        "What the vehicle has said since it connected",
        AnalyzeSection.Live,
        R.drawable.ic_description,
    ),
    Firmware(
        "Firmware",
        "Flash a board through its bootloader over USB",
        AnalyzeSection.Vehicle,
        R.drawable.ic_developer_board,
    ),
    ;
}

internal fun analyzeStatus(page: AnalyzePage, unread: Int, vibration: String?): Pair<String, SetupState> = when {
    page == AnalyzePage.Messages && unread > 0 -> "$unread new" to SetupState.NeedsAttention
    page == AnalyzePage.Vibration && vibration == "danger" -> "High" to SetupState.NeedsAttention
    page == AnalyzePage.Vibration && vibration == "warning" -> "Caution" to SetupState.NeedsAttention
    page == AnalyzePage.Vibration && vibration == "normal" -> "OK" to SetupState.Done
    else -> "" to SetupState.Neutral
}

internal fun analyzeSubtitle(page: AnalyzePage, messages: List<VehicleMessage>, vibration: VibrationReading? = null): String = when {
    page == AnalyzePage.Messages && messages.isNotEmpty() -> severitySummary(messages)
    page == AnalyzePage.Vibration -> vibration?.let(::vibrationGlance) ?: page.description
    else -> page.description
}

internal fun analyzeNote(
    page: AnalyzePage,
    connected: Boolean,
    px4: Boolean,
    vibration: String?,
): String? = when {
    !connected -> null
    page == AnalyzePage.Console && !px4 -> "The shell answers on PX4; this vehicle reports another autopilot"
    page == AnalyzePage.Vibration -> vibration
    else -> null
}

@Composable
private fun AnalyzePageList(onSelect: (AnalyzePage) -> Unit, modifier: Modifier = Modifier, selected: AnalyzePage? = null) {
    val setupJson by qgcPath(SETUP)
    val vibrationJson by qgcPath(VIBRATION_VIEW)
    val connected = hasVehicle()
    val px4 = remember(setupJson) { isPx4(setupReadiness(setupJson)) }
    val caveat = remember(vibrationJson) { vibrationCaveat(vibrationJson) }
    val messagesJson by qgcPath(MESSAGES)
    val vibration = remember(vibrationJson) { vibrationReading(vibrationJson) }
    val vibrationLevel = vibration?.let(::worstSeverity)
    val unread = remember(messagesJson) { unreadCount(messagesJson) }
    val messages = remember(messagesJson) { vehicleMessages(messagesJson) }

    LazyColumn(modifier.fillMaxSize()) {
        item(key = "title") {
            Column(Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 8.dp)) {
                Text("Analyze", style = MaterialTheme.typography.headlineMedium)
                Text(
                    "Logs and tools for the connected vehicle",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        AnalyzePage.entries.groupBy { it.section }.forEach { (section, pages) ->
            item(key = section.name) { SectionHeader(section.title) }
            items(pages, key = { it.name }) { page ->
                val (status, state) = analyzeStatus(page, unread, vibrationLevel)
                SetupRow(
                    title = page.label,
                    status = status,
                    state = state,
                    subtitle = listOfNotNull(analyzeSubtitle(page, messages, vibration), analyzeNote(page, connected, px4, caveat)).joinToString("\n"),
                    onClick = { onSelect(page) },
                    icon = page.icon,
                    selected = page == selected,
                )
            }
        }
    }
}

@Composable
fun AnalyzeScreen(
    page: AnalyzePage?,
    onSelect: (AnalyzePage?) -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val switchTo: (AnalyzePage?) -> Unit = { next ->
        navigationRefusal(AppNavigation.blockedReason, leaving = true)
            ?.takeIf { page != null }
            ?.let { android.widget.Toast.makeText(context, it, android.widget.Toast.LENGTH_SHORT).show() }
            ?: onSelect(next)
    }
    val leave: () -> Unit = { switchTo(null) }
    BackHandler(enabled = page != null) { leave() }

    BoxWithConstraints(modifier.fillMaxSize()) {
        val wide = maxWidth >= LIST_DETAIL_MIN_WIDTH
        when {
            wide -> Row(Modifier.fillMaxSize()) {
                AnalyzePageList(onSelect = switchTo, modifier = Modifier.width(LIST_PANE_WIDTH).background(MaterialTheme.colorScheme.surfaceContainerLow), selected = page)
                Surface(Modifier.weight(1f).fillMaxHeight()) {
                    if (page == null) EmptyState(R.drawable.ic_analytics, "Analyze", "Choose a tool on the left.") else AnalyzePageBody(page, leave)
                }
            }
            page == null -> Surface(Modifier.fillMaxSize()) { AnalyzePageList(onSelect = onSelect) }
            else -> AnalyzePageBody(page, leave)
        }
    }
}

@Composable
private fun AnalyzePageBody(page: AnalyzePage, leave: () -> Unit) {
    Column(Modifier.fillMaxSize()) {
        PageTopBar(page.label, "Back to Analyze") { leave() }
        Surface(Modifier.weight(1f)) {
            when (page) {
                AnalyzePage.LogDownload -> LogDownloadScreen()
                AnalyzePage.Console -> ConsoleScreen()
                AnalyzePage.Inspector -> InspectorScreen()
                AnalyzePage.Vibration -> VibrationScreen()
                AnalyzePage.GeoTag -> GeoTagScreen()
                AnalyzePage.Firmware -> FirmwareScreen()
                AnalyzePage.Messages -> VehicleMessagesPage()
            }
        }
    }
}
