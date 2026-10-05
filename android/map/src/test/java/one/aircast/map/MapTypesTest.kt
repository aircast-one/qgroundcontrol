package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MapTypesTest {
    @Test
    fun `the plan map menu lists the provider's types and marks the current one`() {
        val read = mapTypes(JSONObject("""{"current":"Satellite","types":["Street Map","Satellite"],"path":"settings.flightMapSettings.mapType.rawValue"}"""))
        assertEquals(MapTypes("Satellite", listOf("Street Map", "Satellite"), "settings.flightMapSettings.mapType.rawValue"), read)
        assertNull(mapTypes(JSONObject("""{"kind":"null"}""")))
    }
}
