package one.aircast.android.ui

import android.app.Activity
import android.content.Context
import android.net.wifi.WifiManager
import android.os.PowerManager
import android.view.WindowManager
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.LocalContext
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.VEHICLES_VIEW
import org.json.JSONObject

private const val LOCK_TAG = "aircast:vehicle"

internal fun vehicleConnected(view: JSONObject?): Boolean = (view?.optInt("count") ?: 0) > 0

@Composable
fun ConnectionLocks() {
    val vehicles by qgcPath(VEHICLES_VIEW)
    val connected = vehicleConnected(vehicles)
    val context = LocalContext.current
    DisposableEffect(connected) {
        if (!connected) return@DisposableEffect onDispose { }
        val window = (context as? Activity)?.window
        window?.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        val wake = (context.getSystemService(Context.POWER_SERVICE) as PowerManager)
            .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, LOCK_TAG)
            .apply { acquire() }
        @Suppress("DEPRECATION")
        val wifi = (context.applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager)
            .createWifiLock(WifiManager.WIFI_MODE_FULL_HIGH_PERF, LOCK_TAG)
            .apply { acquire() }
        onDispose {
            window?.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            wake.takeIf { it.isHeld }?.release()
            wifi.takeIf { it.isHeld }?.release()
        }
    }
}
