package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FlyStateTest {
    private val inContact = JSONObject(
        """{"class":"FlyState","connected":true,"armed":false,"flying":false,"landing":false,
            "contactLost":false,"state":"disarmed","stateText":"Stabilize · Disarmed",
            "staleNotice":"","mode":"Stabilize"}""",
    )

    private val lost = JSONObject(
        """{"class":"FlyState","connected":true,"armed":false,"flying":false,"landing":false,
            "contactLost":true,"state":"contactLost","stateText":"Communication lost",
            "staleNotice":"No contact — these are the last values the vehicle sent.",
            "mode":"Stabilize"}""",
    )

    @Test
    fun `the sentence and the token come from the core, not from the head`() {
        val state = flyState(inContact)!!

        assertEquals("disarmed", state.state)
        assertEquals("Stabilize · Disarmed", state.stateText)
    }

    @Test
    fun `a vehicle in contact carries no notice, so nothing dims`() {
        val state = flyState(inContact)!!

        assertFalse(state.contactLost)
        assertEquals("", state.staleNotice)
    }

    @Test
    fun `lost contact carries the notice and the head shows it verbatim`() {
        val state = flyState(lost)!!

        assertTrue(state.contactLost)
        assertEquals("Communication lost", state.stateText)
        assertEquals(
            "No contact — these are the last values the vehicle sent.",
            state.staleNotice,
        )
    }

    @Test
    fun `the notice uses the core's dash, not an ascii hyphen`() {
        assertTrue(flyState(lost)!!.staleNotice.contains("—"))
        assertFalse(flyState(lost)!!.staleNotice.contains(" - "))
    }

    @Test
    fun `a payload that is not the fly state yields nothing`() {
        assertNull(flyState(null))
        assertNull(flyState(JSONObject("{}")))
        assertNull(flyState(JSONObject("""{"class":"Calibration"}""")))
    }

    @Test
    fun `a payload using an invented name for the notice leaves it blank`() {
        val invented = JSONObject(
            """{"class":"FlyState","contactLost":true,"stale":"No contact","notice":"No contact"}""",
        )

        assertEquals("", flyState(invented)!!.staleNotice)
    }
}

class VehicleSubtitleTest {
    private fun state(
        connected: Boolean = true,
        contactLost: Boolean = false,
        stateText: String = "Disarmed",
        mode: String = "Stabilize",
    ) = FlyState(connected, contactLost, "disarmed", stateText, "", mode, false, "", null)

    @Test
    fun `the header keeps the flight mode the core reports separately`() {
        assertEquals("Stabilize · Disarmed", vehicleSubtitle(state()))
    }

    @Test
    fun `a lost link says so on its own, because the mode is no longer known`() {
        assertEquals(
            "Communication lost",
            vehicleSubtitle(state(contactLost = true, stateText = "Communication lost")),
        )
    }

    @Test
    fun `no vehicle says so rather than showing an empty line`() {
        assertEquals("No vehicle", vehicleSubtitle(null))
        assertEquals("No vehicle", vehicleSubtitle(state(connected = false)))
    }

    @Test
    fun `a blank mode does not leave a dangling separator`() {
        assertEquals("Armed", vehicleSubtitle(state(stateText = "Armed", mode = "")))
    }
}
