package one.aircast.android.ui

import one.aircast.android.hatBits
import one.aircast.android.scaled
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class JoystickScreenTest {
    @Test
    fun the_page_reads_devices_settings_and_axes() {
        assertNull(joystickPage(JSONObject("""{"available":false}""")))
        val page = joystickPage(JSONObject("""{"available":true,"names":["Pad"],"active":"Pad","vehicle":true,"enabled":false,"calibrated":false,
            "settings":[{"name":"useDeadband","type":"bool","label":"Use deadband","units":"","value":true}],
            "state":{"axes":[{"index":0,"raw":1200,"function":"roll"}]}}"""))!!
        assertEquals("Pad", page.active)
        assertEquals(true, page.settings.single().value)
        assertEquals(JoystickAxis(0, 1200, "roll"), page.axes.single())
    }

    @Test
    fun gamepad_values_scale_to_the_core_range_and_the_dpad_becomes_hat_bits() {
        assertEquals(32767, scaled(1f))
        assertEquals(-32767, scaled(-2f))
        assertEquals(0x01 or 0x08, hatBits(-1f, -1f))
        assertEquals(0x04 or 0x02, hatBits(1f, 1f))
        assertEquals(0, hatBits(0.2f, -0.2f))
    }
}
