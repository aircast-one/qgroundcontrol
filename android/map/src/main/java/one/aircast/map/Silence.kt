package one.aircast.map

import android.os.SystemClock
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import kotlinx.coroutines.delay

private const val MILLIS_PER_SECOND = 1000L
private const val SECONDS_PER_MINUTE = 60

@Composable
fun silentSeconds(lost: Boolean): Long? {
    val since = remember(lost) { if (lost) SystemClock.elapsedRealtime() else null }
    val now by produceState(SystemClock.elapsedRealtime(), since) {
        while (since != null) {
            value = SystemClock.elapsedRealtime()
            delay(MILLIS_PER_SECOND)
        }
    }
    return since?.let { (now - it).coerceAtLeast(0) / MILLIS_PER_SECOND }
}

fun silenceDuration(seconds: Long): String =
    if (seconds < SECONDS_PER_MINUTE) "$seconds s" else "${seconds / SECONDS_PER_MINUTE} min"

fun lastSeenText(seconds: Long): String = "Last seen ${silenceDuration(seconds)} ago"
