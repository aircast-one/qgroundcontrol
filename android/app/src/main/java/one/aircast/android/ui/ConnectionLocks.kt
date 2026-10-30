package one.aircast.android.ui

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.view.WindowManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.ContextCompat
import one.aircast.android.bridge.qgcPath
import one.aircast.map.VEHICLES_VIEW
import org.json.JSONObject

internal fun vehicleConnected(view: JSONObject?): Boolean = (view?.optInt("count") ?: 0) > 0

internal fun linkAwaitingVehicle(view: JSONObject?): Boolean = linkRows(view).any { it.connected }

private fun notificationsBlocked(context: Context): Boolean =
    Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU && ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED

@Composable
fun ConnectionLocks() {
    val vehicles by qgcPath(VEHICLES_VIEW)
    val links by qgcPath(LINKS_VIEW)
    val connected = vehicleConnected(vehicles)
    val downloading = watchedAppUpdate()?.phase == UpdatePhase.Downloading
    val awake = connected || downloading
    val linked = connected || linkAwaitingVehicle(links)
    val context = LocalContext.current
    val askToNotify = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { }
    LaunchedEffect(linked) {
        if (linked && notificationsBlocked(context)) askToNotify.launch(Manifest.permission.POST_NOTIFICATIONS)
    }
    DisposableEffect(awake) {
        if (!awake) return@DisposableEffect onDispose { }
        val window = (context as? Activity)?.window
        window?.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        onDispose { window?.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON) }
    }
}
