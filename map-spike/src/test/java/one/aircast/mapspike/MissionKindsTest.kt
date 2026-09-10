package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class MissionKindsTest {
    @Test
    fun `an accepted insert carries no complaint`() {
        val done = insertOutcome(JSONObject("""{"ok":true,"inserted":"survey","atSequence":4}"""))
        assertTrue(done.ok)
        assertEquals("", done.reason)
    }

    @Test
    fun `a refusal is shown in the core's words`() {
        val refused = insertOutcome(
            JSONObject("""{"ok":false,"reason":"The mission already takes off before this point.","refused":"takeoff"}"""),
        )
        assertFalse(refused.ok)
        assertEquals("The mission already takes off before this point.", refused.reason)
    }

    @Test
    fun `a kind the catalogue does not hold says so rather than going quiet`() {
        val unknown = insertOutcome(
            JSONObject("""{"ok":false,"unknown":"corkscrew","reason":"the core has no corkscrew in its catalogue"}"""),
        )
        assertFalse(unknown.ok)
        assertEquals("the core has no corkscrew in its catalogue", unknown.reason)
    }

    @Test
    fun `a refusal with no reason still says something`() {
        assertEquals("The plan did not answer.", insertOutcome(JSONObject("""{"ok":false}""")).reason)
        assertEquals("The plan did not answer.", insertOutcome(null).reason)
    }
}
