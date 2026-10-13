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
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat

private const val CHANNEL = "vehicle-link"
private const val CHANNEL_NAME = "Aircraft link"
private const val NOTIFICATION_ID = 1
private const val LOCK_TAG = "aircast:vehicle"

class VehicleLinkService : Service() {
    private val wake by lazy { getSystemService(PowerManager::class.java).newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, LOCK_TAG) }

    @Suppress("DEPRECATION")
    private val wifi by lazy { (applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager).createWifiLock(WifiManager.WIFI_MODE_FULL_HIGH_PERF, LOCK_TAG) }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        getSystemService(NotificationManager::class.java).createNotificationChannel(NotificationChannel(CHANNEL, CHANNEL_NAME, NotificationManager.IMPORTANCE_LOW))
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP), PendingIntent.FLAG_IMMUTABLE)
        val notification = NotificationCompat.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_flight)
            .setContentTitle("Connected to your aircraft")
            .setContentText("The link stays open while the screen is off")
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
        super.onDestroy()
    }

    companion object {
        fun start(context: Context) {
            runCatching { ContextCompat.startForegroundService(context, Intent(context, VehicleLinkService::class.java)) }
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, VehicleLinkService::class.java))
        }
    }
}
