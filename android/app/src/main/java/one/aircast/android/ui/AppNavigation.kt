package one.aircast.android.ui

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

internal object AppNavigation {
    var settingsPage by mutableStateOf<String?>(null)
    var setupPage by mutableStateOf<String?>(null)
    var blockedReason by mutableStateOf<String?>(null)
}

internal const val LOG_DOWNLOAD_BLOCK = "Wait for the log download to complete or cancel it first"
internal const val CALIBRATION_BLOCK = "Complete or cancel the current calibration first"

internal fun navigationRefusal(blockedReason: String?, leaving: Boolean): String? = blockedReason?.takeIf { leaving }

@androidx.compose.runtime.Composable
internal fun BlocksNavigation(blocking: Boolean, reason: String) {
    androidx.compose.runtime.DisposableEffect(blocking) {
        if (blocking) AppNavigation.blockedReason = reason
        onDispose { if (AppNavigation.blockedReason == reason) AppNavigation.blockedReason = null }
    }
}
