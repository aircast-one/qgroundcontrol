package one.aircast.android.ui

import android.content.Context
import android.content.Intent
import android.net.Uri
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import one.aircast.android.bridge.Qgc

private const val GEOTAG_POLL_MS = 250L
internal const val DEFAULT_GEOTAG_OUTPUT = "TAGGED"

internal fun geoTagOutputText(output: String?, images: String?): String =
    output ?: images?.let { "$it/$DEFAULT_GEOTAG_OUTPUT" } ?: "Default: /$DEFAULT_GEOTAG_OUTPUT subfolder"

internal object GeoTagRun {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val files = Mutex()
    val state = MutableStateFlow<GeoTagState?>(null)
    val note = MutableStateFlow<String?>(null)
    val busy = MutableStateFlow(false)
    val imageTree = MutableStateFlow<Uri?>(null)
    val outputTree = MutableStateFlow<Uri?>(null)

    fun refresh() {
        state.value = geoTagState(Qgc.get(GEOTAG_ROOT))
    }

    fun pickDownloadedLog(path: String) {
        note.value = null
        set("logFile", path)
    }

    fun set(path: String, value: Any?) {
        scope.launch {
            Qgc.set("$GEOTAG_ROOT.$path", value)
            refresh()
        }
    }

    private fun withFiles(work: suspend () -> Unit) {
        if (!files.tryLock()) return
        busy.value = true
        scope.launch {
            runCatching { work() }
            busy.value = false
            files.unlock()
        }
    }

    private fun keep(context: Context, tree: Uri) {
        runCatching { context.contentResolver.takePersistableUriPermission(tree, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION) }
    }

    fun pickLog(activity: Context, uri: Uri) = withFiles {
        val context = activity.applicationContext
        val staged = stageLog(context, uri, documentName(context, uri))
        note.value = if (staged == null) "That file could not be read." else null
        staged?.let { Qgc.set("$GEOTAG_ROOT.logFile", it) }
        refresh()
    }

    fun pickImages(activity: Context, tree: Uri) = withFiles {
        val context = activity.applicationContext
        keep(context, tree)
        imageTree.value = tree
        val (path, count) = stageImages(context, tree)
        note.value = when {
            count == 0 -> "That folder holds no JPEG, TIFF or DNG images."
            outputTree.value == null && hasTaggedFolder(context, tree) -> GEOTAG_ALREADY_TAGGED
            else -> null
        }
        Qgc.set("$GEOTAG_ROOT.imageDirectory", path)
        refresh()
    }

    fun pickOutput(activity: Context, tree: Uri) = withFiles {
        val context = activity.applicationContext
        keep(context, tree)
        outputTree.value = tree
        note.value = GEOTAG_SAVE_HAS_IMAGES.takeIf { holdsImages(context, tree) }
    }

    fun cancel() {
        scope.launch {
            Qgc.invoke("$GEOTAG_ROOT.cancelTagging")
            refresh()
        }
    }

    fun start(activity: Context) = withFiles {
        val context = activity.applicationContext
        note.value = null
        val output = taggedOutputDir(context)
        Qgc.set("$GEOTAG_ROOT.saveDirectory", output.absolutePath)
        Qgc.invoke("$GEOTAG_ROOT.startTagging")
        refresh()
        while (state.value?.inProgress == true) {
            delay(GEOTAG_POLL_MS)
            refresh()
        }
        val finished = state.value
        val target = outputTree.value ?: imageTree.value
        if (finished != null && !finished.previewMode && finished.tagged > 0 && target != null) {
            val published = publishTagged(context, output, target, DEFAULT_GEOTAG_OUTPUT.takeIf { outputTree.value == null })
            note.value = publishedNote(published, finished.tagged)
        }
    }
}

internal fun publishedNote(published: Int, tagged: Int): String? =
    if (published == tagged) null else "Only $published of the $tagged tagged images could be written to the chosen folder."

private const val GEOTAG_OFFSET_LIMIT = 3600.0

internal fun parsedOffset(typed: String): Double? =
    typed.trim().replace(',', '.').let { text ->
        if (text.isEmpty()) 0.0
        else text.toDoubleOrNull()?.takeIf { it in -GEOTAG_OFFSET_LIMIT..GEOTAG_OFFSET_LIMIT && text.substringAfter('.', "").length <= 1 }
    }

internal const val GEOTAG_ALREADY_TAGGED = "Images have already been tagged. Existing images will be removed."
internal const val GEOTAG_SAVE_HAS_IMAGES = "The save folder already contains images."

internal fun shownOffset(seconds: Double): String = String.format(java.util.Locale.US, "%.1f", seconds)
