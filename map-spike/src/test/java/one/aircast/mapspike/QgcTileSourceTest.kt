package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class QgcTileSourceTest {

    @Test
    fun `a tile path names the tile`() {
        assertEquals(Triple(14, 9876, 6543), tileCoords("/14/9876/6543"))
    }

    @Test
    fun `anything that is not a tile path names no tile`() {
        assertNull(tileCoords("/14/9876"))
        assertNull(tileCoords("/style.json"))
        assertNull(tileCoords("/14/9876/6543.png"))
    }

    @Test
    fun `a tile the cache does not hold is asked of the same source the app falls back to`() {
        assertEquals(
            "https://tile.openstreetmap.org/14/9876/6543.png",
            uncachedTileUrl("/14/9876/6543"),
        )
    }

    @Test
    fun `the fallback url is the one the wholesale style uses, so the two cannot drift`() {
        assertEquals(
            OSM_TILE_URL.replace("{z}", "3").replace("{x}", "2").replace("{y}", "1"),
            uncachedTileUrl("/3/2/1"),
        )
        assert(OSM_RASTER_STYLE.contains(OSM_TILE_URL))
    }

    @Test
    fun `a request that is not a tile at all gets no fallback`() {
        assertNull(uncachedTileUrl("/style.json"))
    }
}
