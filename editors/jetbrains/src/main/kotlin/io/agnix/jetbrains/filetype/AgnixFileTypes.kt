package io.agnix.jetbrains.filetype

import com.intellij.openapi.fileTypes.LanguageFileType
import com.intellij.openapi.fileTypes.PlainTextLanguage
import io.agnix.jetbrains.AgnixIcons
import javax.swing.Icon

/**
 * File type for SKILL.md files.
 *
 * Extends Markdown language with custom icon for better identification.
 */
class SkillFileType private constructor() : LanguageFileType(PlainTextLanguage.INSTANCE) {

    override fun getName(): String = "SKILL.md"

    override fun getDescription(): String = "Agent Skills specification file"

    override fun getDefaultExtension(): String = "md"

    override fun getIcon(): Icon = AgnixIcons.AGNIX

    companion object {
        @JvmField
        val INSTANCE = SkillFileType()
    }
}

/**
 * Utility object for checking if a file is an agnix-supported file.
 */
object AgnixFileTypes {

    private val GLOBAL_FILE_NAMES = setOf(
        "SKILL.md",
        "CLAUDE.md",
        "CLAUDE.local.md",
        "AGENTS.md",
        "AGENTS.local.md",
        "AGENTS.override.md",
        "plugin.json",
        "mcp.json"
    )

    private val SUFFIX_PATTERNS = listOf(
        ".mcp.json",
        ".instructions.md",
        ".mdc"
    )

    private val CLAUDE_SETTINGS_FILE_NAMES = setOf(
        "settings.json",
        "settings.local.json"
    )

    private val GITHUB_FILE_NAMES = setOf(
        "copilot-instructions.md"
    )

    /**
     * Check if a file path matches agnix patterns.
     *
     * Takes into account parent directory requirements.
     */
    fun isAgnixFilePath(path: String): Boolean {
        val normalizedPath = path.replace('\\', '/')
        val fileName = normalizedPath.substringAfterLast('/')

        // Check exact file names at any level
        if (fileName in GLOBAL_FILE_NAMES) {
            return true
        }

        // Check extension patterns
        if (SUFFIX_PATTERNS.any { fileName.endsWith(it) }) {
            return true
        }

        // Check directory-specific patterns
        if (normalizedPath.contains("/.claude/") && fileName in CLAUDE_SETTINGS_FILE_NAMES) {
            return true
        }
        if (normalizedPath.contains("/.github/") && fileName in GITHUB_FILE_NAMES) {
            return true
        }
        if (normalizedPath.contains("/.github/instructions/") && fileName.endsWith(".instructions.md")) {
            return true
        }
        if (normalizedPath.contains("/.cursor/rules/") && fileName.endsWith(".mdc")) {
            return true
        }
        if (fileName == ".cursorrules") {
            return true
        }

        return false
    }
}
