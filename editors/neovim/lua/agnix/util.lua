--- Utility functions for the agnix Neovim plugin.
-- @module agnix.util
local M = {}

local is_windows = vim.loop.os_uname().sysname:find('Windows') ~= nil

--- Separator used in file system paths.
M.path_sep = is_windows and '\\' or '/'

--- Normalize path separators to forward slashes for matching.
--- @param path string
--- @return string
local function normalize(path)
  return path:gsub('\\', '/')
end

--- Return the file name component of a path.
--- @param path string
--- @return string
local function basename(path)
  local p = normalize(path)
  return p:match('[^/]+$') or p
end

--- Return the parent directory name component of a path.
--- @param path string
--- @return string|nil
local function parent_dir_name(path)
  local p = normalize(path)
  local parent = p:match('(.+)/[^/]+$')
  if parent then
    return parent:match('[^/]+$')
  end
  return nil
end

--- Return the grandparent directory name component of a path.
--- @param path string
--- @return string|nil
local function grandparent_dir_name(path)
  local p = normalize(path)
  local gp = p:match('(.+)/[^/]+/[^/]+$')
  if gp then
    return gp:match('[^/]+$')
  end
  return nil
end

--- Search for the agnix-lsp binary.
--- Priority: opts.cmd > PATH > ~/.cargo/bin/agnix-lsp
--- @param opts table|nil Options with optional `cmd` field
--- @return string|nil path The resolved binary path, or nil if not found
function M.find_binary(opts)
  opts = opts or {}

  -- 1. Explicit path from user config
  if opts.cmd and type(opts.cmd) == 'string' then
    if vim.fn.executable(opts.cmd) == 1 then
      return opts.cmd
    end
  end

  -- 2. On PATH
  local name = is_windows and 'agnix-lsp.exe' or 'agnix-lsp'
  if vim.fn.executable(name) == 1 then
    return name
  end

  -- 3. Cargo install location
  local home = vim.loop.os_homedir()
  if home then
    local cargo_bin = home .. M.path_sep .. '.cargo' .. M.path_sep .. 'bin' .. M.path_sep .. name
    if vim.fn.executable(cargo_bin) == 1 then
      return cargo_bin
    end
  end

  return nil
end

--- Check whether a file path corresponds to a file type supported by agnix.
--- This mirrors the detect_file_type logic in agnix-core/src/lib.rs.
--- @param path string Absolute or relative file path
--- @return boolean
function M.is_agnix_file(path)
  if not path or path == '' then
    return false
  end

  -- Normalize once, then use the existing helper functions.
  local p = normalize(path)
  local name = basename(p)
  local parent = parent_dir_name(p)
  local grandparent = grandparent_dir_name(p)

  -- SKILL.md
  if name == 'SKILL.md' then
    return true
  end

  -- Memory files: CLAUDE.md, CLAUDE.local.md, AGENTS.md, AGENTS.local.md, AGENTS.override.md
  if
    name == 'CLAUDE.md'
    or name == 'CLAUDE.local.md'
    or name == 'AGENTS.md'
    or name == 'AGENTS.local.md'
    or name == 'AGENTS.override.md'
  then
    return true
  end

  -- Hook settings: .claude/settings.json, .claude/settings.local.json
  -- Note: Intentionally more restrictive than agnix-core (which matches any settings.json)
  -- to avoid attaching LSP to unrelated settings.json files (e.g. VS Code settings).
  if (name == 'settings.json' or name == 'settings.local.json') and parent == '.claude' then
    return true
  end

  -- Plugin manifest: plugin.json
  if name == 'plugin.json' then
    return true
  end

  -- MCP configuration files: *.mcp.json, mcp.json, mcp-*.json
  if name == 'mcp.json' then
    return true
  end
  if name:match('%.mcp%.json$') then
    return true
  end
  if name:match('^mcp%-') and name:match('%.json$') then
    return true
  end

  -- GitHub Copilot global instructions: .github/copilot-instructions.md
  if name == 'copilot-instructions.md' and parent == '.github' then
    return true
  end

  -- GitHub Copilot scoped instructions: .github/instructions/*.instructions.md
  if name:match('%.instructions%.md$') and parent == 'instructions' and grandparent == '.github' then
    return true
  end

  -- Cursor project rules: .cursor/rules/*.mdc
  if name:match('%.mdc$') and parent == 'rules' and grandparent == '.cursor' then
    return true
  end

  -- Legacy Cursor rules: .cursorrules
  if name == '.cursorrules' then
    return true
  end

  -- Agent files: .claude/agents/*.md or agents/*.md (parent or grandparent named "agents")
  if name:match('%.md$') then
    if parent == 'agents' or grandparent == 'agents' then
      return true
    end
  end

  return false
end

--- Find the project root directory by walking up from the given path.
--- Uses config.root_markers if available, else defaults.
--- @param path string Starting file path
--- @return string|nil root The project root directory, or nil
function M.get_root_dir(path)
  local cfg = package.loaded['agnix.config']
  local markers = (cfg and cfg.current and cfg.current.root_markers)
    or { '.git', '.agnix.toml', 'CLAUDE.md', 'AGENTS.md' }
  local found = vim.fs.find(markers, {
    path = path,
    upward = true,
    stop = vim.loop.os_homedir(),
    type = 'file',
  })

  -- vim.fs.find may return directory matches for .git
  if #found == 0 then
    found = vim.fs.find(markers, {
      path = path,
      upward = true,
      stop = vim.loop.os_homedir(),
      type = 'directory',
    })
  end

  if #found > 0 then
    return vim.fn.fnamemodify(found[1], ':h')
  end

  return nil
end

return M
