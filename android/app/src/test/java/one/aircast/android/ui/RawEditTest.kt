package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RawEditTest {
    @Test
    fun `only simple items offer show all values`() {
        assertNull(rawEdit(JSONObject("""{"simple":false}""")))
        assertEquals(RawEdit(on = true, friendlyAllowed = false), rawEdit(JSONObject("""{"simple":true,"rawEdit":true,"friendlyEditAllowed":false}""")))
    }

    @Test
    fun `an item that cannot be shown friendly stays raw`() {
        assertEquals(RAW_EDIT_STUCK, rawEditRefusal(RawEdit(on = true, friendlyAllowed = false)))
        assertNull(rawEditRefusal(RawEdit(on = true, friendlyAllowed = true)))
        assertNull(rawEditRefusal(RawEdit(on = false, friendlyAllowed = true)))
    }

    @Test
    fun `an unfinished pattern shape shows its help instead of its settings`() {
        assertEquals("Use the Polygon Tools", areaHelp(JSONObject("""{"areaHelp":"Use the Polygon Tools"}""")))
        assertNull(areaHelp(JSONObject("""{"areaHelp":null}""")))
    }

    @Test
    fun `a pattern offers its start corner to rotate`() {
        assertEquals(EntryPoint("Start from", "top left", "p.rotateEntryPoint"), entryPoint(JSONObject("""{"entryPoint":{"label":"Start from","value":"top left","path":"p.rotateEntryPoint"}}""")))
        assertNull(entryPoint(JSONObject("""{"entryPoint":null}""")))
    }
}
