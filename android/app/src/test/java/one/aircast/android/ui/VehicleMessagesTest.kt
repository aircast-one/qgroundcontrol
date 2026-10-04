package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class VehicleMessagesTest {
    @Test
    fun `the log summary counts errors and warnings apart from the rest`() {
        fun message(level: MessageSeverity) = VehicleMessage(0, "", "", level, "x")
        org.junit.Assert.assertEquals("1 error \u00b7 2 warnings \u00b7 1 message", severitySummary(listOf(message(MessageSeverity.Error), message(MessageSeverity.Warning), message(MessageSeverity.Warning), message(MessageSeverity.Normal))))
        org.junit.Assert.assertEquals("0 messages from the vehicle", severitySummary(emptyList()))
    }

    @Test
    fun `the chip carries the arming blocker as a short reason`() {
        assertEquals("GPS 1: not healthy", chipBlocker("PreArm: GPS 1: not healthy"))
        assertEquals("No GPS lock", chipBlocker("No GPS lock. This vehicle needs a position fix before it will arm."))
        assertEquals("Throttle too high", chipBlocker("Throttle too high"))
        val fly = FlyState(true, false, false, "disarmed", "Not ready", "", "Guided", false, "", null, null, null)
        assertEquals("Guided \u00b7 GPS 1: not healthy +2", readinessSubtitle(fly, "PreArm: GPS 1: not healthy", 3))
        assertEquals("Guided \u00b7 Compass not calibrated", readinessSubtitle(fly, "Compass not calibrated", 1))
        assertNull(readinessSubtitle(fly, null, 0))
    }

    @Test
    fun `the banner only interrupts for warnings and errors the chip is not already saying`() {
        fun message(level: MessageSeverity, text: String) = VehicleMessage(0, "", "", level, text)
        val unread = listOf(
            message(MessageSeverity.Error, "PreArm: GPS 1: not healthy"),
            message(MessageSeverity.Normal, "EKF3 IMU0 is using GPS"),
            message(MessageSeverity.Warning, "Battery low"),
        )
        assertEquals(listOf("Battery low"), bannerMessages(unread).map { it.text })
        assertTrue(bannerMessages(unread.take(2)).isEmpty())
        assertEquals("Battery low", bannerText(bannerMessages(unread)))
    }

    @Test
    fun `a message time drops the milliseconds QGC stamps it with`() {
        org.junit.Assert.assertEquals("20:00:53", messageTime("20:00:53.747"))
        org.junit.Assert.assertEquals("yesterday", messageTime("yesterday"))
    }


    private val served = """
        {"kind":"object","class":"VehicleMessages","order":"oldestFirst","items":[
          {"index":0,"time":"1:2:3.4","component":null,"severity":"Error","level":"error","text":"Battery < 20% & \"low\""},
          {"index":1,"time":"1:2:4.0","component":190,"severity":"Notice","level":"warning","text":"heads up"},
          {"index":2,"time":"1:2:5.0","component":null,"severity":"","level":"normal","text":"plain"}]}
    """

    @Test
    fun `messages come from the core with their level and time`() {
        val messages = vehicleMessages(JSONObject(served))

        assertEquals(listOf("Battery < 20% & \"low\"", "heads up", "plain"), messages.map { it.text })
        assertEquals(
            listOf(MessageSeverity.Error, MessageSeverity.Warning, MessageSeverity.Normal),
            messages.map { it.level },
        )
        assertEquals(listOf("1:2:3.4", "1:2:4.0", "1:2:5.0"), messages.map { it.time })
        assertEquals(listOf(0, 1, 2), messages.map { it.index })
    }

    @Test
    fun `the level comes from the core's token, not from a word that gets translated`() {
        val german = JSONObject(
            """{"class":"VehicleMessages","order":"oldestFirst","items":[
                 {"index":0,"time":"1:2:3.4","severity":"Fehler","level":"error","text":"kaputt"}]}""",
        )

        assertEquals(MessageSeverity.Error, vehicleMessages(german).single().level)
        assertEquals("Fehler", vehicleMessages(german).single().severity)
    }

    @Test
    fun `an unknown level is treated as ordinary rather than dropped`() {
        assertEquals(MessageSeverity.Normal, levelOf("something-new"))
        assertEquals(MessageSeverity.Normal, levelOf(""))
    }

    @Test
    fun `nothing to say reads as an empty list`() {
        assertTrue(vehicleMessages(null).isEmpty())
        assertTrue(vehicleMessages(JSONObject("""{"kind":"null"}""")).isEmpty())
        assertTrue(vehicleMessages(JSONObject("""{"class":"VehicleMessages","items":[]}""")).isEmpty())
    }

    @Test
    fun `a message with no text is not shown as a blank row`() {
        val blank = JSONObject("""{"class":"VehicleMessages","items":[{"index":0,"text":"","level":"error"}]}""")

        assertTrue(vehicleMessages(blank).isEmpty())
    }

    @Test
    fun `an arming blocker is read only when the core sets one`() {
        assertEquals("Throttle too high", armingBlocker(JSONObject("""{"armingBlocker":"Throttle too high"}""")))
        assertNull(armingBlocker(JSONObject("""{"armingBlocker":null}""")))
        assertNull(armingBlocker(JSONObject("""{"armingBlocker":""}""")))
        assertNull(armingBlocker(null))
    }

    @Test
    fun `an error is named in the banner, not buried in a count`() {
        val messages = oldestFirst(
            message(0, MessageSeverity.Normal, "Armed"),
            message(1, MessageSeverity.Error, "EKF variance"),
            message(2, MessageSeverity.Normal, "Mode changed"),
        )

        assertEquals("EKF variance", bannerText(messages))
    }

    @Test
    fun `the newest error wins, and the core serves its lists oldest first`() {
        val messages = oldestFirst(
            message(0, MessageSeverity.Error, "Compass variance"),
            message(1, MessageSeverity.Error, "EKF variance"),
        )

        assertEquals("EKF variance", bannerText(messages))
    }

    @Test
    fun `the log renders newest first, which is the order QGC's own log uses`() {
        val messages = oldestFirst(
            message(0, MessageSeverity.Normal, "oldest"),
            message(1, MessageSeverity.Normal, "newest"),
        )

        assertEquals(listOf("newest", "oldest"), messages.asReversed().map { it.text })
    }

    @Test
    fun `a view that says newest first is turned round rather than trusted to match`() {
        val body = """{"order":"newestFirst","items":[
            {"index":0,"time":"12:01","severity":"","level":"normal","text":"newest"},
            {"index":1,"time":"12:00","severity":"","level":"normal","text":"oldest"}]}"""

        assertEquals(listOf("oldest", "newest"), vehicleMessages(JSONObject(body)).map { it.text })
    }

    @Test
    fun `the order the core actually serves is taken as given`() {
        val body = """{"order":"$OLDEST_FIRST","items":[
            {"index":0,"time":"12:00","severity":"","level":"normal","text":"oldest"},
            {"index":1,"time":"12:01","severity":"","level":"normal","text":"newest"}]}"""

        assertEquals(listOf("oldest", "newest"), vehicleMessages(JSONObject(body)).map { it.text })
    }

    @Test
    fun `a warning is named when nothing worse has happened`() {
        val messages = listOf(message(0, MessageSeverity.Warning, "Low battery"))

        assertEquals("Low battery", bannerText(messages))
    }

    @Test
    fun `routine chatter does not interrupt the pilot`() {
        val messages = listOf(
            message(0, MessageSeverity.Normal, "Armed"),
            message(1, MessageSeverity.Normal, "Disarmed"),
        )

        assertTrue(bannerMessages(messages).isEmpty())
    }

    @Test
    fun `nothing to say is nothing shown`() {
        assertNull(bannerText(emptyList()))
    }

    private fun oldestFirst(vararg messages: VehicleMessage) = messages.toList()

    private fun message(index: Int, level: MessageSeverity, text: String) =
        VehicleMessage(index, "12:00", level.name.lowercase(), level, text)
}

