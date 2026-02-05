--- Configuration module for the agnix Neovim plugin.
--- Defaults mirror the VsCodeConfig schema in agnix-lsp/src/vscode_config.rs.
-- @module agnix.config
local M = {}

--- Default configuration values.
--- All LSP settings fields default to nil (use server/toml defaults).
M.defaults = {
  --- Path to the agnix-lsp binary. nil = auto-detect.
  cmd = nil,
  --- File types that may contain agnix-relevant files.
  filetypes = { 'markdown', 'json' },
  --- Markers used to locate the project root directory.
  root_markers = { '.git', '.agnix.toml', 'CLAUDE.md', 'AGENTS.md' },
  --- Start the LSP client automatically when a matching buffer is opened.
  autostart = true,
  --- Optional callback invoked when the LSP client attaches to a buffer.
  --- @type fun(client: table, bufnr: integer)|nil
  on_attach = nil,
  --- LSP settings sent to the server via workspace/didChangeConfiguration.
  --- Matches the VsCodeConfig struct in the agnix-lsp server.
  settings = {
    severity = nil,
    target = nil,
    tools = nil,
    rules = {
      skills = nil,
      hooks = nil,
      agents = nil,
      memory = nil,
      plugins = nil,
      xml = nil,
      mcp = nil,
      imports = nil,
      cross_platform = nil,
      agents_md = nil,
      copilot = nil,
      cursor = nil,
      prompt_engineering = nil,
      disabled_rules = nil,
    },
    versions = {
      claude_code = nil,
      codex = nil,
      cursor = nil,
      copilot = nil,
    },
    specs = {
      mcp_protocol = nil,
      agent_skills_spec = nil,
      agents_md_spec = nil,
    },
  },
  --- Minimum log level forwarded to the server.
  log_level = 'warn',
  --- Optional Telescope integration settings.
  telescope = {
    enable = true,
  },
}

--- The active configuration, set after setup() is called.
--- @type table|nil
M.current = nil

--- Merge user options into the default configuration.
--- @param opts table|nil User-provided options
function M.setup(opts)
  opts = opts or {}
  M.current = vim.tbl_deep_extend('force', vim.deepcopy(M.defaults), opts)
  M.validate(M.current)
end

--- Warn about clearly wrong configuration types.
--- @param opts table Configuration to validate
function M.validate(opts)
  if opts.cmd ~= nil and type(opts.cmd) ~= 'string' then
    vim.notify('[agnix] config.cmd must be a string or nil', vim.log.levels.WARN)
  end
  if opts.filetypes ~= nil and type(opts.filetypes) ~= 'table' then
    vim.notify('[agnix] config.filetypes must be a table', vim.log.levels.WARN)
  end
  if opts.root_markers ~= nil and type(opts.root_markers) ~= 'table' then
    vim.notify('[agnix] config.root_markers must be a table', vim.log.levels.WARN)
  end
  if opts.autostart ~= nil and type(opts.autostart) ~= 'boolean' then
    vim.notify('[agnix] config.autostart must be a boolean', vim.log.levels.WARN)
  end
  if opts.on_attach ~= nil and type(opts.on_attach) ~= 'function' then
    vim.notify('[agnix] config.on_attach must be a function or nil', vim.log.levels.WARN)
  end
  if opts.settings ~= nil and type(opts.settings) ~= 'table' then
    vim.notify('[agnix] config.settings must be a table', vim.log.levels.WARN)
  end
  if opts.log_level ~= nil and type(opts.log_level) ~= 'string' then
    vim.notify('[agnix] config.log_level must be a string', vim.log.levels.WARN)
  end
end

return M
