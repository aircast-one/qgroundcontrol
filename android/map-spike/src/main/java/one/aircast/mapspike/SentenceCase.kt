package one.aircast.mapspike

private val PROPER_NOUNS = setOf("Android", "ArduPilot", "Pixhawk", "Herelink", "Trimble", "Septentrio", "Bing", "Esri", "Google", "Mapbox", "Tianditu", "VWorld", "OpenAIP", "QGroundControl")

private fun keepsCase(word: String): Boolean =
    word in PROPER_NOUNS || word.drop(1).any(Char::isUpperCase) || word.none(Char::isLowerCase)

private val PROPER_PHRASES = setOf("PX4 Pro")

fun sentenceCase(label: String): String =
    if (label in PROPER_PHRASES) label else label.split(" ").mapIndexed { at, word ->
        word.split("-").mapIndexed { part, piece ->
            val lead = piece.takeWhile { !it.isLetterOrDigit() }
            val body = piece.drop(lead.length)
            if ((at == 0 && part == 0) || keepsCase(body)) piece else lead + body.replaceFirstChar(Char::lowercaseChar)
        }.joinToString("-")
    }.joinToString(" ")
