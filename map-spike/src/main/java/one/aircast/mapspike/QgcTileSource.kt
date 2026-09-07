package one.aircast.mapspike

import android.content.Context
import okhttp3.Interceptor
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import org.maplibre.android.module.http.HttpRequestUtil

private val TILE_PATH = Regex("""^/(\d+)/(\d+)/(\d+)$""")

fun qgcRasterStyle(): String = """
{
  "version": 8,
  "glyphs": "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf",
  "sources": {
    "qgc": {
      "type": "raster",
      "tiles": ["$QGC_TILE_URL"],
      "tileSize": 256,
      "maxzoom": 20,
      "attribution": "QGroundControl tile cache"
    }
  },
  "layers": [ { "id": "qgc", "type": "raster", "source": "qgc" } ]
}
"""

class QgcTileInterceptor(
    private val cache: QgcTileCache,
    private val prefix: String,
    format: String,
) : Interceptor {

    private val mediaType = when (format.lowercase()) {
        "png" -> "image/png"
        else -> "image/jpeg"
    }.toMediaType()

    override fun intercept(chain: Interceptor.Chain): Response {
        val request = chain.request()
        if (request.url.host != QGC_TILE_HOST) {
            return chain.proceed(request)
        }

        val match = TILE_PATH.find(request.url.encodedPath)
        val tile = match?.let { result ->
            val (z, x, y) = result.destructured
            cache.tile(prefix, z.toInt(), x.toInt(), y.toInt())
        }

        return Response.Builder()
            .request(request)
            .protocol(Protocol.HTTP_1_1)
            .code(if (tile == null) 404 else 200)
            .message(if (tile == null) "No cached tile" else "OK")
            .body((tile ?: ByteArray(0)).toResponseBody(mediaType))
            .build()
    }
}

// Returns the style to use: QGC's own cache when it holds tiles, OSM otherwise.
fun installQgcTileSource(context: Context): String {
    val cache = QgcTileCache.open(context) ?: return OSM_RASTER_STYLE
    val provider = cache.providers().firstOrNull { it.count > 0 }
    if (provider == null) {
        cache.close()
        return OSM_RASTER_STYLE
    }

    HttpRequestUtil.setOkHttpClient(
        OkHttpClient.Builder()
            .addInterceptor(QgcTileInterceptor(cache, provider.prefix, provider.format))
            .build(),
    )
    return qgcRasterStyle()
}
