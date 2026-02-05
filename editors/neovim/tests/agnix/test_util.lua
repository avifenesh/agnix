--- Tests for agnix.util module.
local util = require('agnix.util')

local function test_is_agnix_file()
  -- SKILL.md
  assert(util.is_agnix_file('SKILL.md'), 'SKILL.md should match')
  assert(util.is_agnix_file('/home/user/project/SKILL.md'), 'absolute SKILL.md should match')

  -- Memory files
  assert(util.is_agnix_file('CLAUDE.md'), 'CLAUDE.md should match')
  assert(util.is_agnix_file('CLAUDE.local.md'), 'CLAUDE.local.md should match')
  assert(util.is_agnix_file('AGENTS.md'), 'AGENTS.md should match')
  assert(util.is_agnix_file('AGENTS.local.md'), 'AGENTS.local.md should match')
  assert(util.is_agnix_file('AGENTS.override.md'), 'AGENTS.override.md should match')

  -- Hook settings
  assert(
    util.is_agnix_file('/project/.claude/settings.json'),
    '.claude/settings.json should match'
  )
  assert(
    util.is_agnix_file('/project/.claude/settings.local.json'),
    '.claude/settings.local.json should match'
  )
  -- settings.json NOT under .claude should NOT match
  assert(
    not util.is_agnix_file('/project/other/settings.json'),
    'settings.json outside .claude should not match'
  )

  -- Plugin manifest
  assert(util.is_agnix_file('plugin.json'), 'plugin.json should match')
  assert(util.is_agnix_file('/project/plugin.json'), 'absolute plugin.json should match')

  -- MCP files
  assert(util.is_agnix_file('mcp.json'), 'mcp.json should match')
  assert(util.is_agnix_file('server.mcp.json'), '*.mcp.json should match')
  assert(util.is_agnix_file('tools.mcp.json'), 'tools.mcp.json should match')
  assert(util.is_agnix_file('mcp-server.json'), 'mcp-*.json should match')
  assert(util.is_agnix_file('mcp-tools.json'), 'mcp-tools.json should match')

  -- Copilot instructions
  assert(
    util.is_agnix_file('/project/.github/copilot-instructions.md'),
    '.github/copilot-instructions.md should match'
  )
  assert(
    not util.is_agnix_file('/project/copilot-instructions.md'),
    'copilot-instructions.md outside .github should not match'
  )

  -- Copilot scoped instructions
  assert(
    util.is_agnix_file('/project/.github/instructions/typescript.instructions.md'),
    '.github/instructions/*.instructions.md should match'
  )
  assert(
    not util.is_agnix_file('/project/instructions/typescript.instructions.md'),
    'instructions/*.instructions.md outside .github should not match'
  )

  -- Cursor rules
  assert(
    util.is_agnix_file('/project/.cursor/rules/typescript.mdc'),
    '.cursor/rules/*.mdc should match'
  )
  assert(
    not util.is_agnix_file('/project/rules/typescript.mdc'),
    '*.mdc outside .cursor/rules should not match'
  )

  -- Legacy .cursorrules
  assert(util.is_agnix_file('.cursorrules'), '.cursorrules should match')
  assert(util.is_agnix_file('/project/.cursorrules'), 'absolute .cursorrules should match')

  -- Agent files
  assert(
    util.is_agnix_file('/project/.claude/agents/researcher.md'),
    '.claude/agents/*.md should match'
  )
  assert(
    util.is_agnix_file('/project/agents/helper.md'),
    'agents/*.md should match'
  )

  -- Non-matching files
  assert(not util.is_agnix_file('README.md'), 'README.md should not match')
  assert(not util.is_agnix_file('package.json'), 'package.json should not match')
  assert(not util.is_agnix_file('src/main.rs'), 'main.rs should not match')
  assert(not util.is_agnix_file(''), 'empty string should not match')
  assert(not util.is_agnix_file(nil), 'nil should not match')
end

local function test_is_agnix_file_windows_paths()
  -- Windows-style paths with backslashes
  assert(
    util.is_agnix_file('C:\\Users\\user\\project\\SKILL.md'),
    'Windows SKILL.md path should match'
  )
  assert(
    util.is_agnix_file('C:\\project\\.claude\\settings.json'),
    'Windows .claude\\settings.json should match'
  )
  assert(
    util.is_agnix_file('C:\\project\\.github\\copilot-instructions.md'),
    'Windows .github\\copilot-instructions.md should match'
  )
  assert(
    util.is_agnix_file('C:\\project\\.cursor\\rules\\test.mdc'),
    'Windows .cursor\\rules\\*.mdc should match'
  )
  assert(
    util.is_agnix_file('C:\\project\\.github\\instructions\\ts.instructions.md'),
    'Windows .github\\instructions\\*.instructions.md should match'
  )
end

local function test_find_binary_explicit_valid()
  -- When an explicit cmd is executable, it should be returned directly
  -- We test with a known executable (nvim itself)
  local nvim_path = vim.v.progpath
  local result = util.find_binary({ cmd = nvim_path })
  assert(result == nvim_path, 'valid explicit cmd should be returned as-is')
end

local function test_find_binary_explicit_invalid_falls_through()
  -- When an explicit cmd is NOT executable, the function falls through to
  -- PATH and cargo bin searches. The result depends on environment.
  local result = util.find_binary({ cmd = '/nonexistent/path/to/agnix-lsp' })
  assert(result == nil or type(result) == 'string',
    'invalid explicit cmd should fall through gracefully')
end

local function test_find_binary_no_opts()
  -- Should not error with nil opts
  local result = util.find_binary(nil)
  -- Result may or may not be nil depending on environment, but should not error
  assert(result == nil or type(result) == 'string', 'find_binary(nil) should return string or nil')
end

-- Run all tests
test_is_agnix_file()
test_is_agnix_file_windows_paths()
test_find_binary_explicit_valid()
test_find_binary_explicit_invalid_falls_through()
test_find_binary_no_opts()
