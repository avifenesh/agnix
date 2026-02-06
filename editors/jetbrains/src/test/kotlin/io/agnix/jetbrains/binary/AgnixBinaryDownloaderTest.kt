package io.agnix.jetbrains.binary

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.nio.file.Path
import java.util.zip.GZIPOutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream
import org.apache.commons.compress.archivers.tar.TarArchiveEntry
import org.apache.commons.compress.archivers.tar.TarArchiveOutputStream

/**
 * Tests for AgnixBinaryDownloader URL trust validation and archive extraction.
 */
class AgnixBinaryDownloaderTest {

    // ---- URL trust tests ----

    @Test
    fun `trusted download URL accepts github release domains`() {
        assertTrue(AgnixBinaryDownloader.isTrustedDownloadUrl("https://github.com/avifenesh/agnix/releases/latest/download/agnix-lsp-x86_64-apple-darwin.tar.gz"))
        assertTrue(AgnixBinaryDownloader.isTrustedDownloadUrl("https://objects.githubusercontent.com/github-production-release-asset/asset.tar.gz"))
        assertTrue(AgnixBinaryDownloader.isTrustedDownloadUrl("https://release-assets.githubusercontent.com/github-production-release-asset/asset.tar.gz"))
    }

    @Test
    fun `trusted download URL rejects non-https and unknown hosts`() {
        assertFalse(AgnixBinaryDownloader.isTrustedDownloadUrl("http://github.com/avifenesh/agnix/releases/latest/download/agnix-lsp.tar.gz"))
        assertFalse(AgnixBinaryDownloader.isTrustedDownloadUrl("https://example.com/agnix-lsp.tar.gz"))
        assertFalse(AgnixBinaryDownloader.isTrustedDownloadUrl("not-a-url"))
    }

    @Test
    fun `trusted download URL rejects malformed uri content`() {
        assertFalse(AgnixBinaryDownloader.isTrustedDownloadUrl("https://github.com/invalid path"))
        assertFalse(AgnixBinaryDownloader.isTrustedDownloadUrl("https://github.com/\nagnix"))
    }

    @Test
    fun `resolve trusted redirect handles absolute and relative locations`() {
        val absolute = AgnixBinaryDownloader.resolveTrustedRedirectUrl(
            "https://github.com/avifenesh/agnix/releases/latest/download/agnix-lsp.tar.gz",
            "https://objects.githubusercontent.com/github-production-release-asset/asset.tar.gz"
        )
        assertTrue(absolute.startsWith("https://objects.githubusercontent.com/"))

        val relative = AgnixBinaryDownloader.resolveTrustedRedirectUrl(
            "https://github.com/avifenesh/agnix/releases/latest/download/agnix-lsp.tar.gz",
            "/avifenesh/agnix/releases/download/v0.8.0/agnix-lsp.tar.gz"
        )
        assertTrue(relative.startsWith("https://github.com/"))
    }

    @Test
    fun `resolve trusted redirect rejects missing and untrusted targets`() {
        assertThrows(IOException::class.java) {
            AgnixBinaryDownloader.resolveTrustedRedirectUrl(
                "https://github.com/avifenesh/agnix/releases/latest/download/agnix-lsp.tar.gz",
                null
            )
        }

        assertThrows(IOException::class.java) {
            AgnixBinaryDownloader.resolveTrustedRedirectUrl(
                "https://github.com/avifenesh/agnix/releases/latest/download/agnix-lsp.tar.gz",
                "http://objects.githubusercontent.com/github-production-release-asset/asset.tar.gz"
            )
        }

        assertThrows(IOException::class.java) {
            AgnixBinaryDownloader.resolveTrustedRedirectUrl(
                "https://github.com/avifenesh/agnix/releases/latest/download/agnix-lsp.tar.gz",
                "https://malicious.example.com/payload.tar.gz"
            )
        }
    }

    // ---- isTargetBinaryEntry tests ----

    @Test
    fun `isTargetBinaryEntry matches exact binary name`() {
        assertTrue(AgnixBinaryDownloader.isTargetBinaryEntry("agnix-lsp", "agnix-lsp"))
    }

    @Test
    fun `isTargetBinaryEntry matches binary in unix subdirectory`() {
        assertTrue(AgnixBinaryDownloader.isTargetBinaryEntry("release/agnix-lsp", "agnix-lsp"))
    }

    @Test
    fun `isTargetBinaryEntry matches binary in windows subdirectory`() {
        assertTrue(AgnixBinaryDownloader.isTargetBinaryEntry("release\\agnix-lsp", "agnix-lsp"))
    }

