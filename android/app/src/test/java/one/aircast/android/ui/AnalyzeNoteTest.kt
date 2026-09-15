package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AnalyzeNoteTest {

    @Test
    fun `a console row does not promise a shell the vehicle will not answer`() {
        assertEquals(
            "the head knows the firmware in the list and made the operator open the page to find out",
            "The shell answers on PX4; this vehicle reports another autopilot",
            analyzeNote(AnalyzePage.Console, connected = true, px4 = false, vibrationAvailable = true),
        )
        assertNull(analyzeNote(AnalyzePage.Console, connected = true, px4 = true, vibrationAvailable = true))
    }

    @Test
    fun `a vibration row says when the vehicle is sending none`() {
        assertEquals(
            "This vehicle is not reporting vibration",
            analyzeNote(AnalyzePage.Vibration, connected = true, px4 = true, vibrationAvailable = false),
        )
        assertNull(analyzeNote(AnalyzePage.Vibration, connected = true, px4 = true, vibrationAvailable = true))
    }

    @Test
    fun `with no vehicle every row stays quiet rather than repeating the same sentence five times`() {
        AnalyzePage.entries.forEach { page ->
            assertNull(
                "the screens themselves say to connect a vehicle, and five copies of it in a menu is noise",
                analyzeNote(page, connected = false, px4 = false, vibrationAvailable = false),
            )
        }
    }

    @Test
    fun `the rows this head cannot answer for carry nothing`() {
        listOf(AnalyzePage.LogDownload, AnalyzePage.Inspector, AnalyzePage.GeoTag).forEach { page ->
            assertNull(analyzeNote(page, connected = true, px4 = false, vibrationAvailable = false))
        }
    }
}
