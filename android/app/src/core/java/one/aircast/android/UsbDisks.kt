package one.aircast.android

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.hardware.usb.UsbConstants
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbDeviceConnection
import android.hardware.usb.UsbManager
import android.os.Build
import android.os.SystemClock
import java.util.concurrent.ConcurrentHashMap

object UsbDisks {
    private const val PERMISSION_ACTION = "one.aircast.android.USB_DISK_PERMISSION"
    private const val PERMISSION_PENDING = -2
    private const val UNAVAILABLE = -1
    private const val SUBCLASS_SCSI = 0x06
    private const val PROTOCOL_BULK_ONLY = 0x50
    private const val ASK_AGAIN_AFTER_MS = 30_000L
    private var context: Context? = null
    private val connections = ConcurrentHashMap<String, UsbDeviceConnection>()
    private val asked = ConcurrentHashMap<String, Long>()

    fun initialize(context: Context) {
        if (this.context == null) this.context = context.applicationContext
    }

    private fun manager(): UsbManager? = context?.getSystemService(UsbManager::class.java)

    internal fun isCardReader(classes: List<Triple<Int, Int, Int>>): Boolean =
        classes.any { (cls, subclass, protocol) -> cls == UsbConstants.USB_CLASS_MASS_STORAGE && subclass == SUBCLASS_SCSI && protocol == PROTOCOL_BULK_ONLY }

    private fun interfaces(device: UsbDevice): List<Triple<Int, Int, Int>> =
        (0 until device.interfaceCount).map(device::getInterface).map { Triple(it.interfaceClass, it.interfaceSubclass, it.interfaceProtocol) }

    internal fun label(manufacturer: String?, product: String?): String =
        listOfNotNull(manufacturer, product).map { it.trim() }.filter { it.isNotEmpty() }.distinct().joinToString(" ").ifBlank { "USB card reader" }.replace('\t', ' ')

    @JvmStatic
    fun devices(): Array<String> {
        val readers = manager()?.deviceList?.values.orEmpty().filter { isCardReader(interfaces(it)) }
        asked.keys.retainAll(readers.map { it.deviceName }.toSet())
        return readers.map { device ->
            val name = label(runCatching { device.manufacturerName }.getOrNull(), runCatching { device.productName }.getOrNull())
            "${device.deviceName}\t$name"
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
