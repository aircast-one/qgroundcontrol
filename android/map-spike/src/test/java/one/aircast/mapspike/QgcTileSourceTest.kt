package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class QgcTileSourceTest {

    private fun pathFor(mapType: String, z: Int, x: Int, y: Int): String =
        qgcTileUrl(mapType).substringAfter("https://$QGC_TILE_HOST")
            .replace("{z}", "$z").replace("{x}", "$x").replace("{y}", "$y")

    @Test
    fun `a tile path names the map type and the tile`() {
        assertEquals(TileAddress("Bing Hybrid", 14, 9876, 6543), tileAddress(pathFor("Bing Hybrid", 14, 9876, 6543)))
        assertEquals(TileAddress("Google Street Map", 0, 0, 0), tileAddress(pathFor("Google Street Map", 0, 0, 0)))
    }

    @Test
    fun `anything that is not a tile path names no tile`() {
        assertNull(tileAddress("/14/9876/6543"))
        assertNull(tileAddress("/style.json"))
        assertNull(tileAddress("/Bing/14/9876/6543.png"))
    }

    @Test
    fun `a tile nothing else serves falls back to OpenStreetMap`() {
        assertEquals("https://tile.openstreetmap.org/14/9876/6543.png", osmTileUrl(TileAddress("Bing Hybrid", 14, 9876, 6543)))
        assertTrue(OSM_RASTER_STYLE.contains(OSM_TILE_URL))
    }

    @Test
    fun `the map type is the provider and type the settings name`() {
        assertEquals("Bing Hybrid", mapTypeName("Bing", "Hybrid"))
        assertEquals("Bing", mapTypeName("Bing", ""))
        assertTrue(qgcRasterStyle("Esri World Satellite").contains(qgcTileUrl("Esri World Satellite")))
    }
}
