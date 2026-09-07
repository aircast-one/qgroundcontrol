package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PlanBridgeTest {
    private fun element(
        latitude: Double = 41.0,
        longitude: Double = 44.0,
        specifies: Boolean = true,
        sequence: Int = 1,
        command: String = "Waypoint",
        current: Boolean = false,
        withCoordinate: Boolean = true,
    ): String {
        val coordinate = if (withCoordinate) {
            ""","coordinate":{"latitude":$latitude,"longitude":$longitude}"""
        } else {
            ""
        }
        return """{"specifiesCoordinate":$specifies,"sequenceNumber":$sequence,""" +
            """"commandName":"$command","isCurrentItem":$current$coordinate}"""
    }

    private fun model(vararg elements: String) =
        JSONObject("""{"kind":"object","elements":[${elements.joinToString(",")}]}""")

    @Test
    fun `items carry their sequence command and position`() {
        val items = missionItems(
            model(
                element(latitude = 41.1, longitude = 44.1, sequence = 1, command = "Takeoff"),
                element(latitude = 41.2, longitude = 44.2, sequence = 2, current = true),
            ),
        )

        assertEquals(2, items.size)
        assertEquals("Takeoff", items[0].command)
        assertEquals(1, items[0].sequence)
        assertEquals(41.1, items[0].latitude, 1e-9)
        assertTrue(items[1].current)
    }

    @Test
    fun `items without a coordinate of their own are not placed`() {
        val items = missionItems(
            model(
                element(specifies = false),
                element(withCoordinate = false),
                element(latitude = 41.3, longitude = 44.3),
            ),
        )

        assertEquals(1, items.size)
        assertEquals(41.3, items.single().latitude, 1e-9)
    }

    @Test
    fun `an unusable coordinate is not placed`() {
        val items = missionItems(model(element(latitude = 0.0, longitude = 0.0)))

        assertEquals(0, items.size)
    }

    @Test
    fun `the index survives skipped items so writes address the right one`() {
        val items = missionItems(
            model(
                element(specifies = false),
                element(latitude = 41.4, longitude = 44.4),
            ),
        )

        assertEquals(1, items.single().index)
    }

    @Test
    fun `an empty or absent model yields nothing`() {
        assertEquals(0, missionItems(null).size)
        assertEquals(0, missionItems(JSONObject("""{"kind":"object","elements":[]}""")).size)
        assertEquals(0, missionItems(JSONObject("""{"kind":"null"}""")).size)
    }

    @Test
    fun `an item reads its altitude from the fact list`() {
        val withAltitude = """{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},""" +
            """"facts":[{"name":"Altitude","value":75.0}]}"""

        assertEquals(75.0, missionItems(model(withAltitude)).single().altitude, 1e-9)
    }

    @Test
    fun `an item with no altitude fact reports it as unknown`() {
        val plain = """{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0}}"""
        val otherFact = """{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},""" +
            """"facts":[{"name":"Radius","value":5.0}]}"""

        assertTrue(missionItems(model(plain)).single().altitude.isNaN())
        assertTrue(missionItems(model(otherFact)).single().altitude.isNaN())
    }
}