    @Test
    fun `isTargetBinaryEntry rejects unrelated file name`() {
        assertFalse(AgnixBinaryDownloader.isTargetBinaryEntry("README.md", "agnix-lsp"))
    }

    @Test
    fun `isTargetBinaryEntry rejects partial name match`() {
        assertFalse(AgnixBinaryDownloader.isTargetBinaryEntry("agnix-lsp-debug", "agnix-lsp"))
        assertFalse(AgnixBinaryDownloader.isTargetBinaryEntry("not-agnix-lsp", "agnix-lsp"))
    }

    @Test
    fun `isTargetBinaryEntry is case sensitive`() {
        assertFalse(AgnixBinaryDownloader.isTargetBinaryEntry("Agnix-Lsp", "agnix-lsp"))
        assertFalse(AgnixBinaryDownloader.isTargetBinaryEntry("release/AGNIX-LSP", "agnix-lsp"))
    }

    // ---- verifyPathWithinDestination tests ----

    @Test
    fun `verifyPathWithinDestination accepts file within destination`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val outFile = File(dest, "agnix-lsp")
        // Should not throw
        AgnixBinaryDownloader.verifyPathWithinDestination(outFile, dest)
    }

    @Test
    fun `verifyPathWithinDestination throws for path traversal`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val escapedFile = File(dest, "../escape")
        assertThrows(SecurityException::class.java) {
            AgnixBinaryDownloader.verifyPathWithinDestination(escapedFile, dest)
        }
    }

    @Test
    fun `verifyPathWithinDestination accepts path equal to destination`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        // Should not throw when outFile is the destination itself
        AgnixBinaryDownloader.verifyPathWithinDestination(dest, dest)
    }

    // ---- extractTarGz tests ----

    @Test
    fun `extractTarGz extracts binary from root level entry`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val archive = File(dest, "test.tar.gz")
        val binaryContent = "binary-content-root".toByteArray()

        createTarGzArchive(archive, mapOf("agnix-lsp" to binaryContent))

        AgnixBinaryDownloader.extractTarGz(archive, dest, "agnix-lsp")

        val extracted = File(dest, "agnix-lsp")
        assertTrue(extracted.exists())
        assertTrue(extracted.readBytes().contentEquals(binaryContent))
    }

    @Test
    fun `extractTarGz extracts binary from subdirectory entry`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val archive = File(dest, "test.tar.gz")
        val binaryContent = "binary-content-subdir".toByteArray()

        createTarGzArchive(archive, mapOf("release/agnix-lsp" to binaryContent))

        AgnixBinaryDownloader.extractTarGz(archive, dest, "agnix-lsp")

        val extracted = File(dest, "agnix-lsp")
        assertTrue(extracted.exists())
        assertTrue(extracted.readBytes().contentEquals(binaryContent))
    }

    @Test
    fun `extractTarGz handles no matching binary`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val archive = File(dest, "test.tar.gz")

        createTarGzArchive(archive, mapOf("README.md" to "readme".toByteArray()))

        AgnixBinaryDownloader.extractTarGz(archive, dest, "agnix-lsp")

        val extracted = File(dest, "agnix-lsp")
        assertFalse(extracted.exists())
    }

    @Test
    fun `extractTarGz extracts first matching entry`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val archive = File(dest, "test.tar.gz")
        val firstContent = "first-match".toByteArray()
        val secondContent = "second-match".toByteArray()

        createTarGzArchive(archive, mapOf(
            "agnix-lsp" to firstContent,
            "release/agnix-lsp" to secondContent
        ))

        AgnixBinaryDownloader.extractTarGz(archive, dest, "agnix-lsp")

        val extracted = File(dest, "agnix-lsp")
        assertTrue(extracted.exists())
        assertTrue(extracted.readBytes().contentEquals(firstContent))
    }

    @Test
    fun `extractTarGz skips directory entries`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val archive = File(dest, "test.tar.gz")

        // Create a tar.gz with a directory entry named "agnix-lsp/" and a file "other.txt"
        FileOutputStream(archive).use { fos ->
            GZIPOutputStream(fos).use { gzos ->
                TarArchiveOutputStream(gzos).use { tos ->
                    // Directory entry with same name as target binary
                    val dirEntry = TarArchiveEntry("agnix-lsp/")
                    tos.putArchiveEntry(dirEntry)
                    tos.closeArchiveEntry()

                    // A non-matching file entry
                    val data = "other-content".toByteArray()
                    val fileEntry = TarArchiveEntry("other.txt")
                    fileEntry.size = data.size.toLong()
                    tos.putArchiveEntry(fileEntry)
                    tos.write(data)
                    tos.closeArchiveEntry()
                }
            }
        }

        AgnixBinaryDownloader.extractTarGz(archive, dest, "agnix-lsp")

        val extracted = File(dest, "agnix-lsp")
        assertFalse(extracted.exists())
    }

    // ---- extractZip tests ----

    @Test
    fun `extractZip extracts binary from root level entry`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val archive = File(dest, "test.zip")
        val binaryContent = "binary-content-root".toByteArray()

        createZipArchive(archive, mapOf("agnix-lsp.exe" to binaryContent))

        AgnixBinaryDownloader.extractZip(archive, dest, "agnix-lsp.exe")

        val extracted = File(dest, "agnix-lsp.exe")
        assertTrue(extracted.exists())
        assertTrue(extracted.readBytes().contentEquals(binaryContent))
    }

    @Test
    fun `extractZip extracts binary from subdirectory entry`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val archive = File(dest, "test.zip")
        val binaryContent = "binary-content-subdir".toByteArray()

        createZipArchive(archive, mapOf("release/agnix-lsp.exe" to binaryContent))

        AgnixBinaryDownloader.extractZip(archive, dest, "agnix-lsp.exe")

        val extracted = File(dest, "agnix-lsp.exe")
        assertTrue(extracted.exists())
        assertTrue(extracted.readBytes().contentEquals(binaryContent))
    }

    @Test
    fun `extractZip handles no matching binary`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val archive = File(dest, "test.zip")

        createZipArchive(archive, mapOf("README.md" to "readme".toByteArray()))

        AgnixBinaryDownloader.extractZip(archive, dest, "agnix-lsp.exe")

        val extracted = File(dest, "agnix-lsp.exe")
        assertFalse(extracted.exists())
    }

    @Test
    fun `extractZip extracts first matching entry`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val archive = File(dest, "test.zip")
        val firstContent = "first-match".toByteArray()
        val secondContent = "second-match".toByteArray()

        createZipArchive(archive, mapOf(
            "agnix-lsp.exe" to firstContent,
            "release/agnix-lsp.exe" to secondContent
        ))

        AgnixBinaryDownloader.extractZip(archive, dest, "agnix-lsp.exe")

        val extracted = File(dest, "agnix-lsp.exe")
        assertTrue(extracted.exists())
        assertTrue(extracted.readBytes().contentEquals(firstContent))
    }

    @Test
    fun `extractZip skips directory entries`(@TempDir tempDir: Path) {
        val dest = tempDir.toFile()
        val archive = File(dest, "test.zip")

        // Create a zip with a directory entry named "agnix-lsp.exe/" and a file "other.txt"
        ZipOutputStream(FileOutputStream(archive)).use { zos ->
            // Directory entry with same name as target binary
            val dirEntry = ZipEntry("agnix-lsp.exe/")
            zos.putNextEntry(dirEntry)
            zos.closeEntry()

            // A non-matching file entry
            val data = "other-content".toByteArray()
            val fileEntry = ZipEntry("other.txt")
            zos.putNextEntry(fileEntry)
            zos.write(data)
            zos.closeEntry()
        }

        AgnixBinaryDownloader.extractZip(archive, dest, "agnix-lsp.exe")

        val extracted = File(dest, "agnix-lsp.exe")
        assertFalse(extracted.exists())
    }

    // ---- Helper methods ----

    private fun createTarGzArchive(archive: File, entries: Map<String, ByteArray>) {
        FileOutputStream(archive).use { fos ->
            GZIPOutputStream(fos).use { gzos ->
                TarArchiveOutputStream(gzos).use { tos ->
                    for ((name, data) in entries) {
                        val entry = TarArchiveEntry(name)
                        entry.size = data.size.toLong()
                        tos.putArchiveEntry(entry)
                        tos.write(data)
                        tos.closeArchiveEntry()
                    }
                }
            }
        }
    }

    private fun createZipArchive(archive: File, entries: Map<String, ByteArray>) {
        ZipOutputStream(FileOutputStream(archive)).use { zos ->
            for ((name, data) in entries) {
                val entry = ZipEntry(name)
                zos.putNextEntry(entry)
                zos.write(data)
                zos.closeEntry()
            }
        }
    }
}
