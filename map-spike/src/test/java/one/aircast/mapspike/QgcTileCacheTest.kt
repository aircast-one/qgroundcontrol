package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class QgcTileCacheTest {
    // Taken from a real qgcMapCache.db row written by QGroundControl:
    // 0308415137 00000000 00000002 002 -> Bing Hybrid, x=0, y=2, z=2
    private val bingPrefix = "0308415137"

    @Test
    fun `a hash matches the key QGroundControl wrote`() {
        assertEquals("03084151370000000000000002002", tileHash(bingPrefix, z = 2, x = 0, y = 2))
        assertEquals("03084151370000000300000000002", tileHash(bingPrefix, z = 2, x = 3, y = 0))
        assertEquals("03084151370000000100000001002", tileHash(bingPrefix, z = 2, x = 1, y = 1))
    }

    @Test
    fun `a hash is always the full 29 characters`() {
        assertEquals(29, tileHash(bingPrefix, z = 0, x = 0, y = 0).length)
        assertEquals(29, tileHash(bingPrefix, z = 20, x = 1048575, y = 1048575).length)
    }

    @Test
    fun `each field keeps its own width`() {
        val hash = tileHash(bingPrefix, z = 17, x = 119846, y = 79314)

        assertEquals(bingPrefix, hash.substring(0, 10))
        assertEquals("00119846", hash.substring(10, 18))
        assertEquals("00079314", hash.substring(18, 26))
        assertEquals("017", hash.substring(26, 29))
    }
}
