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

local function test_setup_autocommands_creates_augroup()
  -- Calling setup_autocommands should create an augroup named 'agnix'
  config.setup({})
  lsp.setup_autocommands()

  -- Get autocmds in the 'agnix' group; if the group does not exist this errors
  local ok, autocmds = pcall(vim.api.nvim_get_autocmds, { group = 'agnix' })
  assert(ok, 'augroup "agnix" should exist after setup_autocommands()')
  assert(type(autocmds) == 'table', 'autocmds should be a table')
  assert(#autocmds > 0, 'there should be at least one autocmd in the agnix group')

  -- Verify the autocmd listens for BufReadPost or FileType
  local found_event = false
  for _, ac in ipairs(autocmds) do
    if ac.event == 'BufReadPost' or ac.event == 'FileType' then
      found_event = true
      break
    end
  end
  assert(found_event, 'agnix augroup should contain a BufReadPost or FileType autocmd')
  config.current = nil
end

local function test_mdc_filetype_registered()
  -- After calling setup_autocommands(), .mdc should be registered as markdown.
  config.setup({})
  lsp.setup_autocommands()
  local tmp = vim.fn.tempname() .. '.mdc'
  local f = io.open(tmp, 'w')
  if f then
    f:write('# Test mdc file\n')
    f:close()
  end

  -- Open the file in a buffer
  vim.cmd('edit ' .. vim.fn.fnameescape(tmp))
  local bufnr = vim.api.nvim_get_current_buf()

  -- Trigger filetype detection
  vim.cmd('filetype detect')
  local ft = vim.bo[bufnr].filetype

  assert(ft == 'markdown', '.mdc file should have markdown filetype, got: ' .. tostring(ft))

  -- Clean up
  vim.cmd('bdelete!')
  vim.fn.delete(tmp)
end

local function test_start_warns_when_binary_not_found()
  -- When binary is not found, start() should return nil (not crash).
  -- We mock util.find_binary to guarantee it returns nil, regardless of
  -- whether agnix-lsp is actually installed on this machine.
  config.setup({})
  local util = require('agnix.util')
  local orig_find_binary = util.find_binary
  util.find_binary = function()
    return nil
  end

  -- Capture notifications to verify a warning is emitted
  local notifications = {}
  local original_notify = vim.notify
  vim.notify = function(msg, level)
    notifications[#notifications + 1] = { msg = msg, level = level }
  end

  local result = lsp.start()

  -- Restore mocks
  vim.notify = original_notify
  util.find_binary = orig_find_binary

  assert(result == nil, 'start() should return nil when binary is not found')
  assert(#notifications > 0, 'start() should emit a notification when binary not found')

  local found_warning = false
  for _, n in ipairs(notifications) do
    if n.msg:find('not found') then
      found_warning = true
      break
    end
  end
  assert(found_warning, 'notification should mention binary not found')
  config.current = nil
end

local function test_stop_is_safe_when_not_started()
  -- Stopping when no client is running should not error
  lsp.client_id = nil
  local ok, err = pcall(lsp.stop)
  assert(ok, 'stop() should not error when no client is running: ' .. tostring(err))
  assert(lsp.client_id == nil, 'client_id should remain nil after stop()')
end

-- Run all tests
test_build_lsp_settings_empty()
test_build_lsp_settings_with_severity()
test_build_lsp_settings_with_rules()
test_build_lsp_settings_with_versions()
test_build_lsp_settings_with_specs()
test_build_lsp_settings_full()
test_setup_autocommands_creates_augroup()
test_mdc_filetype_registered()
test_start_warns_when_binary_not_found()
test_stop_is_safe_when_not_started()