class VehicleMessagesContractTest {

    @org.junit.Test
    fun `the decoder reads the key the core actually serves`() {
        val recorded = JSONObject(
            """{"kind":"object","class":"VehicleMessages","count":1,
                "items":[{"index":0,"time":"1:2:3.4","component":null,"severity":"Error",
                          "level":"error","text":"real"}]}""",
        )

        assertEquals(listOf("real"), vehicleMessages(recorded).map { it.text })
        assertTrue(
            "a fixture invented by the head must not pass where the recorded one fails",
            vehicleMessages(JSONObject("""{"class":"VehicleMessages","messages":[{"text":"x"}]}""")).isEmpty(),
        )
    }

    @Test
    fun `opening the log marks what was there as read, as resetAllMessages does`() {
        val messages = listOf(
            VehicleMessage(0, "", "error", MessageSeverity.Error, "EKF variance"),
            VehicleMessage(1, "", "info", MessageSeverity.Normal, "Armed"),
        )
        assertEquals(2, unreadMessages(messages, 2).size)
        assertEquals("an old error no longer turns the banner red", emptyList<VehicleMessage>(), unreadMessages(messages, 0))
        val newer = messages + VehicleMessage(2, "", "warning", MessageSeverity.Warning, "Low battery")
        assertEquals("the core counts what arrived since resetAllMessages, newest last", listOf("Low battery"), unreadMessages(newer, 1).map { it.text })
        assertEquals(1, unreadCount(JSONObject("""{"unread":1}""")))
        assertEquals(0, unreadCount(null))
        assertEquals("2 messages from the vehicle", messageCountText(2))
    }

}
