package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MissionAltitudeFrameTest {
    @Test
    fun `the mission frame is read from the plan view`() {
        assertEquals(1, globalAltitudeFrame(JSONObject("""{"globalAltitudeFrame":1}""")))
        assertNull(globalAltitudeFrame(JSONObject("""{"globalAltitudeFrame":null}""")))
    }
}
