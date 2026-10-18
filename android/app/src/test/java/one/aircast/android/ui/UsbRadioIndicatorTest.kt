package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class UsbRadioIndicatorTest {
    @Test
    fun `reads the core's radio state and hides while Radio on USB is off`() {
        assertEquals(UsbRadioBadge("Radio · live", UsbRadioState.Live), usbRadioBadge(JSONObject("""{"indicator":{"text":"Radio · live","state":"live"}}""")))
        assertNull(usbRadioBadge(JSONObject("""{"indicator":null}""")))
        assertNull(usbRadioBadge(JSONObject("""{"indicator":{"text":"?","state":"exploded"}}""")))
    }

    @Test
    fun `a stopped radio reads as an error and a paused one as a warning`() {
        assertEquals(UsbRadioTone.Error, usbRadioTone(UsbRadioState.Stopped))
        assertEquals(UsbRadioTone.Warning, usbRadioTone(UsbRadioState.Paused))
        assertNull(usbRadioTone(UsbRadioState.Live))
        assertNull(usbRadioTone(UsbRadioState.Standby))
    }
}
