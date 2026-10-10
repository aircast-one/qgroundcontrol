package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class FlyDeckTest {
    @Test
    fun groundOffersChecklistAndTakeoffAsPrimary() {
        assertEquals(
            listOf(CHECKLIST to false, "takeoff" to true),
            deckIds(setOf(CHECKLIST, "arm", "takeoff", "changeSpeed"), armed = false),
        )
    }

    @Test
    fun groundWithoutTakeoffArmsAsPrimary() {
        assertEquals(listOf("arm" to true), deckIds(setOf("arm"), armed = false))
    }

    @Test
    fun flyingOffersPauseReturnLandWithReturnPrimary() {
        assertEquals(
            listOf(PAUSE to false, "rtl" to true, "land" to false),
            deckIds(setOf("arm", PAUSE, "rtl", "land", "changeAltitude"), armed = true),
        )
    }

    @Test
    fun armedOnTheGroundKeepsTheGroundDeck() {
        assertEquals(listOf("takeoff" to true), deckIds(setOf("arm", "takeoff"), armed = true))
    }

    @Test
    fun `sliding across most of the button confirms as surely as holding it`() {
        org.junit.Assert.assertEquals(0.5f, slideProgress(300f, 1000), 1e-6f)
        org.junit.Assert.assertEquals(1f, slideProgress(600f, 1000), 1e-6f)
        org.junit.Assert.assertEquals(1f, slideProgress(900f, 1000), 1e-6f)
        org.junit.Assert.assertEquals("sliding back the other way never confirms", 0f, slideProgress(-200f, 1000), 1e-6f)
    }
}
