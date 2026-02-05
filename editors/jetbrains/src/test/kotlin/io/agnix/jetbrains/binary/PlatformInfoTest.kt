package io.agnix.jetbrains.binary

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.*

/**
 * Tests for PlatformInfo utility.
 */
class PlatformInfoTest {

    @Test
    fun `getOS returns valid OS`() {
        val os = PlatformInfo.getOS()
        assertNotNull(os)
        // Should be one of the known values
        assertTrue(os in PlatformInfo.OS.entries)
    }

    @Test
    fun `getArch returns valid architecture`() {
        val arch = PlatformInfo.getArch()
        assertNotNull(arch)
        // Should be one of the known values
        assertTrue(arch in PlatformInfo.Arch.entries)
    }

    @Test
    fun `getBinaryInfo returns non-null on supported platforms`() {
        val binaryInfo = PlatformInfo.getBinaryInfo()

        // This test may return null on unsupported platforms
        // But if we get a result, it should be valid
        if (binaryInfo != null) {
            assertTrue(binaryInfo.assetName.isNotBlank())
            assertTrue(binaryInfo.binaryName.isNotBlank())
        }
    }

    @Test
    fun `isSupported matches getBinaryInfo`() {
        val binaryInfo = PlatformInfo.getBinaryInfo()
        val isSupported = PlatformInfo.isSupported()

        assertEquals(binaryInfo != null, isSupported)
    }

    @Test
    fun `getPlatformDescription returns non-empty string`() {
        val description = PlatformInfo.getPlatformDescription()

        assertNotNull(description)
        assertTrue(description.isNotBlank())
        // Should contain OS and architecture info
        assertTrue(description.contains("("))
        assertTrue(description.contains(")"))
    }

    @Test
    fun `macOS binary info selects correct architecture`() {
        if (PlatformInfo.getOS() == PlatformInfo.OS.MACOS) {
            val binaryInfo = PlatformInfo.getBinaryInfo()
            val arch = PlatformInfo.getArch()

            assertNotNull(binaryInfo)
            assertTrue(binaryInfo!!.assetName.contains("darwin"))
            assertEquals("agnix-lsp", binaryInfo.binaryName)

            // Verify architecture-specific binary is selected
            when (arch) {
                PlatformInfo.Arch.AARCH64 -> assertTrue(binaryInfo.assetName.contains("aarch64"))
                PlatformInfo.Arch.X86_64 -> assertTrue(binaryInfo.assetName.contains("x86_64"))
                else -> {} // Unknown arch returns null, covered by other tests
            }
        }
    }

    @Test
    fun `Linux binary info varies by architecture`() {
        if (PlatformInfo.getOS() == PlatformInfo.OS.LINUX) {
            val binaryInfo = PlatformInfo.getBinaryInfo()
            val arch = PlatformInfo.getArch()

            if (arch == PlatformInfo.Arch.X86_64 || arch == PlatformInfo.Arch.AARCH64) {
                assertNotNull(binaryInfo)
                assertTrue(binaryInfo!!.assetName.contains("linux"))
                assertEquals("agnix-lsp", binaryInfo.binaryName)
            }
        }
    }

    @Test
    fun `Windows binary info has exe extension`() {
        if (PlatformInfo.getOS() == PlatformInfo.OS.WINDOWS) {
            val binaryInfo = PlatformInfo.getBinaryInfo()

            if (PlatformInfo.getArch() == PlatformInfo.Arch.X86_64) {
                assertNotNull(binaryInfo)
                assertTrue(binaryInfo!!.assetName.contains("windows"))
                assertTrue(binaryInfo.binaryName.endsWith(".exe"))
            }
        }
    }

    @Test
    fun `BinaryInfo asset name has valid archive extension`() {
        val binaryInfo = PlatformInfo.getBinaryInfo()

        if (binaryInfo != null) {
            assertTrue(
                binaryInfo.assetName.endsWith(".tar.gz") ||
                binaryInfo.assetName.endsWith(".zip")
            )
        }
    }
}
