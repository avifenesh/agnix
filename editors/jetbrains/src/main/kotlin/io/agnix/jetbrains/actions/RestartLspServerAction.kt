package io.agnix.jetbrains.actions

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.diagnostic.Logger
import com.redhat.devtools.lsp4ij.LanguageServerManager
import io.agnix.jetbrains.AgnixIcons

/**
 * Action to restart the agnix LSP server.
 *
 * Useful when the server becomes unresponsive or after updating the binary.
 */
class RestartLspServerAction : AnAction(
    "Restart Language Server",
    "Restart the agnix language server",
    AgnixIcons.AGNIX
) {
    private val logger = Logger.getInstance(RestartLspServerAction::class.java)

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return

        logger.info("Restarting agnix LSP server")

        // Use LSP4IJ's LanguageServerManager to restart the server
        val serverManager = LanguageServerManager.getInstance(project)
        serverManager.stop("io.agnix.lsp")

        // The server will be automatically restarted when needed
        logger.info("agnix LSP server stopped, will restart on next request")
    }

    override fun update(e: AnActionEvent) {
        // Always enabled when a project is open
        e.presentation.isEnabledAndVisible = e.project != null
    }
}
