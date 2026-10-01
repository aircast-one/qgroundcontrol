package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc

internal const val VIDEO_STATS_VIEW = "view.videoStats"
private const val VIDEO_STATS_POLL_MS = 1000L

internal fun videoStatsText(view: org.json.JSONObject?): String = view?.optString("text").orEmpty()

@Composable
internal fun VideoStatsPill(modifier: Modifier) {
    var text by remember { mutableStateOf("") }
    LaunchedEffect(Unit) {
        while (isActive) {
            text = withContext(Dispatchers.Default) { videoStatsText(Qgc.get(VIDEO_STATS_VIEW)) }
            delay(VIDEO_STATS_POLL_MS)
        }
    }
    if (text.isBlank()) return
    Box(modifier) {
        Text(
            text,
            color = Color.White,
            fontWeight = FontWeight.Bold,
            style = MaterialTheme.typography.labelSmall,
            modifier = Modifier
                .align(Alignment.BottomStart)
                .padding(8.dp)
                .background(Color(0f, 0f, 0f, 0.6f), RoundedCornerShape(50))
                .padding(horizontal = 10.dp, vertical = 4.dp),
        )
    }
}
