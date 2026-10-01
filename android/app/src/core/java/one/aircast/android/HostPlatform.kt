package one.aircast.android

import android.app.Activity
import android.view.View
import java.io.File
import one.aircast.mapspike.MapTileHost
import org.mavlink.qgroundcontrol.QGCBridge
import org.mavlink.qgroundcontrol.QGCUsbSerialManager

private const val CORE_LIBRARY = "groundstation"
private const val ORGANIZATION = "Aircast"
private const val DEFAULT_APPLICATION = "Aircast QGC"
private const val DEBUG_API_PORT = "8777"

object HostPlatform {
    private var started = false

    fun start(activity: Activity): View? {
        if (started) return null
        started = true
        System.loadLibrary(CORE_LIBRARY)
        QGCUsbSerialManager.initialize(activity)
        BluetoothLinks.initialize(activity)
        val folder = File(activity.filesDir, "settings/$ORGANIZATION").apply { mkdirs() }
        val settings = folder.listFiles { file -> file.extension == "ini" }?.minByOrNull { it.name }
            ?: File(folder, "$DEFAULT_APPLICATION.ini")
        val application = settings.nameWithoutExtension
        val mapCache = File(activity.cacheDir, "QGCMapCache").apply { mkdirs() }
        val arguments = listOf(
            "--settings", settings.absolutePath,
            "--app-name", application,
            "--map-cache", File(mapCache, "qgcMapCache.db").absolutePath,
        ) + if (BuildConfig.DEBUG) listOf("--port", DEBUG_API_PORT) else emptyList()
        QGCBridge.start(arguments.toTypedArray())
        val tileCache = File(mapCache, "qgcMapCache.db").absolutePath
        MapTileHost.fetch = { mapType, x, y, zoom -> QGCBridge.mapTile(mapType, x, y, zoom, tileCache) }
        GcsLocation.start(activity)
        return null
    }

    fun stop(activity: Activity) {
        if (!activity.isFinishing) return
        GcsLocation.stop()
        QGCBridge.shutdown()
        QGCUsbSerialManager.cleanup(activity)
    }
}
