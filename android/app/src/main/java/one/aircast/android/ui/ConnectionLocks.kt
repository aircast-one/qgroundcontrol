package one.aircast.android.ui

import android.app.Activity
import android.view.WindowManager
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.LocalContext
import one.aircast.android.VehicleLinkService
import one.aircast.android.bridge.qgcPath
import one.aircast.map.VEHICLES_VIEW
import org.json.JSONObject

internal fun vehicleConnected(view: JSONObject?): Boolean = (view?.optInt("count") ?: 0) > 0

internal fun linkAwaitingVehicle(view: JSONObject?): Boolean = linkRows(view).any { it.connected }

@Composable
fun ConnectionLocks() {
    val vehicles by qgcPath(VEHICLES_VIEW)
    val links by qgcPath(LINKS_VIEW)
    val connected = vehicleConnected(vehicles)
    val linked = connected || linkAwaitingVehicle(links)
    val context = LocalContext.current
    DisposableEffect(connected) {
        if (!connected) return@DisposableEffect onDispose { }
        val window = (context as? Activity)?.window
        window?.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        onDispose { window?.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON) }
    }
    DisposableEffect(linked) {
        if (!linked) return@DisposableEffect onDispose { }
        VehicleLinkService.start(context)
        onDispose { VehicleLinkService.stop(context) }
    }
}
