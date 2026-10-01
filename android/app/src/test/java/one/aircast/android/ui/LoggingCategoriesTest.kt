package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class LoggingCategoriesTest {
    private val served = JSONObject(
        """{"active":["groundstation::hub"],"categories":[
            {"name":"groundstation","shortName":"groundstation","depth":0,"enabled":false},
            {"name":"groundstation::hub","shortName":"hub","depth":1,"enabled":true},
            {"name":"ureq","shortName":"ureq","depth":0,"enabled":false}]}""",
    )

    @Test
    fun `the tree reads with its depths and the active list`() {
        val read = logCategories(served)!!
        assertEquals(listOf("groundstation::hub"), read.active)
        assertEquals(listOf(0, 1, 0), read.categories.map { it.depth })
        assertEquals(listOf(false, true, false), read.categories.map { it.enabled })
    }

    @Test
    fun `searching matches anywhere in the full name, ignoring case`() {
        val read = logCategories(served)!!
        assertEquals(listOf("groundstation::hub"), filteredCategories(read.categories, "HUB").map { it.name })
        assertEquals(3, filteredCategories(read.categories, "  ").size)
    }
}
