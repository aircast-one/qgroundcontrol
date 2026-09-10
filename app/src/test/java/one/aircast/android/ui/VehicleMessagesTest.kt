package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class VehicleMessagesTest {

    private val served = """
        {"kind":"object","class":"VehicleMessages","items":[
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
            """{"class":"VehicleMessages","items":[
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
}
