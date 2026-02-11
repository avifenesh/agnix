-- Root-level shim: adds editors/neovim to runtimepath so that
-- require('agnix') resolves to editors/neovim/lua/agnix/init.lua.
-- This allows lazy.nvim users to install with just "avifenesh/agnix"
-- without needing subdir = "editors/neovim".

if vim.g.loaded_agnix then
  return
end

local plugin_root = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h")
local neovim_dir = plugin_root .. "/editors/neovim"

if vim.fn.isdirectory(neovim_dir) == 1 then
  vim.opt.runtimepath:prepend(neovim_dir)
end

-- The actual plugin/agnix.lua in editors/neovim/ will handle the rest
-- (setting vim.g.loaded_agnix and registering commands).
dofile(neovim_dir .. "/plugin/agnix.lua")
