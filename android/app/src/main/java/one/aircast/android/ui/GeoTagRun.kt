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
import kotlinx.coroutines.sync.withLock
import one.aircast.android.bridge.Qgc

private const val GEOTAG_POLL_MS = 250L
internal const val DEFAULT_GEOTAG_OUTPUT = "TAGGED"

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

    fun set(path: String, value: Any?) {
        scope.launch {
            Qgc.set("$GEOTAG_ROOT.$path", value)
            refresh()
        }
    }

    private fun withFiles(work: suspend () -> Unit) {
        scope.launch {
            files.withLock {
                busy.value = true
                runCatching { work() }
                busy.value = false
            }
        }
    }

    private fun keep(context: Context, tree: Uri) {
        runCatching { context.contentResolver.takePersistableUriPermission(tree, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION) }
    }

    fun pickLog(context: Context, uri: Uri) = withFiles {
        val staged = stageLog(context, uri, documentName(context, uri))
        if (staged == null) note.value = "That file could not be read." else Qgc.set("$GEOTAG_ROOT.logFile", staged)
        refresh()
    }

    fun pickImages(context: Context, tree: Uri) = withFiles {
        keep(context, tree)
        imageTree.value = tree
        val (path, count) = stageImages(context, tree)
        note.value = if (count == 0) "That folder holds no JPEG, TIFF or DNG images." else null
        Qgc.set("$GEOTAG_ROOT.imageDirectory", path)
        refresh()
    }

    fun pickOutput(context: Context, tree: Uri) {
        keep(context, tree)
        outputTree.value = tree
    }

    fun cancel() {
        scope.launch {
            Qgc.invoke("$GEOTAG_ROOT.cancelTagging")
            refresh()
        }
    }

    fun start(context: Context) = withFiles {
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

internal fun parsedOffset(typed: String): Double? = typed.trim().replace(',', '.').toDoubleOrNull()

internal fun shownOffset(seconds: Double): String = String.format(java.util.Locale.US, "%.1f", seconds)
