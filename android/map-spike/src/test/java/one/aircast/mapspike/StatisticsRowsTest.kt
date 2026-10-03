package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class StatisticsRowsTest {
    @Test
    fun `transects and structures list QGC's statistics rows`() {
        val survey = SurveyStats(areaText = "1.2 ha", warning = "", intervalText = "2.0 s", distanceText = "840 m", photosText = "120")
        assertEquals(listOf("Area", "Distance", "Photos", "Photo interval"), statisticsRows(survey).map { it.first })
        val structure = survey.copy(structure = StructureStats("3", "10 m", "40 m", "20 m"))
        assertEquals(listOf("Layers" to "3", "Layer height" to "10 m", "Top layer altitude" to "40 m", "Bottom layer altitude" to "20 m", "Photos" to "120", "Photo interval" to "2.0 s"), statisticsRows(structure))
    }
}
