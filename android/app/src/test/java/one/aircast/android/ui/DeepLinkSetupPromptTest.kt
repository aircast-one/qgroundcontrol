package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DeepLinkSetupPromptTest {
    @Test
    fun `a device link is shown only while the core holds one`() {
        assertNull(deepLinkSetup(JSONObject("""{"show":false,"host":null}""")))
        assertNull(deepLinkSetup(null))
        val shown = deepLinkSetup(JSONObject("""{"show":true,"title":"Set up from this device?","text":"from 10.0.0.5","accept":"Set up","decline":"Ignore"}"""))
        assertEquals(DeepLinkSetup("Set up from this device?", "from 10.0.0.5", "Set up", "Ignore"), shown)
    }
}
