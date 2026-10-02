package one.aircast.android.ui

import one.aircast.android.R
import org.junit.Assert.assertEquals
import org.junit.Test

class SetupIconTest {
    @Test
    fun aKnownPageKeepsItsIcon() = assertEquals(R.drawable.ic_gamepad, setupIcon("radio", "APMRadioComponent"))

    @Test
    fun anUnknownPageFallsBackToItsUntranslatedClass() {
        assertEquals(R.drawable.ic_warning, setupIcon(null, "APMFailsafesComponent"))
        assertEquals(R.drawable.ic_tune, setupIcon(null, "APMAdvancedTuningCopterComponent"))
        assertEquals(R.drawable.ic_build, setupIcon(null, "SomethingNewComponent"))
    }

    @Test
    fun theNoteFollowsTheMostSpecificClassToken() {
        assertEquals("Every rate and filter, per axis", setupNote("APMAdvancedTuningCopterComponent"))
        assertEquals("How it responds to the sticks", setupNote("APMTuningComponent"))
        assertEquals("Return altitude, landing speed and geofence", setupNote("APMFlightSafetyComponent"))
        assertEquals("", setupNote("SomethingNewComponent"))
    }
}
