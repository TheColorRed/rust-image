package com.alakazam.mobile

import android.util.Base64
import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import java.net.BindException
import java.net.Inet4Address
import java.net.InetSocketAddress
import java.net.NetworkInterface
import java.net.ServerSocket
import java.net.Socket
import java.util.Collections
import java.util.UUID
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/** A temporary HTTP endpoint for syncing raw plugin files from a development computer. */
class PluginDevServer(cacheDir: File) {
  data class State(val running: Boolean, val port: Int, val addresses: List<String>)

  private val root = File(cacheDir, "plugin-toolkit-server").canonicalFile
  private val staging = File(root, "staging")
  private val incoming = File(root, "incoming")
  private var listener: ServerSocket? = null
  private var worker: Thread? = null
  private var clients: ExecutorService? = null

  @Synchronized
  fun start(): State {
    if (listener?.isClosed == false) return state()
    if (!staging.exists() && !staging.mkdirs()) throw IOException("Could not create the plugin sync folder")
    if (!incoming.exists() && !incoming.mkdirs()) throw IOException("Could not create the plugin inbox")
    val socket = openListener()
    listener = socket
    val executor = Executors.newCachedThreadPool()
    clients = executor
    worker = Thread({
      while (!socket.isClosed) {
        try {
          val client = socket.accept()
          executor.execute { handle(client) }
        } catch (_: IOException) {
          if (!socket.isClosed) stop()
        } catch (_: RuntimeException) {
          if (!socket.isClosed) stop()
        }
      }
    }, "plugin-dev-server").apply { isDaemon = true; start() }
    return state()
  }

  @Synchronized
  fun stop() {
    listener?.close()
    listener = null
    worker = null
    clients?.shutdownNow()
    clients = null
  }

  @Synchronized
  fun state(): State {
    val port = listener?.takeUnless { it.isClosed }?.localPort ?: 0
    val addresses = if (port == 0) emptyList() else localAddresses().map { "$it:$port" }
    return State(port != 0, port, addresses)
  }

  fun takeFolder(): String? {
    val ready = incoming.listFiles()?.firstOrNull { it.isDirectory && it.name.endsWith(".ready") } ?: return null
    val installing = File(ready.parentFile, ready.name.removeSuffix(".ready") + ".installing")
    if (installing.exists()) installing.deleteRecursively()
    return if (ready.renameTo(installing)) installing.absolutePath else null
  }

  fun clearFolder(path: String) {
    val target = File(path).canonicalFile
    if (!target.path.startsWith(incoming.canonicalPath + File.separator) || !target.name.endsWith(".installing")) {
      throw IOException("Not a development plugin folder")
    }
    if (target.exists() && !target.deleteRecursively()) throw IOException("Could not clear the installed plugin folder")
  }

  private fun handle(client: Socket) {
    client.use { socket ->
      socket.soTimeout = 60_000
      val input = BufferedInputStream(socket.getInputStream())
      val output = BufferedOutputStream(socket.getOutputStream())
      try {
        val headerBytes = ByteArrayOutputStream()
        var match = 0
        while (headerBytes.size() < MAX_HEADER_BYTES && match < 4) {
          val byte = input.read()
          if (byte < 0) throw IOException("Incomplete HTTP headers")
          headerBytes.write(byte)
          match = when {
            match == 0 && byte == 13 -> 1
            match == 1 && byte == 10 -> 2
            match == 2 && byte == 13 -> 3
            match == 3 && byte == 10 -> 4
            byte == 13 -> 1
            else -> 0
          }
        }
        if (match != 4) throw IOException("HTTP headers are too large")
        val lines = headerBytes.toString(Charsets.ISO_8859_1.name()).split("\r\n")
        val request = lines.firstOrNull()?.split(' ') ?: emptyList()
        if (request.size < 2) { respond(output, 400, "Bad request"); return }
        val headers = lines.drop(1).mapNotNull { line ->
          val colon = line.indexOf(':')
          if (colon <= 0) null else line.substring(0, colon).trim().lowercase() to line.substring(colon + 1).trim()
        }.toMap()
        if (request[0] == "GET" && request[1] == "/health") {
          respond(output, 200, "Alakazam plugin development server is ready"); return
        }
        if (request[0] != "POST") { respond(output, 404, "Not found"); return }
        val id = headers["x-plugin-id"].orEmpty()
        if (!PLUGIN_ID.matches(id)) { respond(output, 400, "Invalid plugin ID"); return }
        when (request[1]) {
          "/begin" -> beginPlugin(id, output)
          "/file" -> receiveFile(id, headers["x-plugin-path"], headers["content-length"], input, output)
          "/complete" -> completePlugin(id, output)
          else -> respond(output, 404, "Not found")
        }
      } catch (error: Exception) {
        try { respond(output, 400, error.message ?: "Bad request") } catch (_: Exception) {}
      }
    }
  }

