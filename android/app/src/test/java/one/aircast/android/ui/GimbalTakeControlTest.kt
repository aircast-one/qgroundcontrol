package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GimbalTakeControlTest {
    @Test
    fun `another holder of the gimbal asks to take control instead of failing quietly`() {
        gimbalAsksForControl.value = false
        assertNull(gimbalRefusal(JSONObject("""{"ok":false,"refusal":"othersHaveControl","reason":"Command not sent. Another user has control of the gimbal."}""")))
        assertTrue(gimbalAsksForControl.value)
        assertEquals("The gimbal is not ready yet.", gimbalRefusal(JSONObject("""{"ok":false,"refusal":"notReady","reason":"The gimbal is not ready yet."}""")))
        gimbalAsksForControl.value = false
    }
}
