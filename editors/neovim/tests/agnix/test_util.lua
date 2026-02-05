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

local function test_get_root_dir_exists_and_callable()
  -- get_root_dir should be a function exposed on the module
  assert(type(util.get_root_dir) == 'function', 'get_root_dir should be a function')
end

local function test_get_root_dir_returns_nil_for_path_with_no_markers()
  -- When vim.fs.find returns nothing for both file and directory searches,
  -- get_root_dir should return nil.
  -- We mock vim.fs.find because real temp paths may sit under a directory
  -- tree that contains .git (e.g. the user home).
  local orig_find = vim.fs.find
  vim.fs.find = function()
    return {}
  end

  local result = util.get_root_dir('/some/isolated/path/with/no/markers')
  assert(result == nil, 'get_root_dir should return nil when no markers exist, got: ' .. tostring(result))

  -- Restore
  vim.fs.find = orig_find
end

local function test_get_root_dir_finds_git_marker()
  -- When vim.fs.find returns a .git directory, get_root_dir should return
  -- its parent (the project root). We mock vim.fs.find to avoid interference
  -- from .git directories elsewhere in the filesystem.
  local orig_find = vim.fs.find
  local call_count = 0
  vim.fs.find = function(markers, opts)
    call_count = call_count + 1
    if call_count == 1 then
      -- First call searches for type='file'; .git is a directory, so no match
      return {}
    end
    -- Second call searches for type='directory'; return .git path
    return { '/mock/project/.git' }
  end

  local result = util.get_root_dir('/mock/project/src/deep')

  vim.fs.find = orig_find

  assert(result ~= nil, 'get_root_dir should find .git marker')
  local normalized = result:gsub('\\', '/')
  assert(
    normalized == '/mock/project',
    'get_root_dir should return parent of .git, got: ' .. tostring(result)
  )
end

local function test_get_root_dir_finds_file_marker()
  -- When vim.fs.find returns a file marker (e.g. .agnix.toml) on the first
  -- call (type='file'), get_root_dir should return its parent directory.
  local orig_find = vim.fs.find
  vim.fs.find = function(markers, opts)
    if opts and opts.type == 'file' then
      return { '/mock/project/.agnix.toml' }
    end
    return {}
  end

  local result = util.get_root_dir('/mock/project/nested')

  vim.fs.find = orig_find

  assert(result ~= nil, 'get_root_dir should find .agnix.toml marker')
  local normalized = result:gsub('\\', '/')
  assert(
    normalized == '/mock/project',
    'get_root_dir should return parent of .agnix.toml, got: ' .. tostring(result)
  )
end

local function test_get_root_dir_finds_claude_md_marker()
  -- When vim.fs.find returns a CLAUDE.md file, get_root_dir should return
  -- its parent directory.
  local orig_find = vim.fs.find
  vim.fs.find = function(markers, opts)
    if opts and opts.type == 'file' then
      return { '/mock/project/CLAUDE.md' }
    end
    return {}
  end

  local result = util.get_root_dir('/mock/project/lib')

  vim.fs.find = orig_find

  assert(result ~= nil, 'get_root_dir should find CLAUDE.md marker')
  local normalized = result:gsub('\\', '/')
  assert(
    normalized == '/mock/project',
    'get_root_dir should return parent of CLAUDE.md, got: ' .. tostring(result)
  )
end

local function test_get_root_dir_prefers_file_over_directory()
  -- When a file marker is found on the first call, the directory search
  -- should not be needed.
  local orig_find = vim.fs.find
  local calls = {}
  vim.fs.find = function(markers, opts)
    calls[#calls + 1] = opts and opts.type or 'unknown'
    if opts and opts.type == 'file' then
      return { '/mock/project/AGENTS.md' }
    end
    return { '/mock/other/.git' }
  end

  local result = util.get_root_dir('/mock/project/src')

  vim.fs.find = orig_find

  assert(result ~= nil, 'get_root_dir should find file marker')
  -- The file search found a result, so only one call should have been made
  assert(#calls == 1, 'should only call vim.fs.find once when file marker found, got: ' .. #calls)
  local normalized = result:gsub('\\', '/')
  assert(
    normalized == '/mock/project',
    'get_root_dir should return parent of AGENTS.md, got: ' .. tostring(result)
  )
end

-- Run all tests
test_is_agnix_file()
test_is_agnix_file_windows_paths()
test_find_binary_explicit_valid()
test_find_binary_explicit_invalid_falls_through()
test_find_binary_no_opts()
test_get_root_dir_exists_and_callable()
test_get_root_dir_returns_nil_for_path_with_no_markers()
test_get_root_dir_finds_git_marker()
test_get_root_dir_finds_file_marker()
test_get_root_dir_finds_claude_md_marker()
test_get_root_dir_prefers_file_over_directory()
