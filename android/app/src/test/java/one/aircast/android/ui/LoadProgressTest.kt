package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class LoadProgressTest {
    @Test
    fun `the chip shows load progress until the initial connect completes`() {
        assertEquals(0.4f, loadingProgress(JSONObject("""{"initialConnectComplete":false,"loadProgress":0.4}""")))
        assertNull(loadingProgress(JSONObject("""{"initialConnectComplete":true,"loadProgress":1.0}""")))
        assertNull(loadingProgress(JSONObject("{}")))
    }
}
