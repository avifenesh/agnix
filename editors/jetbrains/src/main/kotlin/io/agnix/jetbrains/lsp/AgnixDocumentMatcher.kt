package io.agnix.jetbrains.lsp

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.redhat.devtools.lsp4ij.AbstractDocumentMatcher
import io.agnix.jetbrains.filetype.AgnixFileTypes

/**
 * Path-aware matcher to avoid attaching agnix-lsp to unrelated files that share
 * common names like settings.json or plugin.json.
 */
class AgnixDocumentMatcher : AbstractDocumentMatcher() {
    override fun match(virtualFile: VirtualFile, project: Project): Boolean {
        return AgnixFileTypes.isAgnixFilePath(virtualFile.path)
    }
}
