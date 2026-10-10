package one.aircast.android

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.hardware.usb.UsbDeviceConnection
import android.hardware.usb.UsbManager
import android.os.Build
import android.os.SystemClock
import java.util.concurrent.ConcurrentHashMap

object WfbUsb {
    private const val PERMISSION_ACTION = "one.aircast.android.WFB_USB_PERMISSION"
    private const val PERMISSION_PENDING = -2
    private const val UNAVAILABLE = -1
    private const val ASK_AGAIN_AFTER_MS = 30_000L
    private var context: Context? = null
    private val connections = ConcurrentHashMap<String, UsbDeviceConnection>()
    private val asked = ConcurrentHashMap<String, Long>()

    fun initialize(context: Context) {
        if (this.context == null) this.context = context.applicationContext
    }

    private fun manager(): UsbManager? = context?.getSystemService(UsbManager::class.java)

    @JvmStatic
    fun devices(): Array<String> {
        val attached = manager()?.deviceList?.values.orEmpty()
        asked.keys.retainAll(attached.map { it.deviceName }.toSet())
        return attached.map { device ->
            val product = runCatching { device.productName }.getOrNull().orEmpty().replace('\t', ' ')
            "${device.deviceName}\t${device.vendorId}\t${device.productId}\t$product"
        }.toTypedArray()
    }

    @JvmStatic
    fun open(deviceName: String): Int {
        val manager = manager() ?: return UNAVAILABLE
        val device = manager.deviceList[deviceName] ?: return UNAVAILABLE
        if (!manager.hasPermission(device)) {
            val now = SystemClock.elapsedRealtime()
            val last = asked[deviceName]
            if (last == null || now - last > ASK_AGAIN_AFTER_MS) {
                val app = context ?: return UNAVAILABLE
                asked[deviceName] = now
                val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0
                val intent = Intent(PERMISSION_ACTION).setPackage(app.packageName)
                manager.requestPermission(device, PendingIntent.getBroadcast(app, 0, intent, flags))
            }
            return PERMISSION_PENDING
        }
        connections.remove(deviceName)?.close()
        val connection = manager.openDevice(device) ?: return UNAVAILABLE
        connections[deviceName] = connection
        return connection.fileDescriptor
    }

    @JvmStatic
    fun close(deviceName: String) {
        connections.remove(deviceName)?.close()
    }
}
