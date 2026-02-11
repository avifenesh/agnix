-- Root-level shim so that lazy.nvim users can install with just
-- "avifenesh/agnix" and require('agnix') resolves correctly.
--
-- lazy.nvim adds the repo root to runtimepath, which makes Neovim's
-- loader find this file for require('agnix').  We prepend
-- editors/neovim/lua to package.path so that all sub-module requires
-- (agnix.config, agnix.lsp, etc.) resolve to the real implementations.

local this_file = debug.getinfo(1, "S").source:sub(2)
local plugin_root = vim.fn.fnamemodify(this_file, ":h:h:h")
local neovim_lua = plugin_root .. "/editors/neovim/lua"

-- Prepend editors/neovim/lua to package.path (once)
if not string.find(package.path, neovim_lua, 1, true) then
  package.path = neovim_lua .. "/?.lua;" .. neovim_lua .. "/?/init.lua;" .. package.path
end

-- Also add editors/neovim to runtimepath for doc/, plugin/, etc.
local neovim_dir = plugin_root .. "/editors/neovim"
if vim.fn.isdirectory(neovim_dir) == 1 then
  vim.opt.runtimepath:prepend(neovim_dir)
end

-- Load and return the real module
return dofile(neovim_lua .. "/agnix/init.lua")
