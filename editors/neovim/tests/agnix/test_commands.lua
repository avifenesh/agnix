--- Tests for agnix.commands module.
local commands = require('agnix.commands')
local config = require('agnix.config')
local util = require('agnix.util')

local function test_setup_registers_user_commands()
  -- Ensure config is initialised so commands.setup() can run
  config.setup({})

  commands.setup()

  -- Check that all expected user commands were registered
  local expected_commands = {
    'AgnixStart',
    'AgnixStop',
    'AgnixRestart',
    'AgnixInfo',
    'AgnixValidateFile',
    'AgnixShowRules',
    'AgnixFixAll',
    'AgnixFixSafe',
    'AgnixIgnoreRule',
    'AgnixShowRuleDoc',
  }

  local registered = vim.api.nvim_get_commands({})
  for _, cmd_name in ipairs(expected_commands) do
    assert(registered[cmd_name] ~= nil, 'command ' .. cmd_name .. ' should be registered after setup()')
  end
  config.current = nil
end

local function test_rule_categories_count()
  -- There should be exactly 12 rule categories
  assert(type(commands.rule_categories) == 'table', 'rule_categories should be a table')
  assert(#commands.rule_categories == 12, 'rule_categories should have 12 entries, got: ' .. #commands.rule_categories)
end

local function test_rule_categories_have_required_fields()
  for i, cat in ipairs(commands.rule_categories) do
    assert(type(cat.prefix) == 'string' and cat.prefix ~= '',
      'category ' .. i .. ' should have a non-empty prefix')
    assert(type(cat.name) == 'string' and cat.name ~= '',
      'category ' .. i .. ' should have a non-empty name')
    -- Prefix should end with a dash (convention for rule IDs)
    assert(cat.prefix:sub(-1) == '-',
      'category ' .. i .. ' prefix "' .. cat.prefix .. '" should end with "-"')
  end
end

local function test_rule_categories_all_present()
  -- Verify all known categories are present
  local expected_prefixes = {
    'AS-', 'CC-SK-', 'CC-HK-', 'CC-AG-', 'CC-PL-', 'PE-',
    'MCP-', 'AGM-', 'COP-', 'CUR-', 'XML-', 'XP-',
  }
  local found = {}
  for _, cat in ipairs(commands.rule_categories) do
    found[cat.prefix] = true
  end
  for _, prefix in ipairs(expected_prefixes) do
    assert(found[prefix], 'rule category with prefix "' .. prefix .. '" should be present')
  end
end

local function test_ignore_rule_creates_new_toml()
  -- When .agnix.toml does not exist, _ignore_rule should create it
  local tmp = vim.fn.tempname()
  vim.fn.mkdir(tmp, 'p')
  local toml_path = tmp .. util.path_sep .. '.agnix.toml'

  -- Ensure the file does not exist
  assert(vim.fn.filereadable(toml_path) == 0, '.agnix.toml should not exist yet')

  -- Mock get_root_dir to return our temp dir
  local orig_get_root_dir = util.get_root_dir
  util.get_root_dir = function()
    return tmp
  end

  -- Suppress notifications
  local orig_notify = vim.notify
  vim.notify = function() end

  commands._ignore_rule('AS-001')

  -- Restore mocks
  vim.notify = orig_notify
  util.get_root_dir = orig_get_root_dir

  -- Verify the file was created with the rule
  assert(vim.fn.filereadable(toml_path) == 1, '.agnix.toml should be created')
  local f = io.open(toml_path, 'r')
  assert(f ~= nil, 'should be able to read .agnix.toml')
  local content = f:read('*a')
  f:close()

  assert(content:find('%[rules%]'), '.agnix.toml should contain [rules] section')
  assert(content:find('"AS%-001"'), '.agnix.toml should contain "AS-001" in disabled_rules')

  -- Clean up
  vim.fn.delete(tmp, 'rf')
end

local function test_ignore_rule_adds_to_existing_toml_without_disabled_rules()
  -- When .agnix.toml exists but has no disabled_rules key
  local tmp = vim.fn.tempname()
  vim.fn.mkdir(tmp, 'p')
  local toml_path = tmp .. util.path_sep .. '.agnix.toml'

  local f = io.open(toml_path, 'w')
  f:write('[settings]\nseverity = "Warning"\n')
  f:close()

  local orig_get_root_dir = util.get_root_dir
  util.get_root_dir = function()
    return tmp
  end
  local orig_notify = vim.notify
  vim.notify = function() end

  commands._ignore_rule('PE-003')

  vim.notify = orig_notify
  util.get_root_dir = orig_get_root_dir

  local out = io.open(toml_path, 'r')
  local content = out:read('*a')
  out:close()

  -- Should have both original content and new disabled_rules
  assert(content:find('severity'), 'original content should be preserved')
  assert(content:find('disabled_rules'), 'disabled_rules should be added')
  assert(content:find('"PE%-003"'), 'should contain "PE-003"')

  -- Clean up
  vim.fn.delete(tmp, 'rf')
end

local function test_ignore_rule_appends_to_existing_disabled_rules()
  -- When .agnix.toml already has disabled_rules, append to the array
  local tmp = vim.fn.tempname()
  vim.fn.mkdir(tmp, 'p')
  local toml_path = tmp .. util.path_sep .. '.agnix.toml'

  local f = io.open(toml_path, 'w')
  f:write('[rules]\ndisabled_rules = ["MCP-001"]\n')
  f:close()

  local orig_get_root_dir = util.get_root_dir
  util.get_root_dir = function()
    return tmp
  end
  local orig_notify = vim.notify
  vim.notify = function() end

  commands._ignore_rule('AS-004')

  vim.notify = orig_notify
  util.get_root_dir = orig_get_root_dir

  local out = io.open(toml_path, 'r')
  local content = out:read('*a')
  out:close()

  -- Should contain both rules
  assert(content:find('"MCP%-001"'), 'original rule MCP-001 should be preserved')
  assert(content:find('"AS%-004"'), 'new rule AS-004 should be added')

  -- Clean up
  vim.fn.delete(tmp, 'rf')
end

local function test_ignore_rule_no_duplicate()
  -- When the rule is already disabled, it should not be added again.
  -- Uses hyphenated rule ID to verify plain text matching works correctly.
  local tmp = vim.fn.tempname()
  vim.fn.mkdir(tmp, 'p')
  local toml_path = tmp .. util.path_sep .. '.agnix.toml'

  local original_content = '[rules]\ndisabled_rules = ["XML-001"]\n'
  local f = io.open(toml_path, 'w')
  f:write(original_content)
  f:close()

  local orig_get_root_dir = util.get_root_dir
  util.get_root_dir = function()
    return tmp
  end

  -- Capture notification
  local notify_messages = {}
  local orig_notify = vim.notify
  vim.notify = function(msg)
    notify_messages[#notify_messages + 1] = msg
  end

  commands._ignore_rule('XML-001')

  vim.notify = orig_notify
  util.get_root_dir = orig_get_root_dir

  -- File content should be unchanged
  local out = io.open(toml_path, 'r')
  local content = out:read('*a')
  out:close()
  assert(content == original_content,
    'file should be unchanged when rule already disabled, got: ' .. content)

  -- Should have notified about already being disabled
  local found_already = false
  for _, msg in ipairs(notify_messages) do
    if msg:find('already disabled') then
      found_already = true
      break
    end
  end
  assert(found_already, 'should notify that rule is already disabled')

  -- Clean up
  vim.fn.delete(tmp, 'rf')
end

local function test_show_rules_fallback_callable()
  -- _show_rules_fallback should be callable without error.
  -- We mock vim.ui.select to prevent interactive prompt.
  local orig_select = vim.ui.select
  local select_called = false
  local select_items = nil
  vim.ui.select = function(items, opts, on_choice)
    select_called = true
    select_items = items
    -- Simulate user selecting nothing (cancel)
    on_choice(nil)
  end

  local ok, err = pcall(commands._show_rules_fallback)

  vim.ui.select = orig_select

  assert(ok, '_show_rules_fallback should not error: ' .. tostring(err))
  assert(select_called, 'vim.ui.select should be called')
  assert(type(select_items) == 'table', 'items passed to select should be a table')
  assert(#select_items == 12, 'should offer 12 rule categories, got: ' .. #select_items)
end

local function test_show_rules_fallback_selection()
  -- When user selects an item, should call vim.notify with a URL
  local orig_select = vim.ui.select
  local orig_notify = vim.notify
  local notify_messages = {}

  vim.ui.select = function(items, opts, on_choice)
    -- Simulate selecting the first item
    on_choice(items[1])
  end
  vim.notify = function(msg)
    notify_messages[#notify_messages + 1] = msg
  end

  commands._show_rules_fallback()

  vim.ui.select = orig_select
  vim.notify = orig_notify

  assert(#notify_messages > 0, 'should notify with a URL after selection')
  local found_url = false
  for _, msg in ipairs(notify_messages) do
    if msg:find('VALIDATION%-RULES') then
      found_url = true
      break
    end
  end
  assert(found_url, 'notification should contain a link to VALIDATION-RULES.md')
end

-- Run all tests
test_setup_registers_user_commands()
test_rule_categories_count()
test_rule_categories_have_required_fields()
test_rule_categories_all_present()
test_ignore_rule_creates_new_toml()
test_ignore_rule_adds_to_existing_toml_without_disabled_rules()
test_ignore_rule_appends_to_existing_disabled_rules()
test_ignore_rule_no_duplicate()
test_show_rules_fallback_callable()
test_show_rules_fallback_selection()
