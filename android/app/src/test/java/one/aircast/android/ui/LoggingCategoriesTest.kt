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
        assertEquals("QSortFilterProxyModel::setFilterFixedString matches the text as typed", emptyList<String>(), filteredCategories(read.categories, "  ").map { it.name })
    }

    @Test
    fun `the tree starts collapsed, lists children under their parent and opens one level per expanded parent like TreeView`() {
        val read = logCategories(
            JSONObject(
                """{"active":[],"categories":[
                    {"name":"a","shortName":"a","depth":0,"enabled":false},
                    {"name":"a1","shortName":"a1","depth":0,"enabled":false},
                    {"name":"a::b","shortName":"b","depth":1,"enabled":false},
                    {"name":"a::b::c","shortName":"c","depth":2,"enabled":false},
                    {"name":"a1::x","shortName":"x","depth":1,"enabled":false}]}""",
            ),
        )!!
        assertEquals(setOf("a", "a::b", "a1"), parentNames(read.categories))
        assertEquals("a1", parentOf(read.categories, read.categories.last())?.name)
        assertEquals(listOf("a", "a1"), shownInTree(read.categories, emptySet()).map { it.name })
        assertEquals(listOf("a", "a::b", "a1"), shownInTree(read.categories, setOf("a")).map { it.name })
        assertEquals("a child stays hidden while its parent is collapsed", listOf("a", "a1"), shownInTree(read.categories, setOf("a::b")).map { it.name })
        assertEquals(listOf("a", "a::b", "a::b::c", "a1"), shownInTree(read.categories, setOf("a", "a::b")).map { it.name })
    }
}
