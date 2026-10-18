package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceEngineeringTest {
    private fun field(name: String, value: String = "", secret: Boolean = false) = """{"name":"$name","label":"$name","value":"$value","placeholder":"","secret":$secret}"""

    private val served =
        """
        {
          "backend": {"status": {"level": 3, "text": "mediamtx"}, "text": "Video is carried by mediamtx.", "switch": "engine", "label": "Switch to the engine", "confirm": "Restart?", "success": "Switching."},
          "publish": {
            "status": {"level": 3, "text": "awaiting cloud connect"},
            "awaitingCloud": true,
            "mode": "aircast",
            "modes": [
              {"value": "aircast", "title": "Aircast cloud", "desc": "Managed.", "fields": [], "clears": ["token"], "submit": "Test sign-in", "unsaved": "Unsaved change — press Test sign-in to apply."},
              {"value": "oauth", "title": "Custom OAuth server", "desc": "OAuth.", "fields": ["issuer", "clientId", "clientSecret", "tokenUrl", "scope"], "clears": ["token"], "submit": "Save", "unsaved": "Unsaved."},
              {"value": "token", "title": "Access token (WHIP)", "desc": "Token.", "fields": ["token"], "clears": [], "submit": "Save", "unsaved": "Unsaved."},
              {"value": "open", "title": "None — open server", "desc": "Open.", "fields": [], "clears": ["issuer", "clientId", "clientSecret", "tokenUrl", "scope", "token"], "submit": "Save", "unsaved": "Unsaved.", "confirm": "Clear all stored sign-in settings, including the cloud issuer?"}
            ],
            "fields": [${field("issuer", "https://api.dev.aircast.one")}, ${field("clientId", "drone")}, ${field("clientSecret", "s3cr3t", true)}, ${field("tokenUrl")}, ${field("scope")}, ${field("token", "old-token", true)}],
            "environment": "https://api.dev.aircast.one",
            "verdict": {"level": 3, "text": "Not tested yet — press Test sign-in."}
          },
          "diagnostics": {"status": {"level": 3, "text": "off"}, "hint": "Stream logs.", "endpoint": "otel.aircast.one:443", "token": "", "tokenLabel": "Bearer token", "streaming": false, "canStream": false, "state": "Save the endpoint first.", "confirm": "Stream?"}
        }
        """

    private val view = parseEngineering(JSONObject(served))
    private val publish = checkNotNull(view).publish

    @Test
    fun `reads the engineering page the device serves`() {
        assertEquals("engine", view?.backend?.switchTo)
        assertEquals("aircast", publish.mode)
        assertTrue(publish.awaitingCloud)
        assertEquals(true, publish.fields.firstOrNull { it.name == "clientSecret" }?.secret)
        assertFalse(view?.diagnostics?.canStream ?: true)
        assertNull(parseEngineering(JSONObject("""{"publish":{}}""")))
    }

    @Test
    fun `each sign-in method shows only its own fields`() {
        assertEquals(emptyList<SignInField>(), signInFields(publish, "aircast"))
        assertEquals(listOf("issuer", "clientId", "clientSecret", "tokenUrl", "scope"), signInFields(publish, "oauth").map { it.name })
        assertEquals(listOf("token"), signInFields(publish, "token").map { it.name })
        assertEquals("Test sign-in", modeNamed(publish, "aircast")?.submit)
        assertNull(modeNamed(publish, "nowhere"))
    }

    @Test
    fun `saving sends every credential and empties exactly what the method clears`() {
        assertEquals(
            mapOf("issuer" to "https://api.dev.aircast.one", "clientId" to "drone", "clientSecret" to "s3cr3t", "tokenUrl" to "", "scope" to "", "token" to ""),
            signInBody(publish, "aircast", emptyMap()),
        )
        assertEquals(
            mapOf("issuer" to "https://api.dev.aircast.one", "clientId" to "drone", "clientSecret" to "s3cr3t", "tokenUrl" to "", "scope" to "", "token" to "whip-token-abc"),
            signInBody(publish, "token", mapOf("token" to "whip-token-abc")),
        )
        assertEquals(
            mapOf("issuer" to "", "clientId" to "", "clientSecret" to "", "tokenUrl" to "", "scope" to "", "token" to ""),
            signInBody(publish, "open", mapOf("issuer" to "typed")),
        )
        val oauth = signInBody(publish, "oauth", mapOf("clientSecret" to "new"))
        assertEquals("new", oauth["clientSecret"])
        assertEquals("", oauth["token"])
    }

    @Test
    fun `the editor starts over only when what is saved changes`() {
        assertEquals(signInKey(publish), signInKey(checkNotNull(parseEngineering(JSONObject(served))).publish))
        assertNotEquals(signInKey(publish), signInKey(publish.copy(mode = "token")))
        assertNotEquals(signInKey(publish), signInKey(publish.copy(fields = publish.fields.map { if (it.name == "token") it.copy(value = "new") else it })))
    }
}
