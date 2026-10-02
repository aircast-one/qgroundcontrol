package one.aircast.android.ui

import android.content.Context
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.LocalContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.settingControl

internal const val PALETTE_SETTING = "settings.appSettings.indoorPalette"
internal const val PALETTE_INDOOR = 1
internal const val PALETTE_OUTDOOR = 0
private const val PALETTE_STORE = "appearance"
private const val PALETTE_DEFAULTED = "paletteDefaultedToDark"

private val PALETTE_NAMES = mapOf("$PALETTE_INDOOR" to "Dark", "$PALETTE_OUTDOOR" to "Light")

internal fun paletteNamed(fact: Fact): Fact =
    if (fact.path != PALETTE_SETTING || fact.enumValues.size != fact.enumStrings.size) fact
    else fact.copy(enumStrings = fact.enumValues.zip(fact.enumStrings) { raw, label -> PALETTE_NAMES[raw] ?: label })

internal fun paletteIsDark(value: Int?, systemDark: Boolean): Boolean = when (value) {
    PALETTE_INDOOR -> true
    PALETTE_OUTDOOR -> false
    null -> true
    else -> systemDark
}

internal fun shouldDefaultToDark(changedFromDefault: Boolean, alreadyDefaulted: Boolean): Boolean =
    !changedFromDefault && !alreadyDefaulted

@Composable
internal fun appDarkTheme(): Boolean {
    val context = LocalContext.current
    val control by qgcPath(settingControl(PALETTE_SETTING))
    val value = control?.opt("value")?.let { (it as? Number)?.toInt() ?: it.toString().toIntOrNull() }
    LaunchedEffect(control != null) {
        val read = control ?: return@LaunchedEffect
        val store = context.getSharedPreferences(PALETTE_STORE, Context.MODE_PRIVATE)
        if (shouldDefaultToDark(read.optBoolean("changedFromDefault"), store.getBoolean(PALETTE_DEFAULTED, false))) {
            offMainDetached { Qgc.set(PALETTE_SETTING, PALETTE_INDOOR) }
        }
        store.edit().putBoolean(PALETTE_DEFAULTED, true).apply()
    }
    return paletteIsDark(value, isSystemInDarkTheme())
}
