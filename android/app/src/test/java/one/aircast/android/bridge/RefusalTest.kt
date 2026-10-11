package one.aircast.android.bridge

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RefusalTest {

    @Test
    fun `a failed answer says why, and names what refused when it gives no reason`() {
        assertNull(refusal(JSONObject("""{"ok":true}""")))
        assertEquals("Not armed", refusal(JSONObject("""{"ok":false,"reason":"Not armed"}""")))
        assertEquals("The vehicle refused.", refusal(JSONObject("""{"ok":false}""")))
        assertEquals("The vehicle did not answer.", refusal(null))
        assertEquals("The link refused.", refusal(JSONObject("""{"ok":false}"""), "The link"))
    }
}
