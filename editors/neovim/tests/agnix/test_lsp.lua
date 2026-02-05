--- Tests for agnix.lsp module - specifically build_lsp_settings().
local config = require('agnix.config')
local lsp = require('agnix.lsp')

local function test_build_lsp_settings_empty()
  -- With defaults (all nil), should return empty table
  config.setup({})
  local settings = lsp.build_lsp_settings()
  assert(type(settings) == 'table', 'settings should be a table')
  assert(next(settings) == nil, 'settings should be empty when all values are nil')
  config.current = nil
end

local function test_build_lsp_settings_with_severity()
  config.setup({
    settings = {
      severity = 'Error',
    },
  })
  local settings = lsp.build_lsp_settings()
  assert(settings.severity == 'Error', 'severity should be set')
  assert(settings.target == nil, 'target should not be present')
  assert(settings.rules == nil, 'rules should not be present (all nil)')
  config.current = nil
end

local function test_build_lsp_settings_with_rules()
  config.setup({
    settings = {
      rules = {
        skills = false,
        hooks = true,
        disabled_rules = { 'AS-001', 'PE-003' },
      },
    },
  })
  local settings = lsp.build_lsp_settings()
  assert(type(settings.rules) == 'table', 'rules should be present')
  assert(settings.rules.skills == false, 'rules.skills should be false')
  assert(settings.rules.hooks == true, 'rules.hooks should be true')
  assert(type(settings.rules.disabled_rules) == 'table', 'disabled_rules should be a table')
  assert(#settings.rules.disabled_rules == 2, 'disabled_rules should have 2 entries')
  -- Nil rules should be absent
  assert(settings.rules.agents == nil, 'nil rules should be absent')
  config.current = nil
end

local function test_build_lsp_settings_with_versions()
  config.setup({
    settings = {
      versions = {
        claude_code = '1.0.0',
      },
    },
  })
  local settings = lsp.build_lsp_settings()
  assert(type(settings.versions) == 'table', 'versions should be present')
  assert(settings.versions.claude_code == '1.0.0', 'claude_code should be set')
  assert(settings.versions.codex == nil, 'codex should be absent')
  config.current = nil
end

local function test_build_lsp_settings_with_specs()
  config.setup({
    settings = {
      specs = {
        mcp_protocol = '2025-06-18',
        agent_skills_spec = '1.0',
      },
    },
  })
  local settings = lsp.build_lsp_settings()
  assert(type(settings.specs) == 'table', 'specs should be present')
  assert(settings.specs.mcp_protocol == '2025-06-18', 'mcp_protocol should be set')
  assert(settings.specs.agent_skills_spec == '1.0', 'agent_skills_spec should be set')
  assert(settings.specs.agents_md_spec == nil, 'agents_md_spec should be absent')
  config.current = nil
end

local function test_build_lsp_settings_full()
  config.setup({
    settings = {
      severity = 'Warning',
      target = 'ClaudeCode',
      tools = { 'claude-code', 'cursor' },
      rules = {
        skills = true,
        hooks = false,
        disabled_rules = { 'MCP-008' },
      },
      versions = {
        claude_code = '1.0.0',
        codex = '0.1.0',
      },
      specs = {
        mcp_protocol = '2025-06-18',
      },
    },
  })
  local settings = lsp.build_lsp_settings()
  assert(settings.severity == 'Warning', 'severity should be set')
  assert(settings.target == 'ClaudeCode', 'target should be set')
  assert(type(settings.tools) == 'table', 'tools should be a table')
  assert(#settings.tools == 2, 'tools should have 2 entries')
  assert(settings.rules.skills == true, 'rules.skills should be true')
  assert(settings.rules.hooks == false, 'rules.hooks should be false')
  assert(settings.versions.claude_code == '1.0.0', 'versions.claude_code should be set')
  assert(settings.specs.mcp_protocol == '2025-06-18', 'specs.mcp_protocol should be set')
  config.current = nil
end

-- Run all tests
test_build_lsp_settings_empty()
test_build_lsp_settings_with_severity()
test_build_lsp_settings_with_rules()
test_build_lsp_settings_with_versions()
test_build_lsp_settings_with_specs()
test_build_lsp_settings_full()
