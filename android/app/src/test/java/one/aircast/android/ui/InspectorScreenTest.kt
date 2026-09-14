package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class InspectorScreenTest {


    @Test
    fun `fields keep their declared order`() {
        val model = JSONObject(
            """
            {"elements":[
              {"name":"roll","type":"float","value":"0.001"},
              {"name":"pitch","type":"float","value":"0.002"}
            ]}
            """.trimIndent(),
        )
        val fields = parseInspectorFields(model)
        assertEquals(listOf("roll", "pitch"), fields.map { it.name })
        assertEquals(InspectorField("roll", "float", "0.001"), fields[0])
    }


}

class InspectorOpenMessageTest {
    private fun message(index: Int, name: String) =
        InspectorMessage(
            index = index,
            id = index,
            name = name,
            rateText = "1.0 Hz",
            count = 1L,
            path = "mavlinkInspector.activeSystem.messages.$index",
            compId = 1,
            title = name,
        )

    @Test
    fun `the open message follows its name when the model is rebuilt at other indices`() {
        val before = listOf(message(0, "HEARTBEAT"), message(1, "ATTITUDE"))
        val after = listOf(message(0, "SYS_STATUS"), message(1, "HEARTBEAT"), message(2, "ATTITUDE"))

        assertEquals(1, openMessageIn(before, "mavlinkInspector.activeSystem.messages.1")?.index)
        assertEquals(2, openMessageIn(after, "mavlinkInspector.activeSystem.messages.2")?.index)
    }

    @Test
    fun `a message that leaves the model closes rather than showing another one`() {
        val after = listOf(message(0, "HEARTBEAT"))

        assertNull(openMessageIn(after, "ATTITUDE"))
    }

    @Test
    fun `nothing open resolves to nothing`() {
        assertNull(openMessageIn(listOf(message(0, "HEARTBEAT")), null))
    }

    @Test
    fun `the table reads the core's rate text and path`() {
        val messages = inspectorMessages(
            JSONObject("""{"messages":[
                {"index":1,"id":30,"name":"ATTITUDE","rateText":"10.0 Hz","count":420,
                 "path":"mavlinkInspector.activeSystem.messages.1"},
                {"index":0,"id":0,"name":"HEARTBEAT","rateText":"1.0 Hz","count":42,
                 "path":"mavlinkInspector.activeSystem.messages.0"}]}"""),
        )

        assertEquals(listOf("ATTITUDE", "HEARTBEAT"), messages.map { it.name })
        assertEquals(listOf("10.0 Hz", "1.0 Hz"), messages.map { it.rateText })
        assertEquals(listOf(1, 0), messages.map { it.index })
    }

    @Test
    fun `no messages is an empty table, not a crash`() {
        assertEquals(emptyList<InspectorMessage>(), inspectorMessages(null))
        assertEquals(emptyList<InspectorMessage>(), inspectorMessages(JSONObject("{}")))
    }

    @Test
    fun `two components sending the same message are distinct rows`() {
        val messages = inspectorMessages(
            JSONObject("""{"messages":[
                {"index":0,"id":271,"name":"CAMERA_CAPTURE_STATUS","rateText":"1.0 Hz","count":9,
                 "compId":100,"path":"mavlinkInspector.systems.1.messages.0"},
                {"index":1,"id":271,"name":"CAMERA_CAPTURE_STATUS","rateText":"1.0 Hz","count":9,
                 "compId":101,"path":"mavlinkInspector.systems.1.messages.1"}]}"""),
        )

        assertEquals(2, messages.size)
        assertEquals(2, messages.map { it.path }.toSet().size)
    }



    @Test
    fun `the row title is the core's, falling back to the name when absent`() {
        val served = inspectorMessages(
            JSONObject("""{"messages":[
                {"index":0,"id":271,"name":"CAMERA_CAPTURE_STATUS","rateText":"1.0 Hz","count":9,
                 "compId":100,"path":"p0","title":"CAMERA_CAPTURE_STATUS (comp 100)"},
                {"index":1,"id":30,"name":"ATTITUDE","rateText":"5.0 Hz","count":99,
                 "compId":1,"path":"p1"}]}"""),
        )

        assertEquals("CAMERA_CAPTURE_STATUS (comp 100)", served.first { it.path == "p0" }.title)
        assertEquals("ATTITUDE", served.first { it.path == "p1" }.title)
    }

    @Test
    fun `the fields and the selection follow the served path, not a rebuilt one`() {
        val path = "mavlinkInspector.systems.0.messages.3"

        assertEquals("mavlinkInspector.systems.0.selected", selectedPathFor(path))
        assertEquals(3, messageIndexIn(path))
    }

    @Test
    fun `a path the core words differently still yields its own system`() {
        assertEquals(
            "mavlinkInspector.systems.2.selected",
            selectedPathFor("mavlinkInspector.systems.2.messages.11"),
        )
        assertEquals(11, messageIndexIn("mavlinkInspector.systems.2.messages.11"))
    }
}

class InspectorRateTest {
    @Test
    fun `no view offers no rates`() {
        assertTrue(inspectorRateChoices(null).isEmpty())
    }

    @Test
    fun `rate choices come from the core with their titles`() {
        val view = JSONObject(
            """{"rateChoices":[{"rate":-1,"title":"Disabled"},{"rate":0,"title":"Default"},{"rate":10,"title":"10 Hz"}]}""",
        )
        val choices = inspectorRateChoices(view)
        assertEquals(3, choices.size)
        assertEquals(-1, choices[0].rate)
        assertEquals("Disabled", choices[0].title)
        assertEquals("10 Hz", choices[2].title)
    }

    @Test
    fun `a choice with no title is not offered`() {
        val view = JSONObject("""{"rateChoices":[{"rate":5,"title":""},{"rate":6,"title":"6 Hz"}]}""")
        val choices = inspectorRateChoices(view)
        assertEquals(1, choices.size)
        assertEquals(6, choices[0].rate)
    }

    @Test
    fun `the target rate title is decoded onto the message`() {
        val view = JSONObject(
            """{"messages":[{"index":0,"id":30,"name":"ATTITUDE","targetRateTitle":"10 Hz"}]}""",
        )
        assertEquals("10 Hz", inspectorMessages(view)[0].targetRateTitle)
    }
}

class InspectorSystemTest {

    @Test
    fun `the inspector names the system its messages came from`() {
        val view = JSONObject(
            """{"kind":"object","class":"MavlinkInspector","available":true,"systemId":1,
               "messages":[]}""",
        )

        assertEquals("System 1", inspectorSystemText(view))
    }

    @Test
    fun `no system yet is no line rather than System 0`() {
        assertNull(
            inspectorSystemText(
                JSONObject("""{"kind":"object","class":"MavlinkInspector","available":true,"systemId":0}"""),
            ),
        )
        assertNull(
            inspectorSystemText(
                JSONObject("""{"kind":"object","class":"MavlinkInspector","available":false,"systemId":3}"""),
            ),
        )
        assertNull(inspectorSystemText(null))
    }
}
