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
         "job":{"phase":"writing","busy":true,"cancellable":true,"done":1,"total":4,"percent":25.0,"speed":12300000,"speedText":"12 MB/s","remainingText":"about 4 min left","error":null,"hostname":"falcon-01"},
         "firstProblem":{"field":"ssid","message":"Enter the WiFi network name, or choose no WiFi."},
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
        assertEquals("Downloading Aircast OS… 25%", state.downloadText)
        assertEquals("running", state.downloadState)
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
        assertEquals("Tap OK on the prompt to let Aircast use the card reader", flasherCardText(JSONObject("{\"state\":\"waiting\"}")))
        assertEquals("no card in the reader", flasherCardText(JSONObject("{\"state\":\"failed\",\"error\":\"no card in the reader\"}")))
        assertEquals("", flasherCardText(JSONObject("{\"state\":\"none\"}")))
        assertEquals("Download failed: offline", flasherDownloadText(JSONObject("{\"state\":\"failed\",\"error\":\"offline\"}")))
        assertEquals("Aircast OS is downloaded", flasherDownloadText(JSONObject("{\"state\":\"ready\",\"imageSizeText\":\"3.9 GB\"}")))
    }

    @Test
    fun every_job_phase_has_a_title() {
        listOf("downloading", "preparing", "writing", "verifying", "customizing", "done", "failed", "cancelled").forEach { phase ->
            assert(flasherPhaseLabel(phase).isNotBlank()) { phase }
        }
        assertEquals("", flasherPhaseLabel("idle"))
    }

    private fun state(phase: String = "idle", busy: Boolean = false, card: String = "none", disks: List<FlasherDisk> = listOf(FlasherDisk("/dev/bus/usb/001/004", "Reader"))) =
        flasherState(view)!!.copy(phase = phase, busy = busy, cardState = card, disks = disks)

    @Test
    fun a_running_or_finished_job_always_shows_the_write_step() {
        assertEquals(FlasherStep.Write, flasherStepFor(state(phase = "writing", busy = true)))
        assertEquals(FlasherStep.Write, flasherStepFor(state(phase = "done")))
        assertNull(flasherStepFor(state()))
        assertNull(flasherStepFor(null))
    }

    @Test
    fun a_single_reader_opens_by_itself_but_a_choice_is_left_to_the_user() {
        assertEquals("/dev/bus/usb/001/004", readerToOpen(state()))
        assertNull(readerToOpen(state(card = "opening")))
        assertNull(readerToOpen(state(disks = listOf(FlasherDisk("a", "A"), FlasherDisk("b", "B")))))
        assertNull(readerToOpen(state(disks = emptyList())))
        assertNull(readerToOpen(state(phase = "done")))
    }

    @Test
    fun problems_show_once_a_field_is_touched_or_continue_is_pressed() {
        val problems = mapOf("ssid" to "Enter the WiFi network name, or choose no WiFi.", "hostname" to "Use lowercase letters")
        assertEquals(emptyMap<String, String>(), visibleProblems(problems, emptySet(), showAll = false))
        assertEquals(setOf("hostname"), visibleProblems(problems, setOf("hostname"), showAll = false).keys)
        assertEquals(problems, visibleProblems(problems, emptySet(), showAll = true))
        assertEquals(false, setupComplete(problems))
        assertEquals(true, setupComplete(emptyMap()))
    }

    @Test
    fun a_phone_without_a_key_signs_in_with_a_password() {
        assertEquals(FLASHER_PASSWORD_MODE, defaultSshMode(FlasherForm()))
        assertNull(defaultSshMode(FlasherForm(authorizedKey = "ssh-ed25519 AAAA")))
        assertNull(defaultSshMode(FlasherForm(sshMode = "disabled")))
    }

    @Test
    fun mobile_data_is_warned_about_only_before_the_download_starts() {
        val release = FlasherRelease("v0.3.5", "Aircast OS v0.3.5", "2026-08-22", "577 MB", recommended = true)
        assertEquals("Aircast OS is a 577 MB download and this phone is on mobile data.", meteredNote(release, "idle", metered = true))
        assertNull(meteredNote(release, "idle", metered = false))
        assertNull(meteredNote(release, "running", metered = true))
        assertNull(meteredNote(release, "ready", metered = true))
        assertNull(meteredNote(null, "idle", metered = true))
    }

    @Test
    fun the_phones_wifi_name_is_cleaned_of_quotes_and_placeholders() {
        assertEquals("field-net", phoneSsid("\"field-net\""))
        assertNull(phoneSsid("<unknown ssid>"))
        assertNull(phoneSsid("\"\""))
        assertNull(phoneSsid(null))
    }

    @Test
    fun the_first_problem_is_named_by_its_field_and_opens_more_options_when_it_lives_there() {
        val parsed = flasherState(view)!!
        assertEquals("WiFi network: Enter the WiFi network name, or choose no WiFi.", problemNote(parsed))
        assertEquals(false, problemInMoreOptions(parsed))
        assertEquals(true, problemInMoreOptions(parsed.copy(firstProblemField = "controlServer")))
        assertEquals("", problemNote(parsed.copy(firstProblem = "")))
        assertEquals("about 4 min left", parsed.remainingText)
    }

    @Test
    fun internal_channels_stay_out_of_customer_builds() {
        assertEquals(false, showsChannels(debug = false, channel = "stable"))
        assertEquals(true, showsChannels(debug = true, channel = "stable"))
        assertEquals(true, showsChannels(debug = false, channel = "staging"))
    }

    @Test
    fun the_screen_stays_on_while_downloading_or_writing() {
        assertEquals(true, keepsScreenOn(state(phase = "writing", busy = true)))
        assertEquals(true, keepsScreenOn(flasherState(view)!!.copy(busy = false, phase = "idle", downloadState = "running")))
        assertEquals(false, keepsScreenOn(flasherState(view)!!.copy(busy = false, phase = "idle", downloadState = "ready")))
        assertEquals(false, keepsScreenOn(null))
    }

    @Test
    fun the_ending_says_what_happens_next() {
        val steps = nextSteps("field-net", "falcon-01", noWifi = false)
        assertEquals(3, steps.size)
        assert(steps[1].contains("joins field-net")) { steps[1] }
        assert(steps[2].contains("http://falcon-01.local")) { steps[2] }
        assert(nextSteps("", "falcon-01", noWifi = true)[1].contains("Ethernet or cellular"))
    }

    @Test
    fun only_an_open_network_counts_as_unsecured() {
        assertEquals(false, securedFrom(android.net.wifi.WifiInfo.SECURITY_TYPE_OPEN))
        assertEquals(true, securedFrom(android.net.wifi.WifiInfo.SECURITY_TYPE_PSK))
        assertNull(securedFrom(android.net.wifi.WifiInfo.SECURITY_TYPE_UNKNOWN))
    }

    @Test
    fun back_returns_to_the_previous_step_unless_writing_or_finished() {
        assertEquals(FlasherStep.Setup, previousStep(FlasherStep.Write, busy = false, phase = "idle"))
        assertEquals(FlasherStep.Card, previousStep(FlasherStep.Setup, busy = false, phase = "idle"))
        assertNull(previousStep(FlasherStep.Card, busy = false, phase = "idle"))
        assertNull(previousStep(FlasherStep.Write, busy = true, phase = "writing"))
        assertNull(previousStep(FlasherStep.Write, busy = false, phase = "done"))
    }
}
