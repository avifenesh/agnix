--- Minimal autoload entry point for the agnix Neovim plugin.
--- Does NOT call setup() automatically; the user must call require('agnix').setup().
--- Registers the :AgnixSetup convenience command.

if vim.g.loaded_agnix then
  return
end
vim.g.loaded_agnix = true

vim.api.nvim_create_user_command('AgnixSetup', function()
  require('agnix').setup({})
end, { desc = 'Initialize the agnix plugin with default configuration' })
