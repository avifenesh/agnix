--- Tests for agnix init module (main entry point).
--- Note: require('agnix') returns the init module.
--- Because the module caches _initialized in an upvalue, we must be careful
--- about ordering: setup() can only run once per Lua VM session.

local function test_module_exports()
  -- The module should expose setup, start, stop, restart as functions
  local agnix = require('agnix')
  assert(type(agnix.setup) == 'function', 'agnix.setup should be a function')
  assert(type(agnix.start) == 'function', 'agnix.start should be a function')
  assert(type(agnix.stop) == 'function', 'agnix.stop should be a function')
  assert(type(agnix.restart) == 'function', 'agnix.restart should be a function')
end

local function test_setup_sets_initialized()
  -- Before calling setup, config.current should be nil (fresh state)
  local config = require('agnix.config')
  config.current = nil

  -- Mock lsp.start to prevent actual binary lookup
  local lsp_mod = require('agnix.lsp')
  local orig_start = lsp_mod.start
  lsp_mod.start = function() return nil end

  local agnix = require('agnix')
  agnix.setup({
    autostart = false, -- prevent immediate start attempt
  })

  -- After setup, config.current should be populated
  assert(config.current ~= nil, 'config.current should be set after setup()')
  assert(config.current.autostart == false, 'autostart should be false as configured')

  -- Restore
  lsp_mod.start = orig_start
end

local function test_double_setup_guard()
  -- Calling setup a second time should be a no-op (the _initialized guard).
  -- We verify by setting a distinctive value first, then calling setup with
  -- different options -- the config should NOT change.
  local config = require('agnix.config')
  local agnix = require('agnix')

  -- First setup already ran in test_setup_sets_initialized with autostart=false.
  -- Attempt a second setup with autostart=true.
  agnix.setup({
    autostart = true,
    log_level = 'debug',
  })

  -- config.current should still reflect the FIRST setup call
  assert(
    config.current.autostart == false,
    'double setup should be blocked; autostart should still be false, got: ' .. tostring(config.current.autostart)
  )
  assert(
    config.current.log_level == 'warn',
    'double setup should be blocked; log_level should still be warn, got: ' .. tostring(config.current.log_level)
  )
end

local function test_start_delegates_to_lsp()
  -- M.start() should delegate to require('agnix.lsp').start()
  local lsp_mod = require('agnix.lsp')
  local start_called = false
  local orig_start = lsp_mod.start
  lsp_mod.start = function()
    start_called = true
    return nil
  end

  local agnix = require('agnix')
  agnix.start()

  lsp_mod.start = orig_start

  assert(start_called, 'agnix.start() should delegate to lsp.start()')
end

local function test_stop_delegates_to_lsp()
  local lsp_mod = require('agnix.lsp')
  local stop_called = false
  local orig_stop = lsp_mod.stop
  lsp_mod.stop = function()
    stop_called = true
  end

  local agnix = require('agnix')
  agnix.stop()

  lsp_mod.stop = orig_stop

  assert(stop_called, 'agnix.stop() should delegate to lsp.stop()')
end

local function test_restart_delegates_to_lsp()
  local lsp_mod = require('agnix.lsp')
  local restart_called = false
  local orig_restart = lsp_mod.restart
  lsp_mod.restart = function()
    restart_called = true
  end

  local agnix = require('agnix')
  agnix.restart()

  lsp_mod.restart = orig_restart

  assert(restart_called, 'agnix.restart() should delegate to lsp.restart()')
end

-- Run all tests
test_module_exports()
test_setup_sets_initialized()
test_double_setup_guard()
test_start_delegates_to_lsp()
test_stop_delegates_to_lsp()
test_restart_delegates_to_lsp()
