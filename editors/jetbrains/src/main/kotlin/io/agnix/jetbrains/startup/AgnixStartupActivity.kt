package io.agnix.jetbrains.startup

import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.ProjectActivity
import io.agnix.jetbrains.binary.AgnixBinaryDownloader
import io.agnix.jetbrains.binary.AgnixBinaryResolver
import io.agnix.jetbrains.notifications.AgnixNotifications
import io.agnix.jetbrains.settings.AgnixSettings

/**
 * Startup activity for the agnix plugin.
 *
 * Runs when a project is opened to:
 * 1. Check if the LSP binary is available
 * 2. Offer to download if not found (and auto-download is enabled)
 * 3. Log startup information
 */
class AgnixStartupActivity : ProjectActivity {

    private val logger = Logger.getInstance(AgnixStartupActivity::class.java)

    override suspend fun execute(project: Project) {
        val settings = AgnixSettings.getInstance()

        // Check if plugin is enabled
        if (!settings.enabled) {
            logger.info("agnix plugin is disabled")
            return
        }

        logger.info("agnix startup activity running for project: ${project.name}")

        // Check for LSP binary using cached resolver
        val existingBinary = AgnixBinaryResolver.resolve()

        if (existingBinary != null) {
            logger.info("agnix-lsp binary found at: $existingBinary")
            return
        }

        // Binary not found
        logger.warn("agnix-lsp binary not found")

        // Auto-download if enabled
        if (settings.autoDownload) {
            logger.info("Auto-download enabled, starting download...")
            val downloader = AgnixBinaryDownloader()
            downloader.downloadAsync(project) { downloadedPath ->
                if (downloadedPath != null) {
                    logger.info("Successfully downloaded agnix-lsp to: $downloadedPath")
                } else {
                    logger.error("Failed to download agnix-lsp")
                }
            }
        } else {
            // Show notification to user
            AgnixNotifications.notifyBinaryNotFound(project)
        }
    }
}
