package io.agnix.jetbrains.binary

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.openapi.project.Project
import io.agnix.jetbrains.notifications.AgnixNotifications
import java.io.*
import java.net.HttpURLConnection
import java.net.URL
import java.nio.file.Files
import java.nio.file.attribute.PosixFilePermission
import java.util.zip.GZIPInputStream
import java.util.zip.ZipInputStream

/**
 * Downloads agnix-lsp binary from GitHub releases.
 *
 * Supports automatic platform detection and handles both .tar.gz and .zip archives.
 */
class AgnixBinaryDownloader {

    private val logger = Logger.getInstance(AgnixBinaryDownloader::class.java)

    companion object {
        const val GITHUB_REPO = "avifenesh/agnix"
        const val DOWNLOAD_TIMEOUT = 60000 // 60 seconds
        const val BUFFER_SIZE = 8192
    }

    /**
     * Download the binary asynchronously with progress indication.
     */
    fun downloadAsync(project: Project, onComplete: (String?) -> Unit) {
        val binaryInfo = PlatformInfo.getBinaryInfo()
        if (binaryInfo == null) {
            AgnixNotifications.notifyPlatformNotSupported(project)
            onComplete(null)
            return
        }

        ProgressManager.getInstance().run(object : Task.Backgroundable(
            project,
            "Downloading agnix-lsp",
            true
        ) {
            override fun run(indicator: ProgressIndicator) {
                indicator.isIndeterminate = false
                indicator.text = "Downloading agnix-lsp binary..."

                try {
                    val result = downloadAndExtract(binaryInfo, indicator)
                    ApplicationManager.getApplication().invokeLater {
                        if (result != null) {
                            AgnixNotifications.notifyDownloadSuccess(project)
                        } else {
                            AgnixNotifications.notifyDownloadFailed(project, "Download failed")
                        }
                        onComplete(result)
                    }
                } catch (e: Exception) {
                    logger.error("Failed to download agnix-lsp", e)
                    ApplicationManager.getApplication().invokeLater {
                        AgnixNotifications.notifyDownloadFailed(project, e.message ?: "Unknown error")
                        onComplete(null)
                    }
                }
            }
        })
    }

    /**
     * Download the binary synchronously (blocking).
     *
     * Used for initial startup when we need the binary immediately.
     */
    fun downloadSync(): String? {
        val binaryInfo = PlatformInfo.getBinaryInfo() ?: return null

        return try {
            downloadAndExtract(binaryInfo, null)
        } catch (e: Exception) {
            logger.error("Failed to download agnix-lsp", e)
            null
        }
    }

    /**
     * Download and extract the binary.
     */
    private fun downloadAndExtract(
        binaryInfo: PlatformInfo.BinaryInfo,
        indicator: ProgressIndicator?
    ): String? {
        val downloadUrl = getDownloadUrl(binaryInfo.assetName)
        val storageDir = AgnixBinaryResolver.getStorageDirectory()

        // Ensure storage directory exists
        if (!storageDir.exists()) {
            storageDir.mkdirs()
        }

        val archivePath = File(storageDir, binaryInfo.assetName)
        val binaryPath = File(storageDir, binaryInfo.binaryName)

        try {
            // Download archive
            indicator?.text = "Downloading from GitHub..."
            downloadFile(downloadUrl, archivePath, indicator)

            // Extract binary
            indicator?.text = "Extracting binary..."
            indicator?.fraction = 0.8

            if (binaryInfo.assetName.endsWith(".tar.gz")) {
                extractTarGz(archivePath, storageDir, binaryInfo.binaryName)
            } else if (binaryInfo.assetName.endsWith(".zip")) {
                extractZip(archivePath, storageDir, binaryInfo.binaryName)
            }

            // Make executable on Unix systems
            if (PlatformInfo.getOS() != PlatformInfo.OS.WINDOWS) {
                makeExecutable(binaryPath)
            }

            // Verify binary exists
            if (!binaryPath.exists()) {
                logger.error("Binary not found after extraction: ${binaryPath.absolutePath}")
                return null
            }

            indicator?.fraction = 1.0
            logger.info("Successfully downloaded agnix-lsp to: ${binaryPath.absolutePath}")

            // Clear resolver cache so it picks up the new binary
            AgnixBinaryResolver.clearCache()

            return binaryPath.absolutePath

        } finally {
            // Clean up archive file
            if (archivePath.exists()) {
                archivePath.delete()
            }
        }
    }

