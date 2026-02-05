--- Minimal Neovim init for running agnix tests.
--- Usage: nvim --headless -u tests/minimal_init.lua -c "lua run_tests()"

-- Add the plugin to the runtime path
local plugin_root = vim.fn.fnamemodify(debug.getinfo(1, 'S').source:sub(2), ':h:h')
vim.opt.rtp:prepend(plugin_root)

-- Disable swap files and other noise
vim.o.swapfile = false
vim.o.backup = false
vim.o.writebackup = false

--- Simple test runner that discovers and executes test files.
--- Tests use assert() for assertions. A failing test raises an error.
function run_tests()
  local test_dir = plugin_root .. '/tests/agnix'
  local test_files = vim.fn.glob(test_dir .. '/test_*.lua', false, true)

  local passed = 0
  local failed = 0
  local errors = {}

  for _, file in ipairs(test_files) do
    local name = vim.fn.fnamemodify(file, ':t:r')
    local ok, err = pcall(dofile, file)
    if ok then
      passed = passed + 1
      print('  PASS: ' .. name)
    else
      failed = failed + 1
      errors[#errors + 1] = { name = name, err = tostring(err) }
      print('  FAIL: ' .. name .. ' - ' .. tostring(err))
    end
  end

  print('')
  print(string.format('Results: %d passed, %d failed, %d total', passed, failed, passed + failed))

  if #errors > 0 then
    print('')
    print('Failures:')
    for _, e in ipairs(errors) do
      print('  ' .. e.name .. ':')
      print('    ' .. e.err)
    end
    -- Exit with error code for CI
    vim.cmd('cquit! 1')
  else
    vim.cmd('quit!')
  end
end
