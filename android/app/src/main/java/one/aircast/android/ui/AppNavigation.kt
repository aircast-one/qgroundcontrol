package one.aircast.android.ui

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf

internal class AppNavigationState {
    var settingsPage by mutableStateOf<String?>(null)
    var settingsOpen by mutableStateOf(false)
    var setupPage by mutableStateOf<String?>(null)
    var blockedReason by mutableStateOf<String?>(null)
}

internal class OpenPlanDocument {
    var document by mutableStateOf<Uri?>(null)
    var name by mutableStateOf<String?>(null)
}

internal val LocalAppNavigation = staticCompositionLocalOf<AppNavigationState> { error("AppNavigationState is provided by MainActivity") }

internal val LocalOpenPlan = staticCompositionLocalOf<OpenPlanDocument> { error("OpenPlanDocument is provided by MainActivity") }

internal const val LOG_DOWNLOAD_BLOCK = "Wait for the log download to complete or cancel it first"
internal const val CALIBRATION_BLOCK = "Complete or cancel the current calibration first"

internal fun navigationRefusal(blockedReason: String?, leaving: Boolean): String? = blockedReason?.takeIf { leaving }

@Composable
internal fun BlocksNavigation(blocking: Boolean, reason: String) {
    val navigation = LocalAppNavigation.current
    DisposableEffect(blocking) {
        if (blocking) navigation.blockedReason = reason
        onDispose { if (navigation.blockedReason == reason) navigation.blockedReason = null }
    }
}
