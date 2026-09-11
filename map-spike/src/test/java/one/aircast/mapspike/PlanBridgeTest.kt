package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PlanBridgeTest {
    private fun element(
        latitude: Double = 41.0,
        longitude: Double = 44.0,
        specifies: Boolean = true,
        sequence: Int = 1,
        command: String = "Waypoint",
        selected: Boolean = false,
        withCoordinate: Boolean = true,
    ): String {
        val coordinate = if (withCoordinate && specifies) {
            ""","coordinate":{"latitude":$latitude,"longitude":$longitude}"""
        } else {
            ""
        }
        return """{"flownLeg":$specifies,"sequence":$sequence,""" +
            """"name":"$command","selected":$selected$coordinate}"""
    }

    private fun model(vararg elements: String) =
        JSONObject("""{"kind":"object","items":[${elements.joinToString(",")}]}""")

    @Test
    fun `everything after the item that ends the route is flagged, read from the plan itself`() {
        val plan = JSONObject(
            """{"kind":"object","items":[""" +
                """{"sequence":0,"name":"Mission Start","flownLeg":false,""" +
                """"coordinate":{"latitude":41.0,"longitude":44.0}},""" +
                """{"sequence":1,"name":"Waypoint","flownLeg":true,""" +
                """"coordinate":{"latitude":41.1,"longitude":44.1}},""" +
                """{"sequence":2,"name":"Land","flownLeg":true,"endsRoute":true,"command":21,""" +
                """"coordinate":{"latitude":41.2,"longitude":44.2}},""" +
                """{"sequence":3,"name":"Waypoint","flownLeg":true,""" +
                """"coordinate":{"latitude":41.3,"longitude":44.3}}]}""",
        )

        val items = missionItems(plan)

        assertEquals(
            listOf(false, false, false, true),
            items.map { it.afterRouteEnds },
        )
        assertFalse("the land itself is the end, not past it", items[2].afterRouteEnds)
        assertFalse("and the map draws no leg to what follows", items[3].routed)
    }

    @Test
    fun `a plan that never ends its route strands nothing`() {
        val items = missionItems(model(element(sequence = 1), element(sequence = 2)))

        assertEquals(listOf(false, false), items.map { it.afterRouteEnds })
    }

    @Test
    fun `items carry their sequence command and position`() {
        val items = missionItems(
            model(
                element(latitude = 41.1, longitude = 44.1, sequence = 1, command = "Takeoff"),
                element(latitude = 41.2, longitude = 44.2, sequence = 2, selected = true),
            ),
        )

        assertEquals(2, items.size)
        assertEquals("Takeoff", items[0].command)
        assertEquals(1, items[0].sequence)
        assertEquals(41.1, items[0].latitude, 1e-9)
        assertTrue("the editor selection, which is what the core calls it", items[1].selected)
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
    fun `an item reads the altitude the core resolved for it`() {
        val withAltitude = """{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0},""" +
            """"altitude":75.0}"""

        assertEquals(75.0, missionItems(model(withAltitude)).single().altitude, 1e-9)
    }

    @Test
    fun `an item the core gave no altitude reports it as unknown`() {
        val plain = """{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0}}"""
        val nulled = """{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0},""" +
            """"altitude":null}"""

        assertTrue(missionItems(model(plain)).single().altitude.isNaN())
        assertTrue(missionItems(model(nulled)).single().altitude.isNaN())
    }

    @Test
    fun `an item reads where it leaves as well as where it starts`() {
        val withExit = """{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},""" +
            """"exitCoordinate":{"latitude":41.5,"longitude":44.5}}"""

        assertEquals(TrackPoint(41.5, 44.5), missionItems(model(withExit)).single().exit)
    }

    @Test
    fun `an exit that repeats the entry or cannot be plotted is not carried`() {
        val same = """{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},""" +
            """"exitCoordinate":{"latitude":41.0,"longitude":44.0}}"""
        val unusable = """{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},""" +
            """"exitCoordinate":{"latitude":0.0,"longitude":0.0}}"""

        assertEquals(null, missionItems(model(same)).single().exit)
        assertEquals(null, missionItems(model(unusable)).single().exit)
    }

    @Test
    fun `an item says whether the waypoint line goes through it`() {
        val roi = """{"specifiesCoordinate":true,"isStandaloneCoordinate":true,""" +
            """"coordinate":{"latitude":41.0,"longitude":44.0}}"""

        assertFalse(missionItems(model(roi)).single().routed)
        assertTrue(missionItems(model(element())).single().routed)
    }
}
