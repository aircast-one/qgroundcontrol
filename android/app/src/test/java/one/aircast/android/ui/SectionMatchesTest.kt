package one.aircast.android.ui

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SectionMatchesTest {
    private val gcs = ParameterRows("Ground Station Failsafe", emptyList(), "", keywords = listOf("heartbeat", "fs_gcs_timeout"))

    @Test
    fun matchesTitleOrKeywordIgnoringCase() {
        assertTrue(sectionMatches(gcs, "  station "))
        assertTrue(sectionMatches(gcs, "HEARTBEAT"))
        assertTrue(sectionMatches(gcs, ""))
        assertFalse(sectionMatches(gcs, "battery"))
    }
}
