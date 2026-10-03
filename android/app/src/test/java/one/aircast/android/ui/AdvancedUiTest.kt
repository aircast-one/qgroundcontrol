package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AdvancedUiTest {
    @Test
    fun `advanced mode is on until the core says otherwise like QGCCorePlugin`() {
        assertTrue(advancedUiShown(null))
        assertFalse(advancedUiShown(JSONObject("""{"shown":false}""")))
    }
}
