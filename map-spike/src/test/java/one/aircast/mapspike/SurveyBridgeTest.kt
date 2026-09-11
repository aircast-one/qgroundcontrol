package one.aircast.mapspike

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class SurveyBridgeTest {
    private fun point(latitude: Double, longitude: Double) =
        """{"latitude":$latitude,"longitude":$longitude}"""

    private val threeCorners = listOf(point(41.0, 44.0), point(41.0, 44.1), point(41.1, 44.1))

    private fun surveyElement(
        transects: List<String> = listOf(point(41.0, 44.0), point(41.0, 44.1)),
        shots: Int = 12,
    ) = """{"isSurveyItem":true,"cameraShots":$shots,""" +
        """"visualTransectPoints":[${transects.joinToString(",")}]}"""

    private fun model(vararg elements: String) =
        JSONObject("""{"kind":"object","elements":[${elements.joinToString(",")}]}""")

    private fun areaRead(corners: List<String> = threeCorners): (Int) -> JSONArray? =
        { JSONArray("[${corners.joinToString(",")}]") }

    private fun found(json: JSONObject?, corners: List<String> = threeCorners) =
        SurveyBridge.surveysFrom(json, areaRead(corners))

    @Test
    fun `a survey carries its area transects and shot count`() {
        val surveys = found(model("""{"isSurveyItem":false}""", surveyElement()))

        assertEquals(1, surveys.size)
        assertEquals(1, surveys.single().index)
        assertEquals(3, surveys.single().area.size)
        assertEquals(2, surveys.single().transects.size)
        assertEquals(12, surveys.single().cameraShots)
    }

    @Test
    fun `the area comes from its own read not from the listed element`() {
        val element = """{"isSurveyItem":true,"visualTransectPoints":[],""" +
            """"surveyAreaPolygon":{"path":[${threeCorners.joinToString(",")}]}}"""

        assertEquals(0, SurveyBridge.surveysFrom(model(element)) { null }.size)
    }

    @Test
    fun `items that are not surveys are ignored`() {
        assertEquals(0, found(model("""{"isSurveyItem":false}""", """{}""")).size)
    }

    @Test
    fun `a survey with neither area nor transects is not shown`() {
        val empty = """{"isSurveyItem":true,"visualTransectPoints":[]}"""

        assertEquals(0, SurveyBridge.surveysFrom(model(empty)) { JSONArray("[]") }.size)
    }

    @Test
    fun `unusable points are dropped from the area and transects`() {
        val surveys = found(
            model(
                surveyElement(
                    transects = listOf(point(41.0, 44.0), point(0.0, 0.0), point(41.2, 44.2)),
                ),
            ),
        )

        assertEquals(2, surveys.single().transects.size)
    }

    @Test
    fun `an area needs three corners before it is drawn`() {
        val thin = listOf(point(41.0, 44.0), point(41.0, 44.1))

        assertEquals(0, surveyAreaFeatures(found(model(surveyElement()), thin)).features()?.size)
    }

    @Test
    fun `transects need two points before they are drawn`() {
        val single = surveyElement(transects = listOf(point(41.0, 44.0)))

        assertEquals(0, surveyTransectFeatures(found(model(single))).features()?.size)
    }

    @Test
    fun `an absent model yields nothing`() {
        assertEquals(0, found(null).size)
    }

    @Test
    fun `a survey reads its grid angle from the fact list`() {
        val element = """{"isSurveyItem":true,"visualTransectPoints":[${point(41.0, 44.0)},${point(41.0, 44.1)}],""" +
            """"facts":[{"name":"GridAngle","value":45.0}]}"""

        assertEquals(45.0, found(model(element)).single().gridAngle, 1e-9)
    }

    @Test
    fun `a survey with no grid angle fact reports it as unknown`() {
        val element = """{"isSurveyItem":true,"visualTransectPoints":[${point(41.0, 44.0)},${point(41.0, 44.1)}],""" +
            """"facts":[{"name":"Something","value":1.0}]}"""

        assertTrue(found(model(element)).single().gridAngle.isNaN())
    }
}
