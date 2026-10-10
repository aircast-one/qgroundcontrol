package one.aircast.android

import android.content.Context

private val BUILT_IN_RADIO_DEVICES = mapOf("com.Flyshark.RadioMasterAX" to "radiomaster-ax12")

fun hostDevice(context: Context): String? =
    BUILT_IN_RADIO_DEVICES.entries.firstOrNull { (marker, _) -> runCatching { context.packageManager.getPackageInfo(marker, 0) }.isSuccess }?.value
