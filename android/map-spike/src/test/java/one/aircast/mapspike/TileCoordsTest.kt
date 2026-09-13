package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class TileCoordsTest {
    private fun pathFromTemplate(z: Int, x: Int, y: Int): String =
        QGC_TILE_URL.substringAfter("https://$QGC_TILE_HOST")
            .replace("{z}", "$z")
            .replace("{x}", "$x")
            .replace("{y}", "$y")

    @Test
    fun `a path built from the url template parses back to its numbers`() {
        assertEquals(Triple(14, 8703, 5723), tileCoords(pathFromTemplate(14, 8703, 5723)))
    }

    @Test
    fun `zero is a real tile coordinate`() {
        assertEquals(Triple(0, 0, 0), tileCoords(pathFromTemplate(0, 0, 0)))
    }

    @Test
    fun `anything that is not three numbers is not a tile`() {
        assertNull(tileCoords("/14/8703"))
        assertNull(tileCoords("/14/8703/5723/1"))
        assertNull(tileCoords("/14/8703/5723.png"))
        assertNull(tileCoords("/a/b/c"))
        assertNull(tileCoords(""))
    }
}
