package one.aircast.android.ui

import androidx.annotation.DrawableRes
import one.aircast.android.R

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
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
        "Log Download",
        "Download flight logs from the vehicle",
        AnalyzeSection.FlightData,
        R.drawable.ic_download,
    ),
    GeoTag(
        "GeoTag Images",
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
        "MAVLink Inspector",
        "Live message rates and field values",
        AnalyzeSection.Live,
        R.drawable.ic_analytics,
    ),
    Console(
        "MAVLink Console",
        "Shell over the vehicle link",
        AnalyzeSection.Live,
        R.drawable.ic_terminal,
    ),
    Firmware(
        "Firmware",
        "Flash a board through its bootloader over USB",
        AnalyzeSection.Vehicle,
        R.drawable.ic_developer_board,
    ),
    ;
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
private fun AnalyzeHeader(title: String, onBack: () -> Unit) {
    Surface(color = MaterialTheme.colorScheme.surface) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 4.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onBack) {
                Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back to Analyze")
            }
            Text(text = title, style = MaterialTheme.typography.titleMedium)
        }
    }
}

@Composable
private fun AnalyzePageList(onSelect: (AnalyzePage) -> Unit, modifier: Modifier = Modifier) {
    val setupJson by qgcPath(SETUP)
    val vibrationJson by qgcPath(VIBRATION_VIEW)
    val connected = hasVehicle()
    val px4 = remember(setupJson) { isPx4(setupReadiness(setupJson)) }
    val caveat = remember(vibrationJson) { vibrationCaveat(vibrationJson) }

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
                SetupRow(
                    title = page.label,
                    subtitle = listOfNotNull(page.description, analyzeNote(page, connected, px4, caveat)).joinToString("\n"),
                    onClick = { onSelect(page) },
                    icon = page.icon,
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
    BackHandler(enabled = page != null) { onSelect(null) }

    if (page == null) {
        Surface(modifier.fillMaxSize()) { AnalyzePageList(onSelect = onSelect) }
        return
    }

    Column(modifier.fillMaxSize()) {
        AnalyzeHeader(page.label) { onSelect(null) }
        Surface(Modifier.weight(1f)) {
            when (page) {
                AnalyzePage.LogDownload -> LogDownloadScreen()
                AnalyzePage.Console -> ConsoleScreen()
                AnalyzePage.Inspector -> InspectorScreen()
                AnalyzePage.Vibration -> VibrationScreen()
                AnalyzePage.GeoTag -> GeoTagScreen()
                AnalyzePage.Firmware -> FirmwareScreen()
            }
        }
    }
}
