package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FlasherScreenTest {
    private val view = JSONObject(
        """
        {"available":true,"channel":"stable","channels":["stable","staging","development"],
         "releases":{"state":"ready","error":null,"items":[{"version":"v0.3.5","label":"Aircast OS v0.3.5","prerelease":false,"published":"2026-08-22","size":576627168,"sizeText":"577 MB"}]},
         "selected":"v0.3.5",
         "form":{"hostname":"falcon-01","ssid":"field-net","wifiPassword":"","noWifi":false,"country":"GE","sshMode":"key-only","authorizedKey":"","devicePassword":"","authKey":"","controlServer":""},
         "problems":{"ssid":"Enter the WiFi network name, or choose no WiFi."},
         "keyIdentity":{"algorithm":"ssh-ed25519","comment":"operator@base","fingerprint":"SHA256:abc"},
         "summary":[{"label":"Hostname","value":"falcon-01.local","warn":false},{"label":"Device access","value":"Image default (pi / raspberry)","warn":true}],
         "disks":[{"id":"/dev/bus/usb/001/004","name":"Generic SD Reader"}],
         "card":{"state":"ready","id":"/dev/bus/usb/001/004","label":"Generic SD","capacity":16000000000,"capacityText":"16.0 GB"},
         "download":{"state":"running","version":"v0.3.5","done":10,"total":40,"percent":25.0},
         "job":{"phase":"writing","busy":true,"cancellable":true,"done":1,"total":4,"percent":25.0,"speed":12300000,"speedText":"12 MB/s","error":null,"hostname":"falcon-01"},
         "canStart":false,"blocked":"A card is being written"}
        """.trimIndent(),
    )

    @Test
    fun the_core_view_becomes_screen_state() {
        val state = flasherState(view)!!
        assertEquals(listOf(FlasherRelease("v0.3.5", "Aircast OS v0.3.5", "2026-08-22", "577 MB")), state.releases)
        assertEquals("falcon-01", state.form.hostname)
        assertEquals("GE", state.form.country)
        assertEquals("Enter the WiFi network name, or choose no WiFi.", state.problems["ssid"])
        assertEquals("ssh-ed25519 SHA256:abc operator@base", state.keyIdentity)
        assertEquals(FlasherSummaryRow("Device access", "Image default (pi / raspberry)", true), state.summary[1])
        assertEquals("Generic SD · 16.0 GB", state.cardText)
        assertEquals("Downloading… 25%", state.downloadText)
        assertEquals(25f, state.percent)
        assertEquals("A card is being written", state.blocked)
    }

    @Test
    fun a_missing_or_foreign_view_is_no_state() {
        assertNull(flasherState(null))
        assertNull(flasherState(JSONObject("{\"ports\":[]}")))
    }

    @Test
    fun card_and_download_states_read_as_sentences() {
        assertEquals("Allow access to the card reader in the dialog", flasherCardText(JSONObject("{\"state\":\"waiting\"}")))
        assertEquals("no card in the reader", flasherCardText(JSONObject("{\"state\":\"failed\",\"error\":\"no card in the reader\"}")))
        assertEquals("", flasherCardText(JSONObject("{\"state\":\"none\"}")))
        assertEquals("Download failed: offline", flasherDownloadText(JSONObject("{\"state\":\"failed\",\"error\":\"offline\"}")))
        assertEquals("Downloaded · 3.9 GB when written", flasherDownloadText(JSONObject("{\"state\":\"ready\",\"imageSizeText\":\"3.9 GB\"}")))
    }

    @Test
    fun every_job_phase_has_a_title() {
        listOf("downloading", "preparing", "writing", "verifying", "customizing", "done", "failed", "cancelled").forEach { phase ->
            assert(flasherPhaseLabel(phase).isNotBlank()) { phase }
        }
        assertEquals("", flasherPhaseLabel("idle"))
    }
}
