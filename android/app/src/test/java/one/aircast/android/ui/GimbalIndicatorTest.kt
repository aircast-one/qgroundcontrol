package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GimbalIndicatorTest {
    @Test
    fun `the cell reads status, pitch and yaw as the toolbar does`() {
        val state = gimbalIndicator(
            JSONObject(
                """{"shown":true,"statusText":"Yaw follow","pitchText":"P: -12.3","yawText":"Y: 5.0","yawLockLabel":"Yaw Lock",""" +
                    """"retractOffered":true,"controlOffered":false,"controlLabel":"Acquire Control","gimbals":[{"name":"Gimbal 1","managerCompid":1,"deviceId":154,"active":true}]}""",
            ),
        )!!
        assertEquals("Yaw follow · P: -12.3 · Y: 5.0", gimbalCellText(state))
        val two = state.copy(gimbals = state.gimbals + GimbalChoice("1-155", 1, 155, false))
        assertEquals("with several gimbals the toolbar names the active one", "Gimbal 1 · Yaw follow · P: -12.3 · Y: 5.0", gimbalCellText(two))
        assertTrue(state.retractOffered)
        assertEquals(154, state.gimbals[0].deviceId)
        assertNull(gimbalIndicator(JSONObject("""{"shown":false}""")))
    }

    @Test
    fun `a tap maps to QGC's cooked screen fractions`() {
        val centre = screenFraction(50f, 50f, 100, 100)
        assertEquals(0f, centre.first, 0f)
        assertEquals(0f, centre.second, 0f)
        val corner = screenFraction(100f, 0f, 100, 100)
        assertEquals(1f, corner.first, 0f)
        assertEquals(1f, corner.second, 0f)
        assertEquals(-1f, screenFraction(0f, 100f, 100, 100).second, 0f)
        assertEquals(OnScreenGimbal(true, true), onScreenGimbal(JSONObject("""{"shown":true,"onScreen":{"enabled":true,"clickAndDrag":true}}""")))
        assertEquals("with on-screen control off the video still aims by drag, as CameraAimArea", OnScreenGimbal(false, true), onScreenGimbal(JSONObject("""{"shown":true,"onScreen":{"enabled":false,"clickAndDrag":true}}""")))
    }
}
