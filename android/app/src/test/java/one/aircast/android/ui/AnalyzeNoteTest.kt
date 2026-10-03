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
            "Only PX4 vehicles answer this shell",
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

    @Test
    fun theListShowsUnreadMessagesAndTheVibrationVerdict() {
        assertEquals("3" to SetupState.NeedsAttention, analyzeStatus(AnalyzePage.Messages, 3, null))
        assertEquals("" to SetupState.Neutral, analyzeStatus(AnalyzePage.Messages, 0, null))
        assertEquals("Unsafe" to SetupState.NeedsAttention, analyzeStatus(AnalyzePage.Vibration, 0, "danger"))
        assertEquals("Healthy" to SetupState.Done, analyzeStatus(AnalyzePage.Vibration, 0, "normal"))
        assertEquals("" to SetupState.Neutral, analyzeStatus(AnalyzePage.Console, 5, "danger"))
    }

    @Test
    fun `the messages row reads what the vehicle has said, the others what the tool does`() {
        val said = listOf(VehicleMessage(0, "", "", MessageSeverity.Warning, "Low battery"), VehicleMessage(1, "", "", MessageSeverity.Normal, "Armed"))
        org.junit.Assert.assertEquals("1 warning \u00b7 1 message", analyzeSubtitle(AnalyzePage.Messages, said))
        org.junit.Assert.assertEquals(AnalyzePage.Messages.description, analyzeSubtitle(AnalyzePage.Messages, emptyList()))
        org.junit.Assert.assertEquals(AnalyzePage.Console.description, analyzeSubtitle(AnalyzePage.Console, said))
    }

    @Test
    fun `the vibration row reads the three axes in their unit`() {
        val axis = { name: String, value: Double? -> VibrationAxis(name, value, 0f, null) }
        val reading = VibrationReading("m/s\u00b2", 60.0, 30.0, 60.0, listOf(axis("X", 12.4), axis("Y", 8.6), axis("Z", 18.0)), emptyList())
        assertEquals("X 12 \u00b7 Y 9 \u00b7 Z 18 m/s\u00b2", analyzeSubtitle(AnalyzePage.Vibration, emptyList(), reading))
        assertEquals(AnalyzePage.Vibration.description, analyzeSubtitle(AnalyzePage.Vibration, emptyList(), reading.copy(axes = listOf(axis("X", null)))))
        assertEquals(AnalyzePage.Vibration.description, analyzeSubtitle(AnalyzePage.Vibration, emptyList(), null))
    }

    @Test
    fun `the inspector row reads the total message rate`() {
        val view = JSONObject("""{"available":true,"messages":[{"rateHz":5.0},{"rateHz":10.4},{"rateHz":0}]}""")
        assertEquals("15 messages/s", inspectorRateText(view))
        assertNull(inspectorRateText(JSONObject("""{"available":true,"messages":[]}""")))
        assertEquals(AnalyzePage.Inspector.description, analyzeSubtitle(AnalyzePage.Inspector, emptyList(), null, null))
    }
}
