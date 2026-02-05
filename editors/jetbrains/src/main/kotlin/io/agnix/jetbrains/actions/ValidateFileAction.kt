package io.agnix.jetbrains.actions

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.fileEditor.FileDocumentManager
import io.agnix.jetbrains.AgnixIcons
import io.agnix.jetbrains.filetype.AgnixFileTypes

/**
 * Action to manually trigger validation on the current file.
 *
 * Forces the LSP server to re-validate the active document.
 */
class ValidateFileAction : AnAction(
    "Validate Current File",
    "Manually trigger agnix validation on the current file",
    AgnixIcons.AGNIX
) {
    private val logger = Logger.getInstance(ValidateFileAction::class.java)

    override fun actionPerformed(e: AnActionEvent) {
        if (e.project == null) return
        val editor = e.getData(CommonDataKeys.EDITOR) ?: return
        val document = editor.document
        val virtualFile = FileDocumentManager.getInstance().getFile(document) ?: return

        logger.info("Validating file: ${virtualFile.path}")

        // Save the document to ensure LSP sees latest content
        FileDocumentManager.getInstance().saveDocument(document)

        // The LSP server will automatically revalidate on document save
        // This action mainly serves to ensure the document is saved and trigger validation
    }

    override fun update(e: AnActionEvent) {
        val editor = e.getData(CommonDataKeys.EDITOR)
        val virtualFile = editor?.let {
            FileDocumentManager.getInstance().getFile(it.document)
        }

        // Enable only for agnix-supported files
        val isAgnixFile = virtualFile?.let {
            AgnixFileTypes.isAgnixFilePath(it.path)
        } ?: false

        e.presentation.isEnabledAndVisible = e.project != null && isAgnixFile
    }
}
