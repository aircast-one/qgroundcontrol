package one.aircast.android.ui

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.wifi.WifiInfo
import android.net.wifi.WifiManager
import android.os.Build
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.produceState
import androidx.compose.ui.platform.LocalContext

private const val UNKNOWN_SSID = "<unknown ssid>"

internal fun phoneSsid(raw: String?): String? =
    raw?.trim()?.removeSurrounding("\"")?.trim()?.takeIf { it.isNotEmpty() && it != UNKNOWN_SSID && it != "0x" }

@Suppress("DEPRECATION")
private fun legacySsid(context: Context): String? =
    runCatching { context.applicationContext.getSystemService(WifiManager::class.java)?.connectionInfo?.ssid }.getOrNull()

internal data class PhoneWifi(val ssid: String?, val secured: Boolean?)

internal fun securedFrom(securityType: Int): Boolean? = when (securityType) {
    WifiInfo.SECURITY_TYPE_UNKNOWN -> null
    WifiInfo.SECURITY_TYPE_OPEN, WifiInfo.SECURITY_TYPE_OWE -> false
    else -> true
}

@Composable
internal fun rememberPhoneWifi(): State<PhoneWifi> {
    val context = LocalContext.current
    return produceState(initialValue = PhoneWifi(null, null), context) {
        val connectivity = context.getSystemService(ConnectivityManager::class.java)
        if (connectivity == null || Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
            value = PhoneWifi(phoneSsid(legacySsid(context)), null)
            return@produceState
        }
        val callback = object : ConnectivityManager.NetworkCallback(FLAG_INCLUDE_LOCATION_INFO) {
            override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) {
                val info = capabilities.transportInfo as? WifiInfo ?: return
                value = PhoneWifi(phoneSsid(info.ssid) ?: value.ssid, securedFrom(info.currentSecurityType))
            }
        }
        val request = NetworkRequest.Builder().addTransportType(NetworkCapabilities.TRANSPORT_WIFI).build()
        runCatching { connectivity.registerNetworkCallback(request, callback) }
        awaitDispose { runCatching { connectivity.unregisterNetworkCallback(callback) } }
    }
}

internal fun phoneIsOnMeteredNetwork(context: Context): Boolean =
    runCatching { context.getSystemService(ConnectivityManager::class.java)?.isActiveNetworkMetered }.getOrNull() ?: false
