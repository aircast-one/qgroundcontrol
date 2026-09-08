package one.aircast.mapspike

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class InsertedAnItemTest {
    @Test
    fun `an item comes back as an object`() {
        assertTrue(insertedItem("""{"ok":true,"result":{"kind":"object","command":22}}"""))
    }

    @Test
    fun `a refused insert answers ok with a null item`() {
        assertFalse(insertedItem("""{"ok":true,"result":{"kind":"null"}}"""))
    }

    @Test
    fun `ok on its own is not evidence that anything was created`() {
        assertFalse(insertedItem("""{"ok":true}"""))
    }

    @Test
    fun `a call that did not run is not an insert`() {
        assertFalse(insertedItem("""{"ok":false}"""))
        assertFalse(insertedItem("""{"ok":false,"result":{"kind":"object"}}"""))
    }

    @Test
    fun `nothing readable is not an insert`() {
        assertFalse(insertedItem(""))
        assertFalse(insertedItem("not json"))
    }
}
