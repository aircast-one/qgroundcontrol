package one.aircast.android.ui

import one.aircast.mapspike.TrackPoint
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class OfflineMapsTest {
    @Test
    fun `the visible corners become the region the url factory counts`() {
        val region = offlineRegion(
            listOf(TrackPoint(47.40, 8.50), TrackPoint(47.40, 8.60), TrackPoint(47.35, 8.60), TrackPoint(47.35, 8.50)),
        )!!
        assertEquals(OfflineRegion(west = 8.50, north = 47.40, east = 8.60, south = 47.35), region)
        assertEquals(
            "view.offlineMaps(Google Satellite,8.5000000,47.4000000,8.6000000,47.3500000,13,19)",
            offlineMapsPath("Google Satellite", region, 13, 19),
        )
        assertEquals("view.offlineMaps", offlineMapsPath("Google Satellite", null, 13, 19))
        assertNull(offlineRegion(emptyList()))
    }

    @Test
    fun `sets and the estimate read from the view`() {
        val read = offlineMaps(
            JSONObject(
                """{"class":"OfflineMaps","available":true,"sets":[{"id":1,"name":"System Wide Tile Cache","defaultSet":true,""" +
                    """"downloadStatus":"1.0MB","downloading":false,"complete":true}],"mapList":["Bing Road"],"uniqueName":"Tile Set 001",""" +
                    """"takenNames":["Default Tile Set"],"estimate":{"tileCountText":"1,234","tileSizeText":"5.0MB","tooMany":false}}""",
            ),
        )!!
        assertEquals("System Wide Tile Cache", read.sets.single().name)
        assertEquals("Tile Set 001", read.uniqueName)
        assertEquals(OfflineEstimate("1,234", "5.0MB", false), read.estimate)
    }

    @Test
    fun `ok renames only to a new non-blank name`() {
        org.junit.Assert.assertEquals("Valley", renameWanted("Set 1", " Valley "))
        org.junit.Assert.assertNull(renameWanted("Set 1", "Set 1"))
        org.junit.Assert.assertNull(renameWanted("Set 1", "  "))
    }
}
