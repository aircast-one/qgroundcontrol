package one.aircast.android.ui

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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

private const val ANALYZE_VIEW_QML = "qrc:/qml/QGroundControl/AnalyzeView/AnalyzeView.qml"

enum class AnalyzePage(
    val label: String,
    val description: String,
    val qml: String,
) {
    LogDownload(
        "Log Download",
        "Download flight logs from the vehicle",
        "",
    ),
    Vibration(
        "Vibration",
        "Accelerometer vibration levels and clipping",
        "",
    ),
    MoreTools(
        "More analysis tools",
        "MAVLink console and inspector",
        ANALYZE_VIEW_QML,
    ),
    ;

    val isNative get() = qml.isEmpty()
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
    LazyColumn(modifier.fillMaxSize()) {
        items(AnalyzePage.entries, key = { it.name }) { page ->
            ListItem(
                headlineContent = { Text(page.label) },
                supportingContent = { Text(page.description) },
                modifier = Modifier.clickable { onSelect(page) },
            )
            HorizontalDivider()
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

    if (!page.isNative) {
        return
    }

    Column(modifier.fillMaxSize()) {
        AnalyzeHeader(page.label) { onSelect(null) }
        Surface(Modifier.weight(1f)) {
            when (page) {
                AnalyzePage.LogDownload -> LogDownloadScreen()
                else -> VibrationScreen()
            }
        }
    }
}
