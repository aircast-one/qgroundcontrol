package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AdvancedUiTest {
    @Test
    fun `advanced mode is on until the core says otherwise like QGCCorePlugin`() {
        assertTrue(advancedUi(null).shown)
        val off = advancedUi(JSONObject("""{"shown":false,"title":"Advanced Mode","confirmation":"WARNING"}"""))
        assertFalse(off.shown)
        assertEquals("Advanced Mode", off.title)
        assertEquals("WARNING", off.confirmation)
    }
}
