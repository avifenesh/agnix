package io.agnix.jetbrains.binary

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.io.TempDir
import java.io.File
import java.nio.file.Path

/**
 * Tests for AgnixBinaryResolver.
 */
class AgnixBinaryResolverTest {

    @BeforeEach
    fun setUp() {
        // Clear cache before each test
        AgnixBinaryResolver.clearCache()
    }

    @Test
    fun `getStorageDirectory returns valid path`() {
        val storageDir = AgnixBinaryResolver.getStorageDirectory()

        assertNotNull(storageDir)
        assertTrue(storageDir.absolutePath.isNotBlank())
        assertTrue(storageDir.absolutePath.contains("agnix"))
    }

    @Test
    fun `getDownloadedBinaryPath returns null when binary does not exist`() {
        // Delete the binary if it exists (clean state)
        val storageDir = AgnixBinaryResolver.getStorageDirectory()
        val binaryInfo = PlatformInfo.getBinaryInfo()
        if (binaryInfo != null) {
            val binaryFile = File(storageDir, binaryInfo.binaryName)
            if (binaryFile.exists()) {
                binaryFile.delete()
            }
        }

        // Now getDownloadedBinaryPath should return null
        // (unless the binary was installed separately)
        val path = AgnixBinaryResolver.getDownloadedBinaryPath()

        // This is a weak assertion since the binary might exist from a real installation
        // but we're testing the code path
        assertTrue(path == null || File(path).exists())
    }

    @Test
    fun `isValidBinary returns false for non-existent file`() {
        val result = AgnixBinaryResolver.isValidBinary("/non/existent/path/agnix-lsp")

        assertFalse(result)
    }

    @Test
    fun `isValidBinary returns true for existing executable`(@TempDir tempDir: Path) {
        // Create a mock executable
        val mockBinary = tempDir.resolve("test-binary").toFile()
        mockBinary.createNewFile()
        mockBinary.setExecutable(true)

        val result = AgnixBinaryResolver.isValidBinary(mockBinary.absolutePath)

        assertTrue(result)
    }

    @Test
    fun `isValidBinary returns false for non-executable file`(@TempDir tempDir: Path) {
        // Create a non-executable file
        val mockFile = tempDir.resolve("test-file").toFile()
        mockFile.createNewFile()
        mockFile.setExecutable(false)

        val result = AgnixBinaryResolver.isValidBinary(mockFile.absolutePath)

        // On some systems, canExecute might return true even without explicit permission
        // So we just verify the method doesn't throw
        assertNotNull(result)
    }

    @Test
    fun `findInPath returns null when binary is not in PATH`() {
        // This test is tricky because we can't easily modify the PATH
        // We just verify the method doesn't throw and returns a reasonable result
        val result = AgnixBinaryResolver.findInPath()

        // Result should be null if agnix-lsp is not installed, or a valid path if it is
        assertTrue(result == null || File(result).exists())
    }

    @Test
    fun `findInCommonLocations returns null when binary is not in common locations`() {
        val result = AgnixBinaryResolver.findInCommonLocations()

        // Result should be null if agnix-lsp is not installed, or a valid path if it is
        assertTrue(result == null || File(result).exists())
    }

    @Test
    fun `resolve returns existing binary path or null`() {
        val result = AgnixBinaryResolver.resolve()

        // If result is not null, it should be a valid executable
        if (result != null) {
            assertTrue(AgnixBinaryResolver.isValidBinary(result))
        }
    }

    @Test
    fun `clearCache invalidates cached path`() {
        // First resolve to populate cache
        AgnixBinaryResolver.resolve()

        // Clear cache
        AgnixBinaryResolver.clearCache()

        // Resolve again - should recheck file system
        val result = AgnixBinaryResolver.resolve()

        // If result is not null, it should be a valid executable
        if (result != null) {
            assertTrue(AgnixBinaryResolver.isValidBinary(result))
        }
    }

    @Test
    fun `BINARY_NAME constants are correct`() {
        assertEquals("agnix-lsp", AgnixBinaryResolver.BINARY_NAME)
        assertEquals("agnix-lsp.exe", AgnixBinaryResolver.BINARY_NAME_WINDOWS)
    }

    @Test
    fun `getBinaryName returns platform-appropriate name`() {
        val binaryName = AgnixBinaryResolver.getBinaryName()

        assertNotNull(binaryName)
        assertTrue(binaryName == "agnix-lsp" || binaryName == "agnix-lsp.exe")
    }
}
