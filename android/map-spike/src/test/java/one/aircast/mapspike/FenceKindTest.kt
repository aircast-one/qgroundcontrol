package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FenceKindTest {

    private val view = JSONObject(
        """{"kind":"object","polygons":[
             {"index":0,"inclusion":false,"kindText":"Keep-out polygon",
              "detailText":"3 vertices \u00b7 2.5 ha","vertices":[
               {"latitude":41.0,"longitude":44.0},{"latitude":41.1,"longitude":44.0},
               {"latitude":41.1,"longitude":44.1}]}],
           "circles":[
             {"index":0,"inclusion":true,"kindText":"Keep-in circle",
              "centre":{"latitude":41.0,"longitude":44.0},"radius":150.0}]}""",
    )

    private val polygons = fencePolygons(view)
    private val circles = fenceCircles(view)

    @Test
    fun `a selected fence says whether it keeps the vehicle in or out`() {
        assertEquals("Keep-in circle", fenceDetail(MapHit.Circle(0), polygons, circles))
        assertEquals("Keep-in circle", fenceDetail(MapHit.CircleCentre(0), polygons, circles))
    }

    @Test
    fun `a fence says what it is and how big, not one or the other`() {
        assertEquals(
            "Keep-out polygon \u00b7 3 vertices \u00b7 2.5 ha",
            fenceDetail(MapHit.FenceVertex(0, 0), polygons, circles),
        )
    }

    @Test
    fun `a circle drawn as a polygon keeps the wording it was given`() {
        assertEquals("Keep-in circle", circlesAsPolygons(circles).single().kindText)
    }

    @Test
    fun `a selection that is not a fence says nothing`() {
        assertNull(fenceDetail(MapHit.Waypoint(0), polygons, circles))
        assertNull(fenceDetail(null, polygons, circles))
        assertNull(fenceDetail(MapHit.FenceVertex(9, 0), polygons, circles))
    }
}
