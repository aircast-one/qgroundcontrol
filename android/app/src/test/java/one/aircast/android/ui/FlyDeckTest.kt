package one.aircast.android.ui

import androidx.compose.ui.unit.dp
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
    fun `a drop sits on the deck beside the flight actions in flight and never takes the primary slot`() {
        assertEquals(
            listOf(PAUSE to false, "rtl" to true, "land" to false, RELEASE to false),
            deckIds(setOf("arm", PAUSE, "rtl", "land", RELEASE), armed = true),
        )
    }

    @Test
    fun `a crowded deck drops the slide instruction so five buttons fit a phone`() {
        assertEquals(listOf("Slide to land", "Land", "Drop 2"), listOf(deckLabel("Slide to land", 4), deckLabel("Slide to land", 5), deckLabel("Slide to drop 2", 5)))
    }

    @Test
    fun `on the ground the deck is for taking off and the drop stays in the payload sheet`() {
        assertEquals(listOf("takeoff" to true), deckIds(setOf("arm", "takeoff", RELEASE), armed = false))
    }

    private val flying = deckIds(setOf("arm", PAUSE, "rtl", "land", RELEASE), armed = true)

    @Test
    fun `on a phone every flight action stays on the deck and the camera moves above it`() {
        assertEquals(listOf(BottomDeck(flying, false), BottomDeck(flying, false)), listOf(358.dp, 328.dp).map { bottomDeck(flying, it, 112.dp) })
    }

    @Test
    fun `a deck with room keeps the camera between the actions`() {
        assertEquals(BottomDeck(flying, true), bottomDeck(flying, 700.dp, 112.dp))
        assertEquals(BottomDeck(flying, true), bottomDeck(flying, 328.dp, 0.dp))
    }

    @Test
    fun `when the actions run out of room the least critical move into More first`() {
        assertEquals(
            listOf(listOf("rtl", "land", RELEASE), listOf("rtl", "land"), listOf("rtl"), emptyList(), emptyList()),
            listOf(288.dp, 222.dp, 156.dp, 90.dp, 40.dp).map { width -> bottomDeck(flying, width, 112.dp).deck.map { it.first } },
        )
    }

    @Test
    fun `the ground deck gives up the checklist before taking off`() {
        assertEquals(listOf("takeoff"), bottomDeck(deckIds(setOf(CHECKLIST, "arm", "takeoff"), armed = false), 140.dp, 0.dp).deck.map { it.first })
    }

    @Test
    fun `a deck not measured yet shows every action`() {
        assertEquals(BottomDeck(flying, true), bottomDeck(flying, 0.dp, 112.dp))
        assertEquals(RailDeck(flying, true), railDeck(flying, 0.dp))
    }

    @Test
    fun `a sideways phone drops the rail labels before it drops an action`() {
        val noDrop = flying.filter { it.first != RELEASE }
        assertEquals(RailDeck(flying, false), railDeck(flying, 286.dp))
        assertEquals(RailDeck(noDrop, true), railDeck(noDrop, 286.dp))
        assertEquals(listOf("rtl", "land", RELEASE), railDeck(flying, 271.dp).deck.map { it.first })
    }

    @Test
    fun `sliding across most of the button confirms as surely as holding it`() {
        org.junit.Assert.assertEquals(0.5f, slideProgress(300f, 1000), 1e-6f)
        org.junit.Assert.assertEquals(1f, slideProgress(600f, 1000), 1e-6f)
        org.junit.Assert.assertEquals(1f, slideProgress(900f, 1000), 1e-6f)
        org.junit.Assert.assertEquals("sliding back the other way never confirms", 0f, slideProgress(-200f, 1000), 1e-6f)
    }
}
