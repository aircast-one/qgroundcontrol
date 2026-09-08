package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
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
    fun `a duplicated name says which component it came from`() {
        val a = InspectorMessage(0, 271, "CAMERA_CAPTURE_STATUS", "1.0 Hz", 9L, "p0", 100)
        val b = InspectorMessage(1, 271, "CAMERA_CAPTURE_STATUS", "1.0 Hz", 9L, "p1", 101)
        val solo = InspectorMessage(2, 30, "ATTITUDE", "5.0 Hz", 99L, "p2", 1)
        val all = listOf(a, b, solo)

        assertEquals("CAMERA_CAPTURE_STATUS  ·  comp 100", inspectorRowLabel(a, all))
        assertEquals("CAMERA_CAPTURE_STATUS  ·  comp 101", inspectorRowLabel(b, all))
        assertEquals("ATTITUDE", inspectorRowLabel(solo, all))
    }
}
