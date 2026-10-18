package one.aircast.android

import android.content.Context
import android.content.Intent

private val BUILT_IN_RADIO_DEVICES = mapOf("com.Flyshark.RadioMasterAX" to "radiomaster-ax12")

fun hostDevice(context: Context): String? =
    BUILT_IN_RADIO_DEVICES.entries.firstOrNull { (marker, _) -> runCatching { context.packageManager.getPackageInfo(marker, 0) }.isSuccess }?.value

fun stopApp(owner: String): Boolean =
    runCatching { Runtime.getRuntime().exec(arrayOf("su", "0", "am", "force-stop", owner)).waitFor() == 0 }.getOrDefault(false)

fun openApp(context: Context, owner: String): Boolean =
    context.packageManager.getLaunchIntentForPackage(owner)
        ?.let { intent -> runCatching { context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }.isSuccess }
        ?: false
