package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal const val USB_RADIO_VIEW = "view.usbRadio"

internal enum class UsbRadioState { Live, Paused, Standby, Ready, Stopped }

internal enum class UsbRadioTone { Error, Warning }

internal data class UsbRadioBadge(val text: String, val state: UsbRadioState)

internal fun usbRadioBadge(view: JSONObject?): UsbRadioBadge? {
    val it = view?.optJSONObject("indicator") ?: return null
    val state = UsbRadioState.entries.find { known -> known.name.lowercase() == it.optText("state") } ?: return null
    return UsbRadioBadge(it.optText("text"), state)
}

internal fun usbRadioTone(state: UsbRadioState): UsbRadioTone? = when (state) {
    UsbRadioState.Stopped -> UsbRadioTone.Error
    UsbRadioState.Paused -> UsbRadioTone.Warning
    else -> null
}
