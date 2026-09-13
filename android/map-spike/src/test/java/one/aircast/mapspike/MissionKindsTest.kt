package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
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

class InsertedIndexTest {
    @Test
    fun `a successful insert says where the item landed, so the next one can follow it`() {
        val answered = JSONObject("""{"ok":true,"inserted":"waypoint","index":3}""")

        assertEquals(3, insertOutcome(answered).index)
    }

    @Test
    fun `an answer without an index does not invent one`() {
        assertNull(insertOutcome(JSONObject("""{"ok":true}""")).index)
    }

    @Test
    fun `a refusal carries no index`() {
        assertNull(insertOutcome(JSONObject("""{"ok":false,"reason":"no"}""")).index)
    }
}
