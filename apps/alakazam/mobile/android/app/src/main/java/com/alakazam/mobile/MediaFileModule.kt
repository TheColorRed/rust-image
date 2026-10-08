package com.alakazam.mobile

import android.net.Uri
import com.facebook.react.bridge.Promise
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.bridge.ReactMethod
import com.facebook.react.module.annotations.ReactModule
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.util.UUID

/** Copies media URIs the app can read (for example MediaStore/Camera Roll URIs) into its cache. */
@ReactModule(name = MediaFileModule.NAME)
class MediaFileModule(private val context: ReactApplicationContext) : ReactContextBaseJavaModule(context) {
  companion object {
    const val NAME = "MediaFile"
  }

  override fun getName() = NAME

  @ReactMethod
  fun copyToCache(uriString: String, fileName: String, promise: Promise) {
    try {
      val source = Uri.parse(uriString)
      val safeName = File(fileName).name.ifBlank { "image" }
      val destinationDir = File(context.cacheDir, "media/${UUID.randomUUID()}")
      if (!destinationDir.mkdirs()) throw IOException("Could not create the media cache directory")
      val destination = File(destinationDir, safeName)

      context.contentResolver.openInputStream(source)?.use { input ->
        FileOutputStream(destination).use { output -> input.copyTo(output) }
      } ?: throw IOException("Could not open media URI: $uriString")

      promise.resolve(Uri.fromFile(destination).toString())
    } catch (error: Exception) {
      promise.reject("MEDIA_COPY_FAILED", "Could not copy media into the app cache", error)
    }
  }

  /** Deletes a file this module copied into the cache, and its per-copy folder. Anything outside the cache is left alone. */
  @ReactMethod
  fun deleteFromCache(uriString: String, promise: Promise) {
    try {
      val cacheRoot = context.cacheDir.canonicalFile
      val file = File(Uri.parse(uriString).path ?: "").canonicalFile
      if (!file.path.startsWith(cacheRoot.path + File.separator)) {
        promise.resolve(false)
        return
      }
      val deleted = !file.exists() || file.delete()
      // Each copy lives alone in media/<uuid>; remove the folder once it is empty.
      val folder = file.parentFile
      if (folder != null && folder.parentFile?.name == "media" && folder.list()?.isEmpty() == true) folder.delete()
      promise.resolve(deleted)
    } catch (error: Exception) {
      promise.reject("MEDIA_DELETE_FAILED", "Could not delete the cached media", error)
    }
  }
}