    /**
     * Get the download URL for the latest release asset.
     */
    private fun getDownloadUrl(assetName: String): String {
        return "https://github.com/$GITHUB_REPO/releases/latest/download/$assetName"
    }

    /**
     * Download a file from URL with progress tracking.
     */
    private fun downloadFile(urlString: String, destination: File, indicator: ProgressIndicator?) {
        var connection: HttpURLConnection? = null
        var inputStream: InputStream? = null
        var outputStream: FileOutputStream? = null

        try {
            var currentUrl = urlString
            var redirectCount = 0
            val maxRedirects = 5

            // Follow redirects (GitHub releases use redirects)
            while (redirectCount < maxRedirects) {
                val url = URL(currentUrl)
                connection = url.openConnection() as HttpURLConnection
                connection.connectTimeout = DOWNLOAD_TIMEOUT
                connection.readTimeout = DOWNLOAD_TIMEOUT
                connection.instanceFollowRedirects = false
                connection.setRequestProperty("Accept", "application/octet-stream")

                val responseCode = connection.responseCode

                if (responseCode == HttpURLConnection.HTTP_MOVED_PERM ||
                    responseCode == HttpURLConnection.HTTP_MOVED_TEMP ||
                    responseCode == HttpURLConnection.HTTP_SEE_OTHER) {
                    val newUrl = connection.getHeaderField("Location")
                    connection.disconnect()
                    currentUrl = newUrl
                    redirectCount++
                    continue
                }

                if (responseCode != HttpURLConnection.HTTP_OK) {
                    throw IOException("Download failed with status: $responseCode")
                }

                break
            }

            if (connection == null) {
                throw IOException("Failed to establish connection after redirects")
            }

            val contentLength = connection.contentLength.toLong()
            inputStream = connection.inputStream
            outputStream = FileOutputStream(destination)

            val buffer = ByteArray(BUFFER_SIZE)
            var totalBytesRead = 0L
            var bytesRead: Int

            while (true) {
                bytesRead = inputStream.read(buffer)
                if (bytesRead == -1) break

                outputStream.write(buffer, 0, bytesRead)
                totalBytesRead += bytesRead

                if (indicator != null && contentLength > 0) {
                    indicator.fraction = (totalBytesRead.toDouble() / contentLength) * 0.7
                }
            }

        } finally {
            inputStream?.close()
            outputStream?.close()
            connection?.disconnect()
        }
    }

    /**
     * Verify that an output file path is within the destination directory.
     * Appends separator to prevent prefix matching issues (e.g., /tmp/agnix vs /tmp/agnix-other).
     */
    private fun verifyPathWithinDestination(outFile: File, destination: File) {
        val canonicalDest = destination.canonicalPath + File.separator
        val canonicalOut = outFile.canonicalPath
        if (!canonicalOut.startsWith(canonicalDest) && canonicalOut != destination.canonicalPath) {
            throw SecurityException("Output path escapes destination directory: $canonicalOut")
        }
    }

    /**
     * Extract a .tar.gz archive.
     *
     * Writes only the target binary using a fixed filename (not archive entry names)
     * to prevent path traversal. The canonical path check is a defense-in-depth guard.
     */
    private fun extractTarGz(archive: File, destination: File, binaryName: String) {
        FileInputStream(archive).use { fis ->
            GZIPInputStream(fis).use { gzis ->
                TarInputStream(gzis).use { tis ->
                    var entry = tis.nextEntry
                    while (entry != null) {
                        val name = entry.name
                        // Look for the binary file (may be in root or subdirectory)
                        if (name.endsWith(binaryName) || name == binaryName) {
                            val outFile = File(destination, binaryName)
                            verifyPathWithinDestination(outFile, destination)
                            FileOutputStream(outFile).use { fos ->
                                tis.copyTo(fos)
                            }
                            return
                        }
                        entry = tis.nextEntry
                    }
                }
            }
        }
        logger.error("Binary $binaryName not found in tar.gz archive")
    }

