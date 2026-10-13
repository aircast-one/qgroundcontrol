package one.aircast.android

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.wifi.WifiManager
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch
import one.aircast.android.bridge.Qgc
import one.aircast.android.ui.LINKS_VIEW
import one.aircast.android.ui.linkAwaitingVehicle
import one.aircast.android.ui.vehicleConnected
import one.aircast.map.VEHICLES_VIEW

private const val CHANNEL = "vehicle-link"
private const val CHANNEL_NAME = "Aircraft link"
private const val NOTIFICATION_ID = 1
private const val LOCK_TAG = "aircast:vehicle"
private const val LOG_TAG = "VehicleLink"

class VehicleLinkService : Service() {
    private val wake by lazy { getSystemService(PowerManager::class.java).newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, LOCK_TAG) }

    @Suppress("DEPRECATION")
    private val wifi by lazy { (applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager).createWifiLock(WifiManager.WIFI_MODE_FULL_HIGH_PERF, LOCK_TAG) }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int = START_NOT_STICKY

    override fun onCreate() {
        super.onCreate()
        running.set(true)
        getSystemService(NotificationManager::class.java).createNotificationChannel(NotificationChannel(CHANNEL, CHANNEL_NAME, NotificationManager.IMPORTANCE_LOW))
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP), PendingIntent.FLAG_IMMUTABLE)
        val notification = NotificationCompat.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_flight)
            .setContentTitle("Aircraft link open")
            .setContentText("Aircast keeps the link alive while the screen is off")
            .setContentIntent(open)
            .setOngoing(true)
            .build()
        ServiceCompat.startForeground(this, NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
        wake.acquire()
        wifi.acquire()
    }

    override fun onDestroy() {
        wake.takeIf { it.isHeld }?.release()
        wifi.takeIf { it.isHeld }?.release()
        running.set(false)
        super.onDestroy()
    }

    companion object {
        val running = AtomicBoolean(false)

        fun start(context: Context) {
            runCatching { ContextCompat.startForegroundService(context, Intent(context, VehicleLinkService::class.java)) }
                .onFailure { Log.w(LOG_TAG, "event=link_service_refused reason=\"${it.javaClass.simpleName}: ${it.message}\" retry=on_foreground") }
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, VehicleLinkService::class.java))
        }
    }
}

object VehicleLinkKeeper {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val started = AtomicBoolean(false)
    private val linked = AtomicBoolean(false)

    fun start(context: Context) {
        if (!started.compareAndSet(false, true)) return
        val app = context.applicationContext
        scope.launch {
            Qgc.watch(listOf(VEHICLES_VIEW, LINKS_VIEW))
            combine(Qgc.path(VEHICLES_VIEW), Qgc.path(LINKS_VIEW)) { vehicles, links -> vehicleConnected(vehicles) || linkAwaitingVehicle(links) }
                .distinctUntilChanged()
                .collect { now ->
                    linked.set(now)
                    if (now) VehicleLinkService.start(app) else VehicleLinkService.stop(app)
                }
        }
    }

    fun foregrounded(context: Context) {
        if (linked.get() && !VehicleLinkService.running.get()) VehicleLinkService.start(context.applicationContext)
    }
}
