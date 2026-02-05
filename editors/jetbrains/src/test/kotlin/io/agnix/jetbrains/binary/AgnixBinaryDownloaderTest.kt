package io.agnix.jetbrains.binary

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.io.IOException

/**
 * Tests for AgnixBinaryDownloader URL trust validation.
 */
class AgnixBinaryDownloaderTest {

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
    fun `resolve trusted redirect handles absolute and relative locations`() {
        val absolute = AgnixBinaryDownloader.resolveTrustedRedirectUrl(
            "https://github.com/avifenesh/agnix/releases/latest/download/agnix-lsp.tar.gz",
            "https://objects.githubusercontent.com/github-production-release-asset/asset.tar.gz"
        )
        assertTrue(absolute.startsWith("https://objects.githubusercontent.com/"))

        val relative = AgnixBinaryDownloader.resolveTrustedRedirectUrl(
            "https://github.com/avifenesh/agnix/releases/latest/download/agnix-lsp.tar.gz",
            "/avifenesh/agnix/releases/download/v0.7.2/agnix-lsp.tar.gz"
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
}
