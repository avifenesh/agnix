--- agnix - Neovim plugin for linting agent configurations.
--- Main entry point. Call require('agnix').setup(opts) from your Neovim config.
-- @module agnix
local M = {}

local _initialized = false

--- Set up the agnix plugin.
--- Must be called once from your init.lua or plugin manager config.
--- @param opts table|nil Configuration options (see agnix.config for defaults)
function M.setup(opts)
  if _initialized then
    return
  end
  _initialized = true

  -- Load and merge configuration
  local config = require('agnix.config')
  config.setup(opts)

  -- Register .mdc as markdown (side effect of loading lsp module)
  local lsp_mod = require('agnix.lsp')

  -- Register user commands
  local commands = require('agnix.commands')
  commands.setup()

  -- Set up autocommands for automatic LSP attachment
  lsp_mod.setup_autocommands()

  -- If autostart is enabled and the current buffer matches, start immediately
  if config.current.autostart then
    local bufnr = vim.api.nvim_get_current_buf()
    local bufname = vim.api.nvim_buf_get_name(bufnr)
    if bufname ~= '' then
      local util = require('agnix.util')
      if util.is_agnix_file(bufname) then
        lsp_mod.start(bufnr)
      end
    end
  end
end

--- Start the LSP client for the current buffer.
function M.start()
  require('agnix.lsp').start()
end

--- Stop the running LSP client.
function M.stop()
  require('agnix.lsp').stop()
end

--- Restart the LSP client.
function M.restart()
  require('agnix.lsp').restart()
end

return M
