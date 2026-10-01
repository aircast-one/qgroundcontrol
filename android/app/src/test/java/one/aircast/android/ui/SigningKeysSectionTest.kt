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
}
