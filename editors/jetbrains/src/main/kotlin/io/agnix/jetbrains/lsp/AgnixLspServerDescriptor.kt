package io.agnix.jetbrains.lsp

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider
import io.agnix.jetbrains.binary.AgnixBinaryResolver
import io.agnix.jetbrains.notifications.AgnixNotifications
import io.agnix.jetbrains.settings.AgnixSettings
import java.io.File

/**
 * LSP server descriptor for agnix.
 *
 * Manages the lifecycle of the agnix-lsp process using stdio transport.
 * Handles binary resolution, download if needed, and process startup.
 */
class AgnixLspServerDescriptor(
    private val project: Project
) : OSProcessStreamConnectionProvider() {

    private val logger = Logger.getInstance(AgnixLspServerDescriptor::class.java)

    init {
        // Resolve binary path without blocking download - only check existing locations
        val binaryPath = resolveBinaryPathNonBlocking()
        if (binaryPath != null) {
            configureCommandLine(binaryPath)
        }
    }

    private fun configureCommandLine(binaryPath: String) {
        val commandLine = GeneralCommandLine(binaryPath)
            .withWorkDirectory(project.basePath ?: System.getProperty("user.home"))
        setCommandLine(commandLine)
    }

    /**
     * Resolve the path to the agnix-lsp binary without blocking.
     *
     * Checks existing locations only - does NOT trigger download.
     * Download is handled by the LSP4IJ server installer flow.
     */
    private fun resolveBinaryPathNonBlocking(): String? {
        val settings = AgnixSettings.getInstance()

        // Check user-configured path first
        val configuredPath = settings.lspPath
        if (configuredPath.isNotBlank()) {
            val file = File(configuredPath)
            if (file.exists() && file.canExecute()) {
                logger.info("Using configured LSP path: $configuredPath")
                return configuredPath
            }
        }

        // Use cached resolver to check existing binary locations
        val downloadedPath = AgnixBinaryResolver.getDownloadedBinaryPath()
        if (downloadedPath != null) {
            logger.info("Using downloaded LSP binary: $downloadedPath")
            return downloadedPath
        }

        val systemPath = AgnixBinaryResolver.findInPath()
        if (systemPath != null) {
            logger.info("Using system PATH LSP binary: $systemPath")
            return systemPath
        }

        // Binary not found - notify user but do NOT block with download
        logger.warn("agnix-lsp binary not found")
        return null
    }

    override fun start() {
        var commandLine = getCommandLine()

        // On first run, LSP4IJ installer may download agnix-lsp after descriptor init.
        // Re-resolve here so the freshly installed binary can be used immediately.
        if (commandLine == null || !File(commandLine.exePath).exists()) {
            val resolvedPath = resolveBinaryPathNonBlocking()
            if (resolvedPath != null) {
                configureCommandLine(resolvedPath)
                commandLine = getCommandLine()
            }
        }

        if (commandLine == null) {
            logger.error("No LSP command configured - binary not found")
            AgnixNotifications.notifyBinaryNotFound(project)
            return
        }

        val binaryPath = commandLine.exePath
        if (!File(binaryPath).exists()) {
            logger.error("LSP binary not found: $binaryPath")
            AgnixNotifications.notifyBinaryNotFound(project)
            return
        }

        logger.info("Starting agnix-lsp: ${commandLine.commandLineString}")
        super.start()
    }

    override fun stop() {
        logger.info("Stopping agnix-lsp")
        super.stop()
    }

    override fun isAlive(): Boolean {
        return super.isAlive()
    }
}
