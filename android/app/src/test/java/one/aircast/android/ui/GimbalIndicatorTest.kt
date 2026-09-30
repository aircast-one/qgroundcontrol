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
        assertTrue(state.retractOffered)
        assertEquals(154, state.gimbals[0].deviceId)
        assertNull(gimbalIndicator(JSONObject("""{"shown":false}""")))
    }
}
