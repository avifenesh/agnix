--- Minimal autoload entry point for the agnix Neovim plugin.
--- Does NOT call setup() automatically; the user must call require('agnix').setup().
--- Registers the :AgnixSetup convenience command.

if vim.g.loaded_agnix then
  return
end
vim.g.loaded_agnix = true

vim.api.nvim_create_user_command('AgnixSetup', function(args)
  local opts = {}
  -- Allow passing a Lua table as a string argument for simple cases
  if args.args and args.args ~= '' then
    local ok, parsed = pcall(vim.fn.eval, args.args)
    if ok and type(parsed) == 'table' then
      opts = parsed
    end
  end
  require('agnix').setup(opts)
end, { nargs = '?', desc = 'Initialize the agnix plugin with optional configuration' })
