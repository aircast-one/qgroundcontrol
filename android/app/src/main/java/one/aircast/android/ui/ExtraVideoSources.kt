package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val EXTRA_SOURCES_FACT = "settings.videoSettings.extraVideoSources"
internal const val VIDEO_SOURCE_FACT = "settings.videoSettings.videoSource"
internal const val VIDEO_DISABLED = "Video Stream Disabled"

private val URL_SOURCES = setOf(
    "UDP h.264 Video Stream",
    "UDP h.265 Video Stream",
    "MPEG-TS Video Stream",
    "RTSP Video Stream",
    "TCP-MPEG2 Video Stream",
    "WebRTC Video Stream",
)

internal data class ExtraVideoSource(val name: String, val source: String, val url: String)

private fun objects(json: String?): List<JSONObject> {
    val array = runCatching { JSONArray(json.orEmpty()) }.getOrNull() ?: return emptyList()
    return (0 until array.length()).map { array.optJSONObject(it) ?: JSONObject() }
}

private fun encode(entries: List<JSONObject>): String =
    JSONArray().also { array -> entries.forEach { array.put(it) } }.toString()

private fun patch(entry: JSONObject, name: String, source: String, url: String): JSONObject =
    JSONObject(entry.toString()).apply {
        put("name", name)
        put("source", source)
        put("url", url)
    }

internal fun extraSources(json: String?): List<ExtraVideoSource> = objects(json).map { entry ->
    ExtraVideoSource(
        name = entry.optText("name"),
        source = entry.optText("source"),
        url = entry.optText("url"),
    )
}

internal fun extraSourceAdded(json: String?, name: String, source: String, url: String): String =
    encode(objects(json) + patch(JSONObject(), name, source, url))

internal fun extraSourceRemoved(json: String?, index: Int): String =
    encode(objects(json).filterIndexed { at, _ -> at != index })

internal fun extraSourcePatched(
    json: String?,
    index: Int,
    name: String,
    source: String,
    url: String,
): String = encode(
    objects(json).mapIndexed { at, entry ->
        if (at == index) patch(entry, name, source, url) else entry
    },
)

internal fun sourceNeedsUrl(source: String): Boolean = source in URL_SOURCES

internal fun extraSourceProblem(source: String, url: String): String? = when {
    source.isBlank() || source == VIDEO_DISABLED -> "Pick the kind of stream this camera sends."
    sourceNeedsUrl(source) && url.isBlank() -> "This kind of stream needs an address."
    url.contains("://") && sourceNeedsUrl(source) ->
        "Leave the scheme off - QGroundControl adds it, and a doubled one fails to resolve."
    else -> null
}

internal fun extraSourceSummary(entry: ExtraVideoSource): String = when {
    entry.source.isBlank() -> "Not set up"
    !sourceNeedsUrl(entry.source) -> entry.source
    entry.url.isBlank() -> "${entry.source} · no address"
    else -> "${entry.source} · ${entry.url}"
}

internal data class VideoKind(val raw: String, val label: String)

internal fun videoKinds(enumValues: List<String>, enumStrings: List<String>): List<VideoKind> =
    enumValues.indices
        .map { VideoKind(enumValues[it], enumStrings.getOrElse(it) { enumValues[it] }) }
        .filter { it.raw.isNotBlank() && it.raw != VIDEO_DISABLED }

