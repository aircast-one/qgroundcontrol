package one.aircast.android

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Build
import android.os.Handler
import android.os.Looper

object GcsLocation {
    private const val SOURCE_TOKEN = "internalGps"
    private const val UPDATE_INTERVAL_MS = 1000L
    private const val PERMISSION_RETRY_MS = 2000L
    private const val PERMISSION_REQUEST = 4317
    private val PERMISSIONS = arrayOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION)

    private val handler = Handler(Looper.getMainLooper())
    private var manager: LocationManager? = null
    private var listening = false

    private val listener = LocationListener { location -> report(location) }

    private fun report(location: Location) {
        nativeUpdate(
            location.latitude,
            location.longitude,
            if (location.hasAltitude()) location.altitude else Double.NaN,
            if (location.hasAccuracy()) location.accuracy.toDouble() else Double.NaN,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && location.hasVerticalAccuracy()) location.verticalAccuracyMeters.toDouble() else Double.NaN,
            if (location.hasBearing()) location.bearing.toDouble() else Double.NaN,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && location.hasBearingAccuracy()) location.bearingAccuracyDegrees.toDouble() else Double.NaN,
        )
    }

    private fun granted(activity: Activity): Boolean =
        PERMISSIONS.any { activity.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED }

    private fun provider(manager: LocationManager): String? {
        val enabled = manager.getProviders(true)
        val fused = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) LocationManager.FUSED_PROVIDER else null
        return listOfNotNull(fused, LocationManager.GPS_PROVIDER, LocationManager.NETWORK_PROVIDER).firstOrNull { it in enabled }
    }

    @SuppressLint("MissingPermission")
    private fun listen(activity: Activity) {
        if (listening) return
        if (!granted(activity)) {
            handler.postDelayed({ listen(activity) }, PERMISSION_RETRY_MS)
            return
        }
        val locations = activity.getSystemService(LocationManager::class.java) ?: return
        val chosen = provider(locations) ?: run {
            handler.postDelayed({ listen(activity) }, PERMISSION_RETRY_MS)
            return
        }
        manager = locations
        listening = true
        nativeSource(SOURCE_TOKEN)
        locations.getLastKnownLocation(chosen)?.let(::report)
        locations.requestLocationUpdates(chosen, UPDATE_INTERVAL_MS, 0f, listener, Looper.getMainLooper())
    }

    fun start(activity: Activity) {
        if (!granted(activity)) activity.requestPermissions(PERMISSIONS, PERMISSION_REQUEST)
        listen(activity)
    }

    fun stop() {
        handler.removeCallbacksAndMessages(null)
        manager?.removeUpdates(listener)
        manager = null
        listening = false
    }

    @JvmStatic
    private external fun nativeSource(token: String)

    @JvmStatic
    private external fun nativeUpdate(
        latitude: Double,
        longitude: Double,
        altitude: Double,
        horizontalAccuracy: Double,
        verticalAccuracy: Double,
        direction: Double,
        directionAccuracy: Double,
    )
}
