package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SigningKeysSectionTest {
    @Test
    fun a_key_needs_a_name_and_a_long_enough_passphrase_or_64_hex() {
        assertFalse(canAddKey("", false, "longenough", 8))
        assertFalse(canAddKey("field", false, "short", 8))
        assertTrue(canAddKey("field", false, "longenough", 8))
        assertTrue(canAddKey("field", true, "ab".repeat(32), 8))
        assertFalse(canAddKey("field", true, "ab".repeat(31), 8))
        assertFalse(isHexKey("zz".repeat(32)))
    }

    @Test
    fun the_view_lists_keys_only_when_the_core_owns_them() {
        assertNull(signingKeys(JSONObject("""{"available":false}""")))
        val keys = signingKeys(JSONObject("""{"available":true,"vehicle":true,"activeKey":"None","minPassphraseLength":8,"keys":[{"name":"field","inUse":false}]}"""))!!
        assertEquals(listOf(SigningKeyRow("field", false)), keys.keys)
    }

    @Test
    fun only_the_active_key_offers_disable_and_others_say_another_key_is_active() {
        val keys = signingKeys(JSONObject("""{"available":true,"vehicle":true,"armed":false,"state":"on","linkName":"USB","activeKey":"field","keys":[{"name":"field","inUse":true,"activeOnVehicle":true},{"name":"bench","inUse":false,"activeOnVehicle":false}]}"""))!!
        assertEquals(KeyButtons(enable = false, disable = true, otherActive = false, pending = false), keyButtons(keys, keys.keys[0]))
        assertEquals(KeyButtons(enable = false, disable = false, otherActive = true, pending = false), keyButtons(keys, keys.keys[1]))
        val off = keys.copy(activeKey = "None", state = "off")
        assertTrue(keyButtons(off, off.keys[1]).enable)
    }
}
