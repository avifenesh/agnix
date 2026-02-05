--- Health check for :checkhealth agnix
--- Compatible with Neovim 0.9+ (handles both vim.health and vim.health.report_* APIs).
-- @module agnix.health
local M = {}

local util = require('agnix.util')
local config = require('agnix.config')
local lsp = require('agnix.lsp')

-- Compatibility shim: Neovim 0.10+ uses vim.health.start/ok/warn/error,
-- while 0.9.x uses vim.health.report_start/report_ok/report_warn/report_error.
local health = {}
if vim.health and vim.health.start then
  health.start = vim.health.start
  health.ok = vim.health.ok
  health.warn = vim.health.warn
  health.error = vim.health.error
  health.info = vim.health.info
elseif vim.health then
  health.start = vim.health.report_start
  health.ok = vim.health.report_ok
  health.warn = vim.health.report_warn
  health.error = vim.health.report_error
  health.info = vim.health.report_info or vim.health.report_ok
end

function M.check()
  health.start('agnix')

  -- 1. Check Neovim version
  local version = vim.version()
  if version.major == 0 and version.minor < 9 then
    health.error('Neovim >= 0.9 is required, found ' .. tostring(version))
  else
    health.ok('Neovim version: ' .. tostring(version))
  end

  -- 2. Check agnix-lsp binary
  local cfg = config.current or config.defaults
  local binary = util.find_binary({ cmd = cfg.cmd })
  if binary then
    health.ok('agnix-lsp binary found: ' .. binary)
    -- Try to get version
    local result = vim.fn.system({ binary, '--version' })
    if vim.v.shell_error == 0 and result and result ~= '' then
      health.ok('agnix-lsp version: ' .. vim.trim(result))
    end
  else
    health.error(
      'agnix-lsp binary not found',
      { 'Install with: cargo install agnix-lsp', 'Or set cmd in require("agnix").setup({ cmd = "/path/to/agnix-lsp" })' }
    )
  end

  -- 3. Check LSP client status
  if lsp.client_id then
    local client = vim.lsp.get_client_by_id(lsp.client_id)
    if client then
      health.ok('LSP client running (id=' .. lsp.client_id .. ')')
    else
      health.warn('LSP client id set but client not found (may have stopped)')
    end
  else
    health.info('LSP client not started (will start when an agnix file is opened)')
  end

  -- 4. Check for .agnix.toml
  local root = util.get_root_dir(vim.fn.getcwd())
  if root then
    local toml_path = root .. util.path_sep .. '.agnix.toml'
    if vim.fn.filereadable(toml_path) == 1 then
      health.ok('.agnix.toml found at: ' .. toml_path)
    else
      health.info('No .agnix.toml found (using defaults)')
    end
  else
    health.info('No project root detected')
  end

  -- 5. Check optional dependencies
  local has_telescope = pcall(require, 'telescope')
  if has_telescope then
    health.ok('telescope.nvim: available')
  else
    health.info('telescope.nvim: not installed (optional, for rule browsing)')
  end

  local has_lspconfig = pcall(require, 'lspconfig')
  if has_lspconfig then
    health.ok('nvim-lspconfig: available (not required, agnix has built-in LSP management)')
  else
    health.info('nvim-lspconfig: not installed (not required)')
  end
end

return M
