package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.json.JSONObject
import org.junit.Test

private const val NONE = "This vehicle is not reporting vibration"

class AnalyzeNoteTest {

    @Test
    fun `a console row does not promise a shell the vehicle will not answer`() {
        assertEquals(
            "the head knows the firmware in the list and made the operator open the page to find out",
            "The shell answers on PX4; this vehicle reports another autopilot",
            analyzeNote(AnalyzePage.Console, connected = true, px4 = false, vibration = null),
        )
        assertNull(analyzeNote(AnalyzePage.Console, connected = true, px4 = true, vibration = null))
    }

    @Test
    fun `a vibration row says when the vehicle is sending none`() {
        assertEquals(
            NONE,
            analyzeNote(AnalyzePage.Vibration, connected = true, px4 = true, vibration = NONE),
        )
        assertNull(analyzeNote(AnalyzePage.Vibration, connected = true, px4 = true, vibration = null))
    }

    @Test
    fun `the row and the screen answer partly-reported from one source`() {
        fun view(available: Boolean, reason: String) = JSONObject(
            """{"available":$available,"silentReason":$reason}""",
        )

        assertEquals("This vehicle is not reporting vibration", vibrationCaveat(view(false, "\"notReported\"")))
        assertEquals(
            "available is all three axes, so saying 'not reporting' on a vehicle sending one of " +
                "them is a claim the view does not support - the row said it until the screen " +
                "beside it was fixed to distinguish them",
            "This vehicle is reporting only some vibration axes",
            vibrationCaveat(view(false, "null")),
        )
        assertNull(vibrationCaveat(view(true, "null")))
        assertNull(vibrationCaveat(null))
    }

    @Test
    fun `with no vehicle every row stays quiet rather than repeating the same sentence five times`() {
        AnalyzePage.entries.forEach { page ->
            assertNull(
                "the screens themselves say to connect a vehicle, and five copies of it in a menu is noise",
                analyzeNote(page, connected = false, px4 = false, vibration = NONE),
            )
        }
    }

    @Test
    fun `the rows this head cannot answer for carry nothing`() {
        listOf(AnalyzePage.LogDownload, AnalyzePage.Inspector, AnalyzePage.GeoTag).forEach { page ->
            assertNull(analyzeNote(page, connected = true, px4 = false, vibration = NONE))
        }
    }
}
