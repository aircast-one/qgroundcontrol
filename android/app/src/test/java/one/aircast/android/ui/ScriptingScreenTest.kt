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

    @Test
    fun a_refused_transfer_opens_the_lua_dialog_with_the_core_text_or_the_fallback() {
        assertEquals(ScriptRefusal("Lua Delete", "Another FTP operation is in progress"), scriptRefusal("Lua Delete", "Another FTP operation is in progress", "Delete failed"))
        assertEquals(ScriptRefusal("Lua Upload", "Upload failed"), scriptRefusal("Lua Upload", "", "Upload failed"))
        assertEquals(null, scriptRefusal("Lua Download", null, "Download failed"))
    }
}
