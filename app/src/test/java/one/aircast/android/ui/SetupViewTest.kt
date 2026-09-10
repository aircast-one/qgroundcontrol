package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class SetupViewTest {
    @Test
    fun `a page that watches but cannot finish says so before the tap`() {
        assertEquals("Finish on desktop", setupBadge(native = true, completes = false))
    }

    @Test
    fun `a page that can finish the job keeps the plain badge`() {
        assertEquals("Needs setup", setupBadge(native = true, completes = true))
    }

    @Test
    fun `a component with no native page at all is not promised a desktop finish`() {
        assertEquals("Needs setup", setupBadge(native = false, completes = false))
    }

    @Test
    fun `completes is read from the field the core publishes`() {
        val view = JSONObject(
            """{"groups":[{"title":"Setup","pages":[
                {"name":"Radio","native":true,"completes":false},
                {"name":"Safety","native":true,"completes":true}]}]}""",
        )
        assertEquals(false, setupPage(view, "Radio")?.completes)
        assertEquals(true, setupPage(view, "Safety")?.completes)
    }

    @Test
    fun `a payload using an invented name for completes reads as cannot finish`() {
        val view = JSONObject(
            """{"groups":[{"title":"S","pages":[{"name":"Radio","native":true,"canComplete":true}]}]}""",
        )
        assertEquals(false, setupPage(view, "Radio")?.completes)
    }
}
