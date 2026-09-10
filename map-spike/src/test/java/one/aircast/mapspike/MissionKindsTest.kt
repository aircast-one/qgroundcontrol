package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MissionKindsTest {
    @Test
    fun `an unknown enabled does not refuse`() {
        val kind = missionKind(JSONObject("""{"id":"takeoff","enabled":null,"disabledReason":""}"""))
        assertNull(kind!!.enabled)
        assertNull(kindRefusal(kind))
    }

    @Test
    fun `a missing enabled key does not refuse`() {
        val kind = missionKind(JSONObject("""{"id":"land"}"""))
        assertNull(kind!!.enabled)
        assertNull(kindRefusal(kind))
    }

    @Test
    fun `an explicit false refuses in the core's words`() {
        val kind = missionKind(
            JSONObject("""{"id":"takeoff","enabled":false,"disabledReason":"The mission already takes off before this point."}""")
        )
        assertEquals("The mission already takes off before this point.", kindRefusal(kind))
    }

    @Test
    fun `a false with no reason still refuses`() {
        val kind = missionKind(JSONObject("""{"id":"survey","enabled":false,"disabledReason":""}"""))
        assertEquals("That item does not belong here in the mission.", kindRefusal(kind))
    }

    @Test
    fun `an enabled kind does not refuse`() {
        assertNull(kindRefusal(missionKind(JSONObject("""{"id":"land","enabled":true}"""))))
    }

    @Test
    fun `a view with no id is not a kind`() {
        assertNull(missionKind(JSONObject("""{"enabled":false,"disabledReason":"no"}""")))
        assertNull(missionKind(null))
        assertNull(kindRefusal(null))
    }
}
