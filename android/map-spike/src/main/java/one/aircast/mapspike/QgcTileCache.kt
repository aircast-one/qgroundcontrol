package one.aircast.mapspike

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import java.io.File

const val QGC_TILE_HOST = "qgc.tiles"
const val QGC_TILE_URL = "https://$QGC_TILE_HOST/{z}/{x}/{y}"

private const val CACHE_DIR = "QGCMapCache300"
private const val CACHE_FILE = "qgcMapCache.db"

data class TileProvider(val prefix: String, val type: String, val count: Int, val format: String)

fun tileHash(prefix: String, z: Int, x: Int, y: Int): String =
    prefix + "%08d".format(x) + "%08d".format(y) + "%03d".format(z)

fun qgcCacheFile(context: Context): File =
    File(File(context.filesDir, CACHE_DIR), CACHE_FILE)

class QgcTileCache(private val database: SQLiteDatabase) : AutoCloseable {

    fun providers(): List<TileProvider> = runCatching { readProviders() }.getOrDefault(emptyList())

    private fun readProviders(): List<TileProvider> {
        val sql = "SELECT substr(hash,1,10) AS prefix, type, format, COUNT(*) AS n " +
            "FROM Tiles GROUP BY prefix, type, format ORDER BY n DESC"
        return database.rawQuery(sql, null).use { cursor ->
            buildList {
                while (cursor.moveToNext()) {
                    add(
                        TileProvider(
                            prefix = cursor.getString(0),
                            type = cursor.getString(1) ?: "",
                            format = cursor.getString(2) ?: "jpg",
                            count = cursor.getInt(3),
                        ),
                    )
                }
            }
        }
    }

    fun tile(prefix: String, z: Int, x: Int, y: Int): ByteArray? = runCatching {
        val hash = tileHash(prefix, z, x, y)
        database.rawQuery("SELECT tile FROM Tiles WHERE hash = ? LIMIT 1", arrayOf(hash))
            .use { cursor -> if (cursor.moveToFirst()) cursor.getBlob(0) else null }
    }.getOrNull()

    override fun close() {
        database.close()
    }

    companion object {
        fun open(context: Context): QgcTileCache? {
            val file = qgcCacheFile(context)
            if (!file.isFile) {
                return null
            }
            return runCatching {
                SQLiteDatabase.openDatabase(file.path, null, SQLiteDatabase.OPEN_READONLY)
            }.map { QgcTileCache(it) }.getOrNull()
        }
    }
}
