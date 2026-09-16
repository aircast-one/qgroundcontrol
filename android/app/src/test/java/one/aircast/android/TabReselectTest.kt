package one.aircast.android

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TabReselectTest {

    @Test
    fun `re-selecting Analyze drops its sub-page`() {
        assertTrue(reselectClearsAnalyze(Tab.Analyze, Tab.Analyze))
    }

    @Test
    fun `re-selecting another tab leaves the Analyze sub-page alone`() {
        Tab.entries.filter { it != Tab.Analyze }.forEach { entry ->
            assertFalse(
                "re-tapping $entry cleared a sub-page belonging to a tab the operator is not in - " +
                    "invisible only while switching tabs already loses your place, and a silent " +
                    "trap the moment that is fixed",
                reselectClearsAnalyze(entry, entry),
            )
        }
    }

    @Test
    fun `switching to a different tab is not a re-selection at all`() {
        assertFalse(reselectClearsAnalyze(Tab.Settings, Tab.Analyze))
        assertFalse(reselectClearsAnalyze(Tab.Analyze, Tab.Settings))
    }
}
