package one.aircast.android

import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothSocket
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import java.io.IOException
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

@SuppressLint("MissingPermission")
object BluetoothLinks {
    private val SERIAL_PORT_SERVICE: UUID = UUID.fromString("00001101-0000-1000-8000-00805F9B34FB")
    private const val READ_BUFFER_BYTES = 1024
    private var context: Context? = null
    private val found = ConcurrentHashMap<String, String>()
    private val sockets = ConcurrentHashMap<Long, BluetoothSocket>()

    private val discoveries = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            val device: BluetoothDevice? = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                intent.getParcelableExtra(BluetoothDevice.EXTRA_DEVICE, BluetoothDevice::class.java)
            } else {
                @Suppress("DEPRECATION")
                intent.getParcelableExtra(BluetoothDevice.EXTRA_DEVICE)
            }
            device?.let { found[it.address] = runCatching { it.name }.getOrNull().orEmpty() }
        }
    }

    fun initialize(context: Context) {
        if (this.context != null) return
        this.context = context.applicationContext
        val filter = IntentFilter(BluetoothDevice.ACTION_FOUND)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            context.applicationContext.registerReceiver(discoveries, filter, Context.RECEIVER_EXPORTED)
        } else {
            context.applicationContext.registerReceiver(discoveries, filter)
        }
    }

    private fun adapter(): BluetoothAdapter? = context?.getSystemService(BluetoothManager::class.java)?.adapter

    @JvmStatic
    fun devicesInfo(): Array<String> = runCatching {
        val bonded = adapter()?.bondedDevices.orEmpty().associate { it.address to runCatching { it.name }.getOrNull().orEmpty() }
        (bonded + found).map { (address, name) -> "$name\t$address" }.toTypedArray()
    }.getOrDefault(emptyArray())

    @JvmStatic
    fun scan(start: Int): Boolean = runCatching {
        val adapter = adapter() ?: return false
        if (start != 0) {
            found.clear()
            adapter.startDiscovery()
        } else {
            adapter.cancelDiscovery()
        }
    }.getOrDefault(false)

    @JvmStatic
    fun scanning(): Boolean = runCatching { adapter()?.isDiscovering == true }.getOrDefault(false)

    @JvmStatic
    fun open(address: String, id: Long): Boolean = try {
        val adapter = adapter() ?: throw IOException("This phone has no Bluetooth adapter.")
        adapter.cancelDiscovery()
        val socket = adapter.getRemoteDevice(address).createRfcommSocketToServiceRecord(SERIAL_PORT_SERVICE)
        socket.connect()
        sockets[id] = socket
        Thread({ read(id, socket) }, "bluetooth-link-$id").start()
        true
    } catch (failure: Exception) {
        false
    }

    private fun read(id: Long, socket: BluetoothSocket) {
        val buffer = ByteArray(READ_BUFFER_BYTES)
        val reason = try {
            generateSequence { socket.inputStream.read(buffer) }
                .takeWhile { it >= 0 }
                .forEach { count -> if (count > 0) nativeNewData(id, buffer.copyOf(count)) }
            "The Bluetooth device closed the connection."
        } catch (failure: IOException) {
            failure.message ?: "The Bluetooth connection was lost."
        }
        if (sockets.remove(id) != null) nativeClosed(id, reason)
    }

    @JvmStatic
    fun write(id: Long, data: ByteArray): Boolean = runCatching {
        val socket = sockets[id] ?: return false
        socket.outputStream.write(data)
        true
    }.getOrDefault(false)

    @JvmStatic
    fun close(id: Long) {
        sockets.remove(id)?.let { runCatching { it.close() } }
    }

    @JvmStatic
    private external fun nativeNewData(id: Long, data: ByteArray)

    @JvmStatic
    private external fun nativeClosed(id: Long, message: String)
}
