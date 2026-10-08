package com.alakazam.mobile

import android.net.Uri
import com.facebook.react.bridge.Arguments
import com.facebook.react.bridge.Promise
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.bridge.ReactMethod
import com.facebook.react.module.annotations.ReactModule
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.util.UUID

/**
 * Keeps projects in the app's files folder, which Android only clears on uninstall or "clear data" (unlike the cache).
 * Each project is a folder, `projects/<id>/`, holding `source.<ext>` (the untouched original copied in once) and
 * `project.json` (the name, timestamps and the edit recipe) and, once edited, `result-<time>.jpg` (the final image, so it
 * can be shown before the recipe is replayed). The source is never rewritten.
 */
@ReactModule(name = ProjectStoreModule.NAME)
class ProjectStoreModule(private val context: ReactApplicationContext) : ReactContextBaseJavaModule(context) {
  companion object {
    const val NAME = "ProjectStore"
    private const val PROJECT_FILE = "project.json"
  }

  override fun getName() = NAME

  private val root get() = File(context.filesDir, "projects")

  /** The folder for a project id. Ids are generated here, so anything with a path separator is refused. */
  private fun folderOf(id: String): File {
    require(id.matches(Regex("[A-Za-z0-9-]+"))) { "Invalid project id: $id" }
    return File(root, id)
  }

  /** Writes the file in one step: a crash mid-save leaves the old project.json, never half of a new one. */
  private fun writeProjectFile(folder: File, json: JSONObject) {
    val temporary = File(folder, "$PROJECT_FILE.tmp")
    FileOutputStream(temporary).use { it.write(json.toString().toByteArray()) }
    if (!temporary.renameTo(File(folder, PROJECT_FILE))) throw IOException("Could not save $PROJECT_FILE")
  }

  /** The project as the app sees it: its project.json plus `dir`, the folder as a `file://` URI. */
  private fun readProject(folder: File): JSONObject =
    JSONObject(File(folder, PROJECT_FILE).readText()).put("dir", Uri.fromFile(folder).toString())

  /** Copies the image (a `content://` or `file://` URI) into a new project folder and returns the new project as JSON. */
  @ReactMethod
  fun createProject(sourceUri: String, fileName: String, promise: Promise) {
    var folder: File? = null
    try {
      val id = UUID.randomUUID().toString()
      val name = File(fileName).name.ifBlank { "image" }
      val extension = name.substringAfterLast('.', "jpg").lowercase().filter { it.isLetterOrDigit() }.ifBlank { "jpg" }
      val sourceFile = "source.$extension"
      folder = folderOf(id)
      if (!folder.mkdirs()) throw IOException("Could not create the project folder")

      context.contentResolver.openInputStream(Uri.parse(sourceUri))?.use { input ->
        FileOutputStream(File(folder, sourceFile)).use { output -> input.copyTo(output) }
      } ?: throw IOException("Could not open the image: $sourceUri")

      val now = System.currentTimeMillis()
      writeProjectFile(
        folder,
        JSONObject()
          .put("version", 1)
          .put("id", id)
          .put("name", name)
          .put("sourceFile", sourceFile)
          .put("timeline", JSONArray())
          .put("cursor", 0)
          .put("createdAt", now)
          .put("modifiedAt", now),
      )
      promise.resolve(readProject(folder).toString())
    } catch (error: Exception) {
      folder?.deleteRecursively()
      promise.reject("PROJECT_CREATE_FAILED", "Could not create the project", error)
    }
  }

  /** Every project as JSON, newest first. A folder without a readable project.json is skipped, not fatal. */
  @ReactMethod
  fun listProjects(promise: Promise) {
    try {
      val projects = root.listFiles { entry -> entry.isDirectory }
        ?.mapNotNull { folder -> runCatching { readProject(folder) }.getOrNull() }
        ?.sortedByDescending { it.optLong("createdAt") }
        ?: emptyList()
      val result = Arguments.createArray()
      projects.forEach { result.pushString(it.toString()) }
      promise.resolve(result)
    } catch (error: Exception) {
      promise.reject("PROJECT_LIST_FAILED", "Could not list the projects", error)
    }
  }

  /** Saves a project's JSON (its name and edit recipe). The source image is never rewritten. */
  @ReactMethod
  fun writeProject(id: String, json: String, promise: Promise) {
    try {
      val folder = folderOf(id)
      if (!folder.isDirectory) throw IOException("No such project: $id")
      val project = JSONObject(json).put("modifiedAt", System.currentTimeMillis())
      writeProjectFile(folder, project)
      // Each save of the final image gets a new file name (so the image cache never shows a stale one); drop the old ones.
      val resultFile = project.optString("resultFile")
      folder.listFiles { entry -> entry.name.startsWith("result") && entry.name != resultFile }?.forEach { it.delete() }
      promise.resolve(null)
    } catch (error: Exception) {
      promise.reject("PROJECT_WRITE_FAILED", "Could not save the project", error)
    }
  }

  /** Deletes the project's whole folder. The photo it was made from is never touched. */
  @ReactMethod
  fun deleteProject(id: String, promise: Promise) {
    try {
      promise.resolve(folderOf(id).deleteRecursively())
    } catch (error: Exception) {
      promise.reject("PROJECT_DELETE_FAILED", "Could not delete the project", error)
    }
  }
}
