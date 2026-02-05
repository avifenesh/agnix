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

    /**
     * File name patterns that agnix supports.
     */
    private val SUPPORTED_PATTERNS = listOf(
        "SKILL.md",
        "CLAUDE.md",
        "CLAUDE.local.md",
        "AGENTS.md",
        "AGENTS.local.md",
        "settings.json",
        "settings.local.json",
        "plugin.json",
        "mcp.json",
        "copilot-instructions.md",
        ".cursorrules"
    )

    /**
     * File extension patterns that agnix supports.
     */
    private val SUPPORTED_EXTENSIONS = listOf(
        ".mcp.json",
        ".instructions.md",
        ".mdc"
    )

    /**
     * Directory patterns for file matching.
     */
    private val DIRECTORY_PATTERNS = mapOf(
        ".claude" to listOf("settings.json", "settings.local.json"),
        ".github" to listOf("copilot-instructions.md"),
        ".github/instructions" to listOf(".instructions.md"),
        ".cursor/rules" to listOf(".mdc")
    )

    /**
     * Check if a file name matches agnix patterns.
     */
    fun isAgnixFile(fileName: String): Boolean {
        // Check exact file names
        if (fileName in SUPPORTED_PATTERNS) {
            return true
        }

        // Check extension patterns
        for (ext in SUPPORTED_EXTENSIONS) {
            if (fileName.endsWith(ext)) {
                return true
            }
        }

        return false
    }

    /**
     * Check if a file path matches agnix patterns.
     *
     * Takes into account parent directory requirements.
     */
    fun isAgnixFilePath(path: String): Boolean {
        val normalizedPath = path.replace('\\', '/')
        val fileName = normalizedPath.substringAfterLast('/')

        // Check exact file names at any level
        if (fileName in listOf("SKILL.md", "CLAUDE.md", "CLAUDE.local.md", "AGENTS.md", "AGENTS.local.md", "plugin.json", "mcp.json")) {
            return true
        }

        // Check extension patterns
        if (fileName.endsWith(".mcp.json") || fileName.endsWith(".instructions.md") || fileName.endsWith(".mdc")) {
            return true
        }

        // Check directory-specific patterns
        if (normalizedPath.contains("/.claude/") && (fileName == "settings.json" || fileName == "settings.local.json")) {
            return true
        }
        if (normalizedPath.contains("/.github/") && fileName == "copilot-instructions.md") {
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
