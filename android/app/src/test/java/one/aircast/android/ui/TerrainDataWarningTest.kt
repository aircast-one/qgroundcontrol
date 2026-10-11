package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TerrainDataWarningTest {
    @Test
    fun `the warning watches for blocks still pending, and a load that finished or never started is quiet`() {
        val loading = terrainLoad(JSONObject("""{"loaded":3,"pending":1,"fraction":0.75}"""))
        assertEquals(TerrainLoad(3, 1), loading)
        assertTrue(loading.waiting)
        assertFalse("everything loaded is nothing to warn about", terrainLoad(JSONObject("""{"loaded":5,"pending":0}""")).waiting)
        assertFalse("no vehicle, no terrain view", terrainLoad(JSONObject("""{"kind":"null"}""")).waiting)
        assertFalse(terrainLoad(null).waiting)
    }
}
