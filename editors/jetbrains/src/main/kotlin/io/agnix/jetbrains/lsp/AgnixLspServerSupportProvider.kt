package io.agnix.jetbrains.lsp

import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerFactory
import com.redhat.devtools.lsp4ij.client.LanguageClientImpl
import com.redhat.devtools.lsp4ij.installation.ServerInstaller
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider

/**
 * LSP server factory for agnix.
 *
 * This class is responsible for creating LSP server connections using LSP4IJ.
 * It resolves the agnix-lsp binary location and creates a stdio-based connection.
 */
class AgnixLspServerSupportProvider : LanguageServerFactory {

    override fun createConnectionProvider(project: Project): StreamConnectionProvider {
        return AgnixLspServerDescriptor(project)
    }

    override fun createLanguageClient(project: Project): LanguageClientImpl {
        return AgnixLanguageClient(project)
    }

    override fun createServerInstaller(): ServerInstaller {
        return AgnixLspServerInstaller()
    }

    /**
     * Custom language client for agnix-specific features.
     */
    class AgnixLanguageClient(project: Project) : LanguageClientImpl(project) {
        // Can be extended for custom notifications or requests
    }
}
