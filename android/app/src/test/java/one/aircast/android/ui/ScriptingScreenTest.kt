package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ScriptingScreenTest {
    @Test
    fun the_page_lists_the_scripts_the_core_found() {
        val page = scripting(JSONObject("""{"available":true,"enabled":true,"enable":null,"scripts":["hello.lua"],"busy":true,"progress":0.5,"status":"Upload succeeded: /APM/scripts/hello.lua","unsupportedText":"x"}"""))!!
        assertEquals(listOf("hello.lua"), page.scripts)
        assertTrue(page.busy)
        assertEquals(0.5f, page.progress)
        assertFalse(scripting(JSONObject("""{"available":false}"""))!!.available)
    }
}