    /**
     * Extract a .zip archive.
     *
     * Writes only the target binary using a fixed filename (not archive entry names)
     * to prevent path traversal. The canonical path check is a defense-in-depth guard.
     */
    private fun extractZip(archive: File, destination: File, binaryName: String) {
        ZipInputStream(FileInputStream(archive)).use { zis ->
            var entry = zis.nextEntry
            while (entry != null) {
                val name = entry.name
                // Look for the binary file (may be in root or subdirectory)
                if (name.endsWith(binaryName) || name == binaryName) {
                    val outFile = File(destination, binaryName)
                    verifyPathWithinDestination(outFile, destination)
                    FileOutputStream(outFile).use { fos ->
                        zis.copyTo(fos)
                    }
                    return
                }
                entry = zis.nextEntry
            }
        }
        logger.error("Binary $binaryName not found in zip archive")
    }

    /**
     * Make a file executable on Unix systems.
     *
     * Uses Java NIO to set POSIX file permissions directly.
     * This is safe as we're only modifying files we created in our storage directory.
     */
    private fun makeExecutable(file: File) {
        try {
            val permissions = Files.getPosixFilePermissions(file.toPath()).toMutableSet()
            permissions.add(PosixFilePermission.OWNER_EXECUTE)
            permissions.add(PosixFilePermission.GROUP_EXECUTE)
            permissions.add(PosixFilePermission.OTHERS_EXECUTE)
            Files.setPosixFilePermissions(file.toPath(), permissions)
        } catch (e: UnsupportedOperationException) {
            // Windows doesn't support POSIX permissions, but also doesn't need +x
            logger.debug("POSIX permissions not supported on this platform")
        } catch (e: Exception) {
            // Fallback: Use ProcessBuilder with explicit arguments (no shell injection risk)
            // The file path is from our controlled storage directory, not user input
            try {
                val process = ProcessBuilder("chmod", "+x", file.absolutePath)
                    .redirectErrorStream(true)
                    .start()
                process.waitFor()
            } catch (e2: Exception) {
                logger.warn("Failed to make binary executable", e2)
            }
        }
    }

    /**
     * Simple tar input stream implementation.
     *
     * Handles basic tar format for extracting files.
     */
    private class TarInputStream(inputStream: InputStream) : FilterInputStream(inputStream) {
        private var currentEntry: TarEntry? = null
        private var currentFileSize: Long = 0
        private var bytesRead: Long = 0

        data class TarEntry(val name: String, val size: Long)

        private fun skipFully(n: Long) {
            var remaining = n
            while (remaining > 0) {
                val skipped = `in`.skip(remaining)
                if (skipped <= 0) {
                    // skip() returned 0 or negative, read and discard instead
                    val toRead = minOf(remaining, 8192L).toInt()
                    val buffer = ByteArray(toRead)
                    val read = `in`.read(buffer, 0, toRead)
                    if (read < 0) break
                    remaining -= read
                } else {
                    remaining -= skipped
                }
            }
        }

        val nextEntry: TarEntry?
            get() {
                // Skip remaining bytes of current entry
                if (currentEntry != null) {
                    val remaining = currentFileSize - bytesRead
                    if (remaining > 0) {
                        skipFully(remaining)
                    }
                    // Skip padding to 512-byte boundary
                    val padding = (512 - (currentFileSize % 512)) % 512
                    if (padding > 0) {
                        skipFully(padding)
                    }
                }

                // Read header block (512 bytes)
                val header = ByteArray(512)
                var totalRead = 0
                while (totalRead < 512) {
                    val n = `in`.read(header, totalRead, 512 - totalRead)
                    if (n < 0) return null
                    totalRead += n
                }

                // Check for end of archive (two zero blocks)
                if (header.all { it == 0.toByte() }) {
                    return null
                }

                // Parse header
                val name = String(header, 0, 100).trim('\u0000', ' ')
                val sizeStr = String(header, 124, 12).trim('\u0000', ' ')
                val size = if (sizeStr.isNotEmpty()) {
                    sizeStr.toLongOrNull(8) ?: 0L
                } else {
                    0L
                }

                currentEntry = TarEntry(name, size)
                currentFileSize = size
                bytesRead = 0

                return currentEntry
            }

        override fun read(): Int {
            if (bytesRead >= currentFileSize) return -1
            val b = `in`.read()
            if (b >= 0) bytesRead++
            return b
        }

        override fun read(b: ByteArray, off: Int, len: Int): Int {
            if (bytesRead >= currentFileSize) return -1
            val maxRead = minOf(len.toLong(), currentFileSize - bytesRead).toInt()
            val n = `in`.read(b, off, maxRead)
            if (n > 0) bytesRead += n
            return n
        }
    }
}
