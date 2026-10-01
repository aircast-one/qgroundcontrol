package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TerrainProgressTest {
    @Test
    fun `progress shows while blocks are pending or once any have loaded`() {
        val loading = terrainLoad(JSONObject("""{"loaded":3,"pending":1,"fraction":0.75}"""))!!
        assertEquals(0.75f, loading.fraction)
        assertTrue(terrainShowsNow(loading))
        assertTrue("a load that starts out complete still shows, then hides", terrainShowsNow(TerrainLoad(5, 0, 1f)))
        assertFalse(terrainShowsNow(TerrainLoad(0, 0, 0f)))
        assertNull(terrainLoad(JSONObject("""{"kind":"null"}""")))
    }
}
