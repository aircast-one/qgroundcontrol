package one.aircast.android.ui

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.refusal

internal object DeviceLink {
    suspend fun connect(address: String): String? = withContext(Dispatchers.Default) { refusal(Qgc.call(DEVICE_CONNECT, address), "The device") }

    fun disconnect() = offMainDetached { Qgc.invoke(DEVICE_DISCONNECT) }
}
