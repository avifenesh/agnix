package io.agnix.jetbrains.lsp

import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.progress.ProgressIndicator
import com.redhat.devtools.lsp4ij.installation.LanguageServerInstallerBase
import io.agnix.jetbrains.binary.AgnixBinaryDownloader
import io.agnix.jetbrains.binary.AgnixBinaryResolver
import io.agnix.jetbrains.binary.PlatformInfo
import io.agnix.jetbrains.settings.AgnixSettings
import java.io.File

/**
 * LSP4IJ server installer integration for agnix-lsp.
 *
 * Uses the standard LSP4IJ install flow so check/install is tracked in the
 * language server lifecycle and console.
 */
class AgnixLspServerInstaller : LanguageServerInstallerBase() {

    private val logger = Logger.getInstance(AgnixLspServerInstaller::class.java)

    override fun checkServerInstalled(indicator: ProgressIndicator): Boolean {
        progress("Checking agnix-lsp installation...", 0.15, indicator)

        val settings = AgnixSettings.getInstance()
        val configuredPath = settings.lspPath.trim()
        if (configuredPath.isNotEmpty()) {
            val configuredBinary = File(configuredPath)
            if (configuredBinary.exists() && configuredBinary.canExecute()) {
                logger.info("Using configured agnix-lsp path: $configuredPath")
                return true
            }
        }

        val resolved = AgnixBinaryResolver.resolve()
        if (resolved != null) {
            logger.info("Found agnix-lsp binary at: $resolved")
            return true
        }

        logger.info("agnix-lsp binary not installed")
        return false
    }

    override fun install(indicator: ProgressIndicator) {
        val settings = AgnixSettings.getInstance()
        if (!settings.autoDownload) {
            throw IllegalStateException(
                "agnix-lsp was not found and auto-download is disabled in agnix settings."
            )
        }

        if (!PlatformInfo.isSupported()) {
            throw IllegalStateException(
                "No pre-built agnix-lsp binary is available for ${PlatformInfo.getPlatformDescription()}. " +
                    "Install manually with: cargo install agnix-lsp"
            )
        }

        progress("Downloading agnix-lsp binary...", 0.3, indicator)
        val downloadedPath = AgnixBinaryDownloader().downloadSync(indicator)
        if (downloadedPath == null) {
            throw IllegalStateException("Failed to download agnix-lsp binary.")
        }

        progress("agnix-lsp installed successfully.", 1.0, indicator)
        logger.info("Installed agnix-lsp to: $downloadedPath")
    }
}
