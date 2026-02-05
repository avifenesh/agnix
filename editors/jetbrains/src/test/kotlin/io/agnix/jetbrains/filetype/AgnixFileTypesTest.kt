package io.agnix.jetbrains.filetype

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.*

/**
 * Tests for AgnixFileTypes utility.
 */
class AgnixFileTypesTest {

    @Test
    fun `isAgnixFile returns true for SKILL md`() {
        assertTrue(AgnixFileTypes.isAgnixFile("SKILL.md"))
    }

    @Test
    fun `isAgnixFile returns true for CLAUDE md`() {
        assertTrue(AgnixFileTypes.isAgnixFile("CLAUDE.md"))
    }

    @Test
    fun `isAgnixFile returns true for CLAUDE local md`() {
        assertTrue(AgnixFileTypes.isAgnixFile("CLAUDE.local.md"))
    }

    @Test
    fun `isAgnixFile returns true for AGENTS md`() {
        assertTrue(AgnixFileTypes.isAgnixFile("AGENTS.md"))
    }

    @Test
    fun `isAgnixFile returns true for AGENTS local md`() {
        assertTrue(AgnixFileTypes.isAgnixFile("AGENTS.local.md"))
    }

    @Test
    fun `isAgnixFile returns true for mcp json files`() {
        assertTrue(AgnixFileTypes.isAgnixFile("server.mcp.json"))
        assertTrue(AgnixFileTypes.isAgnixFile("mcp.json"))
    }

    @Test
    fun `isAgnixFile returns true for plugin json`() {
        assertTrue(AgnixFileTypes.isAgnixFile("plugin.json"))
    }

    @Test
    fun `isAgnixFile returns true for instructions md files`() {
        assertTrue(AgnixFileTypes.isAgnixFile("copilot-instructions.md"))
        assertTrue(AgnixFileTypes.isAgnixFile("custom.instructions.md"))
    }

    @Test
    fun `isAgnixFile returns true for mdc files`() {
        assertTrue(AgnixFileTypes.isAgnixFile("rule.mdc"))
    }

    @Test
    fun `isAgnixFile returns true for cursorrules`() {
        assertTrue(AgnixFileTypes.isAgnixFile(".cursorrules"))
    }

    @Test
    fun `isAgnixFile returns false for random files`() {
        assertFalse(AgnixFileTypes.isAgnixFile("random.md"))
        assertFalse(AgnixFileTypes.isAgnixFile("package.json"))
        assertFalse(AgnixFileTypes.isAgnixFile("config.yaml"))
    }

    @Test
    fun `isAgnixFilePath returns true for SKILL md in any directory`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/SKILL.md"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/subdir/SKILL.md"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("C:\\project\\SKILL.md"))
    }

    @Test
    fun `isAgnixFilePath returns true for claude settings in correct directory`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.claude/settings.json"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.claude/settings.local.json"))
    }

    @Test
    fun `isAgnixFilePath returns false for settings json outside claude directory`() {
        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/settings.json"))
        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/config/settings.json"))
    }

    @Test
    fun `isAgnixFilePath returns true for copilot instructions in github directory`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.github/copilot-instructions.md"))
    }

    @Test
    fun `isAgnixFilePath returns true for custom instructions in github instructions directory`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.github/instructions/custom.instructions.md"))
    }

    @Test
    fun `isAgnixFilePath returns true for mdc files in cursor rules directory`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("/project/.cursor/rules/rule.mdc"))
    }

    @Test
    fun `isAgnixFilePath handles Windows paths`() {
        assertTrue(AgnixFileTypes.isAgnixFilePath("C:\\project\\SKILL.md"))
        assertTrue(AgnixFileTypes.isAgnixFilePath("C:\\project\\.claude\\settings.json"))
    }

    @Test
    fun `isAgnixFilePath returns false for unrelated paths`() {
        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/src/main.rs"))
        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/package.json"))
        assertFalse(AgnixFileTypes.isAgnixFilePath("/project/README.md"))
    }
}
