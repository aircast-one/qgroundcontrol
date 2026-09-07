package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class SurveyBridgeTest {
    private fun point(latitude: Double, longitude: Double) =
        """{"latitude":$latitude,"longitude":$longitude}"""

    private fun surveyElement(
        transects: List<String> = listOf(point(41.0, 44.0), point(41.0, 44.1)),
        area: List<String> = listOf(point(41.0, 44.0), point(41.0, 44.1), point(41.1, 44.1)),
        shots: Int = 12,
    ) = """{"isSurveyItem":true,"cameraShots":$shots,""" +
        """"visualTransectPoints":[${transects.joinToString(",")}],""" +
        """"surveyAreaPolygon":{"path":[${area.joinToString(",")}]}}"""

    private fun model(vararg elements: String) =
        JSONObject("""{"kind":"object","elements":[${elements.joinToString(",")}]}""")

    @Test
    fun `a survey carries its area transects and shot count`() {
        val found = surveys(model("""{"isSurveyItem":false}""", surveyElement()))

        assertEquals(1, found.size)
        assertEquals(1, found.single().index)
        assertEquals(3, found.single().area.size)
        assertEquals(2, found.single().transects.size)
        assertEquals(12, found.single().cameraShots)
    }

    @Test
    fun `items that are not surveys are ignored`() {
        assertEquals(0, surveys(model("""{"isSurveyItem":false}""", """{}""")).size)
    }

    @Test
    fun `a survey with neither area nor transects is not shown`() {
        val empty = """{"isSurveyItem":true,"visualTransectPoints":[],"surveyAreaPolygon":{"path":[]}}"""

        assertEquals(0, surveys(model(empty)).size)
    }

    @Test
    fun `unusable points are dropped from the area and transects`() {
        val found = surveys(
            model(
                surveyElement(
                    transects = listOf(point(41.0, 44.0), point(0.0, 0.0), point(41.2, 44.2)),
                    area = listOf(point(41.0, 44.0), point(41.0, 44.1), point(41.1, 44.1)),
                ),
            ),
        )

        assertEquals(2, found.single().transects.size)
    }

    @Test
    fun `an area needs three corners before it is drawn`() {
        val thin = surveyElement(area = listOf(point(41.0, 44.0), point(41.0, 44.1)))

        assertEquals(0, surveyAreaFeatures(surveys(model(thin))).features()?.size)
    }

    @Test
    fun `transects need two points before they are drawn`() {
        val single = surveyElement(transects = listOf(point(41.0, 44.0)))

        assertEquals(0, surveyTransectFeatures(surveys(model(single))).features()?.size)
    }

    @Test
    fun `an absent model yields nothing`() {
        assertEquals(0, surveys(null).size)
    }

    @Test
    fun `a survey reads its grid angle from the fact list`() {
        val element = """{"isSurveyItem":true,"visualTransectPoints":[${point(41.0, 44.0)},${point(41.0, 44.1)}],""" +
            """"surveyAreaPolygon":{"path":[]},"facts":[{"name":"GridAngle","value":45.0}]}"""

        assertEquals(45.0, surveys(model(element)).single().gridAngle, 1e-9)
    }

    @Test
    fun `a survey with no grid angle fact reports it as unknown`() {
        val element = """{"isSurveyItem":true,"visualTransectPoints":[${point(41.0, 44.0)},${point(41.0, 44.1)}],""" +
            """"surveyAreaPolygon":{"path":[]},"facts":[{"name":"Something","value":1.0}]}"""

        assertTrue(surveys(model(element)).single().gridAngle.isNaN())
    }
}
