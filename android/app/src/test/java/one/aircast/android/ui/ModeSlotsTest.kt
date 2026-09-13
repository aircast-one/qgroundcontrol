package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

private fun view(vararg live: Int) = JSONObject(
    """
    {"available": true, "channel": 5, "liveSlot": ${live.firstOrNull() ?: 0}, "reason": "",
     "slots": [
       {"slot": 1, "mode": "Acro", "live": ${live.contains(1)}},
       {"slot": 2, "mode": "AltHold", "live": ${live.contains(2)}},
       {"slot": 3, "mode": "Auto", "live": ${live.contains(3)}},
       {"slot": 4, "mode": "Guided", "live": ${live.contains(4)}},
       {"slot": 5, "mode": "Loiter", "live": ${live.contains(5)}},
       {"slot": 6, "mode": "RTL", "live": ${live.contains(6)}}
     ]}
    """.trimIndent(),
)

class ModeSlotsTest {
    @Test
    fun `the live slot is named by its number and its mode, which is what the page exists to answer`() {
        assertEquals(
            "The switch on channel 5 is on slot 4, Guided.",
            liveSlotText(modeSlotsView(view(4))),
        )
    }

    @Test
    fun `a switch sitting between the bands says so rather than naming a slot`() {
        assertEquals(
            "The switch on channel 5 is not on any mode slot.",
            liveSlotText(modeSlotsView(view())),
        )
    }

    @Test
    fun `a vehicle that does not pick modes from a channel has nothing to show`() {
        val unavailable = JSONObject("""{"available": false, "slots": [], "liveSlot": 0}""")
        assertNull(modeSlotsView(unavailable))
        assertNull(liveSlotText(null))
    }
}
