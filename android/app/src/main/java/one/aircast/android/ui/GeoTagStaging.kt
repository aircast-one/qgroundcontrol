package one.aircast.android.ui

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract
import java.io.File

private const val GEOTAG_CACHE = "geotag"

internal data class TreeEntry(val documentId: String, val name: String, val mime: String)

private fun stagingDir(context: Context, name: String): File =
    File(File(context.cacheDir, GEOTAG_CACHE), name).also { dir ->
        dir.deleteRecursively()
        dir.mkdirs()
    }

internal fun taggedOutputDir(context: Context): File = stagingDir(context, "tagged")

internal fun treeChildren(context: Context, tree: Uri, parentId: String = DocumentsContract.getTreeDocumentId(tree)): List<TreeEntry> {
    val children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, parentId)
    val columns = arrayOf(
        DocumentsContract.Document.COLUMN_DOCUMENT_ID,
        DocumentsContract.Document.COLUMN_DISPLAY_NAME,
        DocumentsContract.Document.COLUMN_MIME_TYPE,
    )
    return context.contentResolver.query(children, columns, null, null, null)?.use { cursor ->
        generateSequence { if (cursor.moveToNext()) TreeEntry(cursor.getString(0), cursor.getString(1), cursor.getString(2)) else null }.toList()
    }.orEmpty()
}

private fun copyInto(context: Context, from: Uri, target: File): Boolean = runCatching {
    context.contentResolver.openInputStream(from)?.use { source -> target.outputStream().use { source.copyTo(it) } } != null
}.getOrDefault(false)

internal fun stageLog(context: Context, uri: Uri, name: String): String? {
    val target = File(stagingDir(context, "log"), name.ifBlank { "flight.log" })
    return target.absolutePath.takeIf { copyInto(context, uri, target) }
}

internal fun stageImages(context: Context, tree: Uri): Pair<String, Int> {
    val dir = stagingDir(context, "images")
    val copied = treeChildren(context, tree)
        .filter { it.mime != DocumentsContract.Document.MIME_TYPE_DIR && isGeoTagImage(it.name) }
        .count { entry -> copyInto(context, DocumentsContract.buildDocumentUriUsingTree(tree, entry.documentId), File(dir, entry.name)) }
    return dir.absolutePath to copied
}

private fun imageMime(name: String): String =
    android.webkit.MimeTypeMap.getSingleton().getMimeTypeFromExtension(name.substringAfterLast('.').lowercase()) ?: "application/octet-stream"

internal fun treeName(tree: Uri): String =
    DocumentsContract.getTreeDocumentId(tree).substringAfterLast(':').substringAfterLast('/').ifBlank { tree.toString() }

internal fun documentName(context: Context, uri: Uri): String =
    context.contentResolver.query(uri, arrayOf(android.provider.OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
        if (cursor.moveToFirst()) cursor.getString(0) else null
    } ?: uri.lastPathSegment.orEmpty().substringAfterLast('/')

internal fun downloadedLogs(logSavePath: String): List<File> =
    File(logSavePath).listFiles { file -> file.isFile && file.extension.lowercase() in setOf("ulg", "bin") }
        .orEmpty()
        .sortedByDescending { it.lastModified() }

private fun folderIn(context: Context, tree: Uri, name: String): Uri? {
    val root = DocumentsContract.buildDocumentUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree))
    val existing = treeChildren(context, tree).firstOrNull { it.name == name && it.mime == DocumentsContract.Document.MIME_TYPE_DIR }
    return existing?.let { DocumentsContract.buildDocumentUriUsingTree(tree, it.documentId) }
        ?: runCatching { DocumentsContract.createDocument(context.contentResolver, root, DocumentsContract.Document.MIME_TYPE_DIR, name) }.getOrNull()
}

internal fun publishTagged(context: Context, staged: File, tree: Uri, subfolder: String?): Int {
    val folder = subfolder?.let { folderIn(context, tree, it) }
        ?: DocumentsContract.buildDocumentUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree))
    val present = treeChildren(context, tree, DocumentsContract.getDocumentId(folder)).associate { it.name to DocumentsContract.buildDocumentUriUsingTree(tree, it.documentId) }
    return staged.listFiles().orEmpty().filter { it.isFile && isGeoTagImage(it.name) }.count { file ->
        val created = present[file.name] == null
        val target = present[file.name]
            ?: runCatching { DocumentsContract.createDocument(context.contentResolver, folder, imageMime(file.name), file.name) }.getOrNull()
        val written = target != null && runCatching {
            context.contentResolver.openOutputStream(target, "wt")?.use { out -> file.inputStream().use { it.copyTo(out) } } != null
        }.getOrDefault(false)
        if (!written && created && target != null) runCatching { DocumentsContract.deleteDocument(context.contentResolver, target) }
        written
    }
}
