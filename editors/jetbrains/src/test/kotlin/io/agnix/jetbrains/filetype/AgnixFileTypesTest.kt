package io.agnix.jetbrains.filetype

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

/**
 * Tests for path-aware agnix file detection.
 */
class AgnixFileTypesTest {

    @Test
    fun `matches top-level markdown memory files`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/SKILL.md"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/CLAUDE.md"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/CLAUDE.local.md"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/AGENTS.md"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/AGENTS.local.md"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/AGENTS.override.md"))
    }

    @Test
    fun `matches json and extension-based agnix files`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/mcp.json"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/server.mcp.json"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/plugin.json"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.cursor/rules/rule.mdc"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.github/instructions/custom.instructions.md"))
    }

    @Test
    fun `matches claude settings only under dot claude directory`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.claude/settings.json"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.claude/settings.local.json"))

        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/settings.json"))
        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/config/settings.local.json"))
    }

    @Test
    fun `matches copilot instructions only under github directory`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.github/copilot-instructions.md"))
        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/copilot-instructions.md"))
    }

    @Test
    fun `matches cursorrules at any path level`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.cursorrules"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/subdir/.cursorrules"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("C:\\project\\.cursorrules"))
    }

    @Test
    fun `supports windows paths`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("C:\\project\\SKILL.md"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("C:\\project\\.claude\\settings.json"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("C:\\project\\.cursor\\rules\\rule.mdc"))
    }

    @Test
    fun `does not match unrelated files`() {
        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/README.md"))
        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/src/main.rs"))
        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/package.json"))
    }
}
