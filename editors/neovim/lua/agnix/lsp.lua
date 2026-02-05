--- LSP client management for agnix.
--- Uses the built-in vim.lsp.start() API (Neovim >= 0.9).
-- @module agnix.lsp
local M = {}

local config = require('agnix.config')
local util = require('agnix.util')

--- The active LSP client id, or nil.
--- @type integer|nil
M.client_id = nil

--- Register .mdc as a markdown filetype so LSP attaches to Cursor rule files.
--- Called from setup_autocommands() to avoid side effects on module load.
local function register_mdc_filetype()
  vim.filetype.add({ extension = { mdc = 'markdown' } })
end

--- Build the settings table sent to the LSP server.
--- Filters out nil values so the server only sees explicitly set options.
--- @return table settings JSON-compatible settings table
function M.build_lsp_settings()
  local cfg = config.current or config.defaults
  local s = cfg.settings or {}

  --- Recursively remove nil/vim.NIL entries from a table for JSON encoding.
  local function compact(tbl)
    if type(tbl) ~= 'table' then
      return tbl
    end
    local out = {}
    for k, v in pairs(tbl) do
      if v ~= nil and v ~= vim.NIL then
        local compacted = compact(v)
        -- Keep non-empty tables and non-table values
        if type(compacted) ~= 'table' or next(compacted) ~= nil then
          out[k] = compacted
        end
      end
    end
    return out
  end

  return compact(s)
end

--- Start the agnix LSP client for the given buffer.
--- Reuses an existing client if one is already running with the same root.
--- @param bufnr integer|nil Buffer number (defaults to current buffer)
--- @return integer|nil client_id The LSP client id, or nil on failure
function M.start(bufnr)
  bufnr = bufnr or vim.api.nvim_get_current_buf()
  local cfg = config.current or config.defaults

  local binary = util.find_binary({ cmd = cfg.cmd })
  if not binary then
    vim.notify(
      '[agnix] agnix-lsp binary not found. Install with: cargo install agnix-lsp',
      vim.log.levels.ERROR
    )
    return nil
  end

  local bufname = vim.api.nvim_buf_get_name(bufnr)
  local root = util.get_root_dir(bufname) or vim.fn.getcwd()

  local client_id = vim.lsp.start({
    name = 'agnix',
    cmd = { binary },
    root_dir = root,
    capabilities = vim.lsp.protocol.make_client_capabilities(),
    on_init = function(client)
      local settings = M.build_lsp_settings()
      if next(settings) ~= nil then
        client.notify('workspace/didChangeConfiguration', { settings = settings })
      end
    end,
    on_attach = function(client, attached_bufnr)
      if cfg.on_attach then
        cfg.on_attach(client, attached_bufnr)
      end
    end,
  })

  if client_id then
    M.client_id = client_id
  end
  return client_id
end

--- Stop the running agnix LSP client.
function M.stop()
  if M.client_id then
    local client = vim.lsp.get_client_by_id(M.client_id)
    if client then
      client.stop()
    end
    M.client_id = nil
  end
end

--- Restart the agnix LSP client.
--- Stops the current client, then starts a fresh one for the current buffer.
function M.restart()
  M.stop()
  -- Small delay to allow the old process to exit cleanly.
  vim.defer_fn(function()
    M.start()
  end, 200)
end

--- Set up autocommands that attach the LSP client to matching buffers.
--- Creates the 'agnix' augroup.
function M.setup_autocommands()
  local cfg = config.current or config.defaults
  local group = vim.api.nvim_create_augroup('agnix', { clear = true })

  register_mdc_filetype()

  -- Use FileType event with specific filetypes to avoid running is_agnix_file on every buffer.
  vim.api.nvim_create_autocmd('FileType', {
    group = group,
    pattern = cfg.filetypes or { 'markdown', 'json' },
    callback = function(ev)
      if not cfg.autostart then
        return
      end
      local bufname = vim.api.nvim_buf_get_name(ev.buf)
      if bufname == '' then
        return
      end
      if not util.is_agnix_file(bufname) then
        return
      end
      M.start(ev.buf)
    end,
    desc = 'Attach agnix LSP to supported files',
  })

  -- Also handle BufReadPost for files that may already have their filetype set.
  -- Uses is_agnix_file() as the single source of truth for supported file patterns.
  vim.api.nvim_create_autocmd('BufReadPost', {
    group = group,
    pattern = '*',
    callback = function(ev)
      if not cfg.autostart then
        return
      end
      local bufname = vim.api.nvim_buf_get_name(ev.buf)
      if bufname == '' then
        return
      end
      if not util.is_agnix_file(bufname) then
        return
      end
      M.start(ev.buf)
    end,
    desc = 'Attach agnix LSP to agnix-supported files',
  })
end

return M
