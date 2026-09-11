package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class SurveyBridgeTest {
    private fun point(latitude: Double, longitude: Double) =
        """{"latitude":$latitude,"longitude":$longitude}"""

    private val threeCorners = listOf(point(41.0, 44.0), point(41.0, 44.1), point(41.1, 44.1))

    private fun survey(
        corners: List<String> = threeCorners,
        transects: List<String> = listOf(point(41.0, 44.0), point(41.0, 44.1)),
        shots: Int = 12,
    ) = """{"kind":"survey","cameraShots":$shots,"geometry":{"shape":"area",""" +
        """"vertices":[${corners.joinToString(",")}],""" +
        """"transects":[${transects.joinToString(",")}]}}"""

    private fun plan(vararg items: String) =
        JSONObject("""{"kind":"object","items":[${items.joinToString(",")}]}""")

    private fun found(json: JSONObject?) = SurveyBridge.surveysFrom(json)

    @Test
    fun `a survey carries its area transects and shot count`() {
        val surveys = found(plan("""{"kind":"settings"}""", survey()))

        assertEquals(1, surveys.size)
        assertEquals(1, surveys.single().index)
        assertEquals(3, surveys.single().area.size)
        assertEquals(2, surveys.single().transects.size)
        assertEquals(12, surveys.single().cameraShots)
    }

    @Test
    fun `items that are not surveys are ignored`() {
        assertEquals(0, found(plan("""{"kind":"waypoint"}""", """{"kind":"takeoff"}""")).size)
    }

    @Test
    fun `a survey the core gave no geometry is not shown`() {
        assertEquals(0, found(plan("""{"kind":"survey","cameraShots":3}""")).size)
    }

    @Test
    fun `transects that have not arrived yet leave the area drawable`() {
        val surveys = found(plan(survey(transects = emptyList())))

        assertEquals(3, surveys.single().area.size)
        assertEquals(0, surveys.single().transects.size)
    }

    @Test
    fun `unusable points are dropped from the area and transects`() {
        val surveys = found(
            plan(
                survey(
                    transects = listOf(point(41.0, 44.0), point(0.0, 0.0), point(41.2, 44.2)),
                ),
            ),
        )

        assertEquals(2, surveys.single().transects.size)
    }

    @Test
    fun `an area needs three corners before it is drawn`() {
        val thin = survey(corners = listOf(point(41.0, 44.0), point(41.0, 44.1)))

        assertEquals(0, surveyAreaFeatures(found(plan(thin))).features()?.size)
    }

    @Test
    fun `transects need two points before they are drawn`() {
        val single = survey(transects = listOf(point(41.0, 44.0)))

        assertEquals(0, surveyTransectFeatures(found(plan(single))).features()?.size)
    }

    @Test
    fun `an absent plan yields nothing`() {
        assertEquals(0, found(null).size)
    }
}
