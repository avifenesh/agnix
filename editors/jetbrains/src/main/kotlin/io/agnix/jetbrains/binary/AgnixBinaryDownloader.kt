package io.agnix.jetbrains.binary

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.openapi.project.Project
import io.agnix.jetbrains.notifications.AgnixNotifications
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream
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

        private val TRUSTED_DOWNLOAD_HOSTS = setOf(
            "github.com",
            "objects.githubusercontent.com",
            "release-assets.githubusercontent.com",
            "github-releases.githubusercontent.com"
        )
        private const val GITHUB_USER_CONTENT_SUFFIX = ".githubusercontent.com"

        internal fun isTrustedDownloadUrl(urlString: String): Boolean {
            val url = try {
                URL(urlString)
            } catch (_: Exception) {
                return false
            }
            if (!url.protocol.equals("https", ignoreCase = true)) {
                return false
            }
            return isTrustedHost(url.host)
        }

        internal fun resolveTrustedRedirectUrl(currentUrl: String, redirectLocation: String?): String {
            if (redirectLocation.isNullOrBlank()) {
                throw IOException("Redirect location header missing")
            }

            val resolvedUrl = URL(URL(currentUrl), redirectLocation).toString()
            if (!isTrustedDownloadUrl(resolvedUrl)) {
                throw IOException("Redirect to untrusted URL not allowed: $resolvedUrl")
            }
            return resolvedUrl
        }

        private fun isTrustedHost(host: String): Boolean {
            val normalizedHost = host.lowercase()
            if (normalizedHost in TRUSTED_DOWNLOAD_HOSTS) {
                return true
            }
            return normalizedHost.endsWith(GITHUB_USER_CONTENT_SUFFIX)
        }
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
    fun downloadSync(indicator: ProgressIndicator? = null): String? {
        val binaryInfo = PlatformInfo.getBinaryInfo() ?: return null

        return try {
            downloadAndExtract(binaryInfo, indicator)
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
                if (!isTrustedDownloadUrl(currentUrl)) {
                    throw IOException("Refusing to download from untrusted URL: $currentUrl")
                }

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
                    val newUrl = resolveTrustedRedirectUrl(currentUrl, connection.getHeaderField("Location"))
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
                TarArchiveInputStream(gzis).use { tis ->
                    var entry = tis.nextEntry
                    while (entry != null) {
                        val name = entry.name
                        if (!entry.isDirectory && isTargetBinaryEntry(name, binaryName)) {
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
                if (!entry.isDirectory && isTargetBinaryEntry(name, binaryName)) {
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

    private fun isTargetBinaryEntry(entryName: String, binaryName: String): Boolean {
        return entryName == binaryName ||
            entryName.endsWith("/$binaryName") ||
            entryName.endsWith("\\$binaryName")
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

}
