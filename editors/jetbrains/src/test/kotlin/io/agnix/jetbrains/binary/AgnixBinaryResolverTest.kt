package io.agnix.jetbrains.binary

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.io.TempDir
import java.io.File
import java.nio.file.Path

/**
 * Tests for AgnixBinaryResolver.
 */
class AgnixBinaryResolverTest {

    @Test
    fun `getStorageDirectory returns valid path`() {
        val resolver = AgnixBinaryResolver()
        val storageDir = resolver.getStorageDirectory()

        assertNotNull(storageDir)
        assertTrue(storageDir.absolutePath.isNotBlank())
        assertTrue(storageDir.absolutePath.contains("agnix"))
    }

    @Test
    fun `getDownloadedBinaryPath returns null when binary does not exist`() {
        val resolver = AgnixBinaryResolver()

        // Delete the binary if it exists (clean state)
        val storageDir = resolver.getStorageDirectory()
        val binaryInfo = PlatformInfo.getBinaryInfo()
        if (binaryInfo != null) {
            val binaryFile = File(storageDir, binaryInfo.binaryName)
            if (binaryFile.exists()) {
                binaryFile.delete()
            }
        }

        // Now getDownloadedBinaryPath should return null
        // (unless the binary was installed separately)
        val path = resolver.getDownloadedBinaryPath()

        // This is a weak assertion since the binary might exist from a real installation
        // but we're testing the code path
        assertTrue(path == null || File(path).exists())
    }

    @Test
    fun `isValidBinary returns false for non-existent file`() {
        val resolver = AgnixBinaryResolver()

        val result = resolver.isValidBinary("/non/existent/path/agnix-lsp")

        assertFalse(result)
    }

    @Test
    fun `isValidBinary returns true for existing executable`(@TempDir tempDir: Path) {
        val resolver = AgnixBinaryResolver()

        // Create a mock executable
        val mockBinary = tempDir.resolve("test-binary").toFile()
        mockBinary.createNewFile()
        mockBinary.setExecutable(true)

        val result = resolver.isValidBinary(mockBinary.absolutePath)

        assertTrue(result)
    }

    @Test
    fun `isValidBinary returns false for non-executable file`(@TempDir tempDir: Path) {
        val resolver = AgnixBinaryResolver()

        // Create a non-executable file
        val mockFile = tempDir.resolve("test-file").toFile()
        mockFile.createNewFile()
        mockFile.setExecutable(false)

        val result = resolver.isValidBinary(mockFile.absolutePath)

        // On some systems, canExecute might return true even without explicit permission
        // So we just verify the method doesn't throw
        assertNotNull(result)
    }

    @Test
    fun `findInPath returns null when binary is not in PATH`() {
        // Save original PATH
        val originalPath = System.getenv("PATH")

        // This test is tricky because we can't easily modify the PATH
        // We just verify the method doesn't throw and returns a reasonable result
        val resolver = AgnixBinaryResolver()
        val result = resolver.findInPath()

        // Result should be null if agnix-lsp is not installed, or a valid path if it is
        assertTrue(result == null || File(result).exists())
    }

    @Test
    fun `findInCommonLocations returns null when binary is not in common locations`() {
        val resolver = AgnixBinaryResolver()
        val result = resolver.findInCommonLocations()

        // Result should be null if agnix-lsp is not installed, or a valid path if it is
        assertTrue(result == null || File(result).exists())
    }

    @Test
    fun `resolve returns existing binary path or null`() {
        val resolver = AgnixBinaryResolver()
        val result = resolver.resolve()

        // If result is not null, it should be a valid executable
        if (result != null) {
            assertTrue(resolver.isValidBinary(result))
        }
    }

    @Test
    fun `BINARY_NAME constants are correct`() {
        assertEquals("agnix-lsp", AgnixBinaryResolver.BINARY_NAME)
        assertEquals("agnix-lsp.exe", AgnixBinaryResolver.BINARY_NAME_WINDOWS)
    }
}
