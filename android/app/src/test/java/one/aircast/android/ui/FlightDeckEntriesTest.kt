package one.aircast.android.ui

import kotlinx.coroutines.CoroutineScope
import org.junit.Assert.assertEquals
import org.junit.Test
import kotlin.coroutines.EmptyCoroutineContext

class FlightDeckEntriesTest {
    private fun offer(id: String, state: String = "ready", carriesValue: Boolean = true) =
        GuidedOffer(id = id, title = "", offer = state, reason = "", prompt = "", destructive = false, carriesValue = carriesValue)

    private fun deck(offers: List<GuidedOffer>, armed: Boolean = false, openChecklist: (() -> Unit)? = null, opened: MutableList<String> = mutableListOf(), readiness: Readiness? = null) =
        FlightDeckContext(
            offers = offers.associateBy { it.id },
            armed = armed,
            scope = CoroutineScope(EmptyCoroutineContext),
            confirm = {},
            openValue = { opened += it.offerId },
            report = {},
            withdraw = {},
            openChecklist = openChecklist,
            readiness = readiness,
        )

    @Test
    fun `the deck shows the offers the vehicle shows, and the checklist only on the ground when offered`() {
        val offers = listOf(offer("arm"), offer("takeoff"), offer("rtl", state = "hidden"), offer("land", state = "disabled"))
        assertEquals(listOf("arm", "takeoff", "land"), flightDeckEntries(deck(offers)).map { it.id })
        assertEquals(listOf("arm", "takeoff", "land", CHECKLIST), flightDeckEntries(deck(offers, openChecklist = {})).map { it.id })
        assertEquals(listOf("takeoff", "land"), flightDeckEntries(deck(offers, armed = true, openChecklist = {})).map { it.id })
    }

    @Test
    fun `value actions open the shared guided value flow`() {
        val opened = mutableListOf<String>()
        val entries = flightDeckEntries(deck(listOf(offer("takeoff"), offer("changeSpeed"), offer("changeAltitude"), offer(PAUSE)), opened = opened))
        listOf("takeoff", "changeSpeed", "changeAltitude", PAUSE).forEach { id -> entries.first { it.id == id }.onClick() }
        assertEquals(listOf("takeoff", "changeSpeed", "changeAltitude", PAUSE), opened)
    }

    @Test
    fun `a vehicle that is not fully ready cannot be held straight into the air, the tap opens the warning first`() {
        val ready = flightDeckEntries(deck(listOf(offer("takeoff")))).first { it.id == "takeoff" }
        assertEquals(HOLD_TO_TAKE_OFF to true, ready.label to (ready.onHold != null))
        val warned = flightDeckEntries(deck(listOf(offer("takeoff")), readiness = Readiness("GPS turned off.", blocks = false))).first { it.id == "takeoff" }
        assertEquals(TAKE_OFF to false, warned.label to (warned.onHold != null))
    }
}