  private fun beginPlugin(id: String, output: BufferedOutputStream) {
    val folder = File(staging, id)
    if (folder.exists() && !folder.deleteRecursively()) throw IOException("Could not reset plugin staging folder")
    if (!folder.mkdirs()) throw IOException("Could not create plugin staging folder")
    respond(output, 200, "Ready to receive plugin files")
  }

  private fun receiveFile(
    id: String,
    encodedPath: String?,
    contentLength: String?,
    input: BufferedInputStream,
    output: BufferedOutputStream,
  ) {
    val relative = try {
      String(Base64.decode(encodedPath ?: "", Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING), Charsets.UTF_8)
    } catch (_: IllegalArgumentException) { "" }
    val parts = relative.split('/')
    if (relative.isBlank() || relative.startsWith('/') || relative.contains('\\') || parts.any { it.isBlank() || it == "." || it == ".." }) {
      respond(output, 400, "Invalid plugin file path"); return
    }
    val length = contentLength?.toLongOrNull()
    if (length == null || length < 0) { respond(output, 411, "Content-Length is required"); return }
    if (length > MAX_FILE_BYTES) { respond(output, 413, "Plugin file is too large"); return }
    val base = File(staging, id).canonicalFile
    if (!base.isDirectory) { respond(output, 409, "Begin plugin upload first"); return }
    val destination = File(base, relative).canonicalFile
    if (!destination.path.startsWith(base.path + File.separator)) { respond(output, 400, "Invalid plugin file path"); return }
    val parent = destination.parentFile ?: throw IOException("Invalid plugin file path")
    if (!parent.exists() && !parent.mkdirs()) throw IOException("Could not create plugin file folder")
    val temp = File.createTempFile("upload-", ".tmp", parent)
    try {
      temp.outputStream().buffered().use { file ->
        val buffer = ByteArray(32 * 1024)
        var remaining = length
        while (remaining > 0) {
          val count = input.read(buffer, 0, minOf(buffer.size.toLong(), remaining).toInt())
          if (count < 0) throw IOException("Plugin upload ended early")
          file.write(buffer, 0, count)
          remaining -= count
        }
      }
      if (destination.exists() && !destination.delete()) throw IOException("Could not replace plugin file")
      if (!temp.renameTo(destination)) throw IOException("Could not stage plugin file")
      respond(output, 200, "Plugin file received")
    } finally {
      if (temp.exists()) temp.delete()
    }
  }

  private fun completePlugin(id: String, output: BufferedOutputStream) {
    val folder = File(staging, id)
    if (!File(folder, "plugin.yml").isFile) { respond(output, 400, "plugin.yml was not uploaded"); return }
    val ready = File(incoming, "$id-${UUID.randomUUID()}.ready")
    if (!folder.renameTo(ready)) throw IOException("Could not stage completed plugin")
    respond(output, 202, "Plugin files received")
  }

  private fun respond(output: BufferedOutputStream, status: Int, message: String) {
    val reason = when (status) {
      200 -> "OK"; 202 -> "Accepted"; 400 -> "Bad Request"; 401 -> "Unauthorized"
      404 -> "Not Found"; 411 -> "Length Required"; 413 -> "Payload Too Large"; 409 -> "Conflict"; else -> "Error"
    }
    val body = message.toByteArray(Charsets.UTF_8)
    output.write("HTTP/1.1 $status $reason\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: ${body.size}\r\nConnection: close\r\n\r\n".toByteArray(Charsets.US_ASCII))
    output.write(body)
    output.flush()
  }

  private fun localAddresses(): List<String> = try {
    Collections.list(NetworkInterface.getNetworkInterfaces())
      .filter { it.isUp && !it.isLoopback }
      .flatMap { Collections.list(it.inetAddresses) }
      .filterIsInstance<Inet4Address>()
      .filter { !it.isLoopbackAddress && !it.isLinkLocalAddress }
      .map { it.hostAddress }
      .distinct()
  } catch (_: Exception) { emptyList() }

  private fun openListener(): ServerSocket {
    var lastBindError: BindException? = null
    for (port in SERVER_PORT..MAX_PORT) {
      val socket = ServerSocket()
      socket.reuseAddress = true
      try {
        socket.bind(InetSocketAddress(port))
        return socket
      } catch (error: BindException) {
        socket.close()
        lastBindError = error
      } catch (error: IOException) {
        socket.close()
        throw error
      }
    }
    throw IOException("No available plugin development port from $SERVER_PORT to $MAX_PORT", lastBindError)
  }

  companion object {
    private const val SERVER_PORT = 2559
    private const val MAX_PORT = 65535
    private const val MAX_HEADER_BYTES = 16 * 1024
    private const val MAX_FILE_BYTES = 512L * 1024L * 1024L
    private val PLUGIN_ID = Regex("^[a-z0-9]+(?:-[a-z0-9]+)*\\.[a-z0-9]+(?:-[a-z0-9]+)*$")
  }
}
