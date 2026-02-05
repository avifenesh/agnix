--- Tests for agnix.config module.
local config = require('agnix.config')

local function test_defaults_structure()
  local d = config.defaults
  assert(d.cmd == nil, 'default cmd should be nil')
  assert(type(d.filetypes) == 'table', 'default filetypes should be a table')
  assert(#d.filetypes == 2, 'default filetypes should have 2 entries')
  assert(d.filetypes[1] == 'markdown', 'first filetype should be markdown')
  assert(d.filetypes[2] == 'json', 'second filetype should be json')
  assert(type(d.root_markers) == 'table', 'default root_markers should be a table')
  assert(d.autostart == true, 'default autostart should be true')
  assert(d.on_attach == nil, 'default on_attach should be nil')
  assert(type(d.settings) == 'table', 'default settings should be a table')
  assert(d.log_level == 'warn', 'default log_level should be warn')
  assert(type(d.telescope) == 'table', 'default telescope should be a table')
  assert(d.telescope.enable == true, 'default telescope.enable should be true')
end

local function test_defaults_settings()
  local s = config.defaults.settings
  assert(s.severity == nil, 'default severity should be nil')
  assert(s.target == nil, 'default target should be nil')
  assert(s.tools == nil, 'default tools should be nil')
  assert(type(s.rules) == 'table', 'default rules should be a table')
  assert(s.rules.skills == nil, 'default rules.skills should be nil')
  assert(s.rules.hooks == nil, 'default rules.hooks should be nil')
  assert(s.rules.disabled_rules == nil, 'default rules.disabled_rules should be nil')
  assert(type(s.versions) == 'table', 'default versions should be a table')
  assert(s.versions.claude_code == nil, 'default versions.claude_code should be nil')
  assert(type(s.specs) == 'table', 'default specs should be a table')
  assert(s.specs.mcp_protocol == nil, 'default specs.mcp_protocol should be nil')
end

local function test_setup_empty_opts()
  config.setup({})
  assert(config.current ~= nil, 'current should be set after setup')
  assert(config.current.autostart == true, 'empty opts should preserve defaults')
  assert(config.current.log_level == 'warn', 'empty opts should preserve log_level default')
  -- Reset
  config.current = nil
end

local function test_setup_merges_user_opts()
  config.setup({
    cmd = '/custom/agnix-lsp',
    autostart = false,
    settings = {
      severity = 'Error',
      rules = {
        skills = false,
      },
    },
  })
  assert(config.current.cmd == '/custom/agnix-lsp', 'cmd should be overridden')
  assert(config.current.autostart == false, 'autostart should be overridden')
  assert(config.current.settings.severity == 'Error', 'severity should be overridden')
  assert(config.current.settings.rules.skills == false, 'rules.skills should be overridden')
  -- Other settings should remain nil (defaults)
  assert(config.current.settings.rules.hooks == nil, 'rules.hooks should remain nil')
  assert(config.current.settings.target == nil, 'target should remain nil')
  -- Non-overridden top-level defaults should remain
  assert(config.current.log_level == 'warn', 'log_level should remain default')
  assert(#config.current.filetypes == 2, 'filetypes should remain default')
  -- Reset
  config.current = nil
end

local function test_setup_deep_merge_preserves_nested()
  config.setup({
    settings = {
      versions = {
        claude_code = '1.0.0',
      },
    },
  })
  assert(config.current.settings.versions.claude_code == '1.0.0', 'version should be set')
  assert(config.current.settings.versions.codex == nil, 'unset version should remain nil')
  assert(config.current.settings.rules.skills == nil, 'unset rule should remain nil')
  -- Reset
  config.current = nil
end

-- Run all tests
test_defaults_structure()
test_defaults_settings()
test_setup_empty_opts()
test_setup_merges_user_opts()
test_setup_deep_merge_preserves_nested()
