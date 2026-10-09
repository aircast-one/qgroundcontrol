package one.aircast.map

import android.content.Context
import okhttp3.Interceptor
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import org.json.JSONObject
import org.maplibre.android.module.http.HttpRequestUtil
import org.mavlink.qgroundcontrol.QGCBridge
import java.net.URLDecoder
import java.net.URLEncoder

private val TILE_PATH = Regex("""^/([^/]+)/(\d+)/(\d+)/(\d+)$""")
private val PNG_MAGIC = byteArrayOf(0x89.toByte(), 0x50, 0x4E, 0x47)
private const val MAP_PROVIDER = "settings.flightMapSettings.mapProvider.rawValue"
private const val MAP_TYPE = "settings.flightMapSettings.mapType.rawValue"

object MapTileHost {
    @Volatile
    var fetch: ((String, Int, Int, Int) -> ByteArray?)? = null
}

internal data class TileAddress(val mapType: String, val z: Int, val x: Int, val y: Int)

internal fun tileAddress(path: String): TileAddress? =
    TILE_PATH.find(path)?.destructured?.let { (type, z, x, y) ->
        runCatching { TileAddress(URLDecoder.decode(type, "UTF-8"), z.toInt(), x.toInt(), y.toInt()) }.getOrNull()
    }

internal fun qgcTileUrl(mapType: String): String =
    "https://$QGC_TILE_HOST/${URLEncoder.encode(mapType, "UTF-8")}/{z}/{x}/{y}"

internal fun osmTileUrl(address: TileAddress): String =
    OSM_TILE_URL.replace("{z}", "${address.z}").replace("{x}", "${address.x}").replace("{y}", "${address.y}")

internal fun mapTypeName(provider: String, type: String): String =
    listOf(provider, type).filter { it.isNotBlank() }.joinToString(" ")

private fun setting(path: String): String =
    runCatching { JSONObject(QGCBridge.get(path)).optString("value") }.getOrDefault("")

fun currentMapType(): String = mapTypeName(setting(MAP_PROVIDER), setting(MAP_TYPE))

fun tileMimeType(tile: ByteArray): String =
    if (tile.size >= PNG_MAGIC.size && PNG_MAGIC.indices.all { tile[it] == PNG_MAGIC[it] }) "image/png" else "image/jpeg"

fun coreTile(encodedPath: String): ByteArray? =
    tileAddress(encodedPath)?.let { address -> runCatching { MapTileHost.fetch?.invoke(address.mapType, address.x, address.y, address.z) }.getOrNull() }

fun qgcRasterStyle(mapType: String): String = """
{
  "version": 8,
  "glyphs": "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf",
  "sources": {
    "qgc": {
      "type": "raster",
      "tiles": ["${qgcTileUrl(mapType)}"],
      "tileSize": 256,
      "maxzoom": 20,
      "attribution": "$mapType"
    }
  },
  "layers": [ { "id": "qgc", "type": "raster", "source": "qgc" } ]
}
"""

class QgcTileInterceptor(
    private val cache: QgcTileCache?,
    private val cachedPrefix: String?,
) : Interceptor {

    private fun served(request: okhttp3.Request, tile: ByteArray): Response =
        Response.Builder()
            .request(request)
            .protocol(Protocol.HTTP_1_1)
            .code(200)
            .message("OK")
            .body(tile.toResponseBody(tileMimeType(tile).toMediaType()))
            .build()

    override fun intercept(chain: Interceptor.Chain): Response {
        val request = chain.request()
        if (request.url.host != QGC_TILE_HOST) {
            return chain.proceed(request)
        }
        val address = tileAddress(request.url.encodedPath)
            ?: return Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(404).message("Not a tile")
                .body(ByteArray(0).toResponseBody("image/png".toMediaType())).build()

        val fromCore = coreTile(request.url.encodedPath)
        val tile = fromCore ?: cachedPrefix?.let { prefix -> runCatching { cache?.tile(prefix, address.z, address.x, address.y) }.getOrNull() }
        if (tile != null) {
            return served(request, tile)
        }
        return runCatching { chain.proceed(request.newBuilder().url(osmTileUrl(address)).build()) }.getOrNull()
            ?: Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(404).message("No tile")
                .body(ByteArray(0).toResponseBody("image/png".toMediaType())).build()
    }
}

fun installQgcTileSource(context: Context) {
    val cache = QgcTileCache.open(context)
    val prefix = cache?.providers()?.firstOrNull { it.count > 0 }?.prefix
    HttpRequestUtil.setOkHttpClient(
        OkHttpClient.Builder()
            .addInterceptor(QgcTileInterceptor(cache, prefix))
            .build(),
    )
}
