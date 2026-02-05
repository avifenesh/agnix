--- User commands for the agnix Neovim plugin.
-- @module agnix.commands
local M = {}

local lsp = require('agnix.lsp')
local util = require('agnix.util')

--- Register all :Agnix* user commands.
function M.setup()
  vim.api.nvim_create_user_command('AgnixStart', function()
    lsp.start()
  end, { desc = 'Start the agnix LSP server' })

  vim.api.nvim_create_user_command('AgnixStop', function()
    lsp.stop()
    vim.notify('[agnix] LSP server stopped', vim.log.levels.INFO)
  end, { desc = 'Stop the agnix LSP server' })

  vim.api.nvim_create_user_command('AgnixRestart', function()
    lsp.restart()
    vim.notify('[agnix] LSP server restarting...', vim.log.levels.INFO)
  end, { desc = 'Restart the agnix LSP server' })

  vim.api.nvim_create_user_command('AgnixInfo', function()
    local cfg = require('agnix.config').current or require('agnix.config').defaults
    local binary = util.find_binary({ cmd = cfg.cmd }) or '(not found)'
    local lines = { 'agnix info', '  binary: ' .. binary }

    if lsp.client_id then
      local client = vim.lsp.get_client_by_id(lsp.client_id)
      if client then
        lines[#lines + 1] = '  status: running (id=' .. lsp.client_id .. ')'
        if client.server_capabilities then
          lines[#lines + 1] = '  server: ' .. (client.name or 'agnix')
        end
        -- List attached buffers
        local attached = vim.lsp.get_buffers_by_client_id(lsp.client_id)
        if attached and #attached > 0 then
          lines[#lines + 1] = '  attached buffers:'
          for _, bufnr in ipairs(attached) do
            local name = vim.api.nvim_buf_get_name(bufnr)
            lines[#lines + 1] = '    [' .. bufnr .. '] ' .. (name ~= '' and name or '(unnamed)')
          end
        end
      else
        lines[#lines + 1] = '  status: stopped'
      end
    else
      lines[#lines + 1] = '  status: not started'
    end

    vim.notify(table.concat(lines, '\n'), vim.log.levels.INFO)
  end, { desc = 'Show agnix LSP server info' })

  vim.api.nvim_create_user_command('AgnixValidateFile', function()
    local bufnr = vim.api.nvim_get_current_buf()
    if not lsp.client_id then
      vim.notify('[agnix] LSP server is not running', vim.log.levels.WARN)
      return
    end
    local client = vim.lsp.get_client_by_id(lsp.client_id)
    if not client then
      vim.notify('[agnix] LSP client not available', vim.log.levels.WARN)
      return
    end
    local uri = vim.uri_from_bufnr(bufnr)
    client.notify('textDocument/didSave', {
      textDocument = { uri = uri },
    })
    vim.notify('[agnix] Validation triggered for ' .. vim.fn.expand('%:t'), vim.log.levels.INFO)
  end, { desc = 'Trigger agnix validation for the current file' })

  vim.api.nvim_create_user_command('AgnixShowRules', function()
    local ok, telescope = pcall(require, 'agnix.telescope')
    if ok and telescope.pick_rules then
      telescope.pick_rules()
    else
      M._show_rules_fallback()
    end
  end, { desc = 'Browse agnix rule categories' })

  vim.api.nvim_create_user_command('AgnixFixAll', function()
    M._apply_code_actions(false)
  end, { desc = 'Apply all agnix code action fixes for the current buffer' })

  vim.api.nvim_create_user_command('AgnixFixSafe', function()
    M._apply_code_actions(true)
  end, { desc = 'Apply only preferred (safe) agnix fixes for the current buffer' })

  vim.api.nvim_create_user_command('AgnixIgnoreRule', function(args)
    local rule_id = args.args
    if not rule_id or rule_id == '' then
      vim.notify('[agnix] Usage: AgnixIgnoreRule <rule_id>', vim.log.levels.WARN)
      return
    end
    if not (rule_id:match('^[A-Z]+%-%d+$') or rule_id:match('^[A-Z]+%-[A-Z]+%-%d+$')) then
      vim.notify('[agnix] Invalid rule ID format: ' .. rule_id, vim.log.levels.ERROR)
      return
    end
    M._ignore_rule(rule_id)
  end, { nargs = 1, desc = 'Add a rule to disabled_rules in .agnix.toml' })

  vim.api.nvim_create_user_command('AgnixShowRuleDoc', function(args)
    local rule_id = args.args
    if not rule_id or rule_id == '' then
      vim.notify('[agnix] Usage: AgnixShowRuleDoc <rule_id>', vim.log.levels.WARN)
      return
    end
    if not (rule_id:match('^[A-Z]+%-%d+$') or rule_id:match('^[A-Z]+%-[A-Z]+%-%d+$')) then
      vim.notify('[agnix] Invalid rule ID format: ' .. rule_id, vim.log.levels.ERROR)
      return
    end
    local url = 'https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md#'
      .. rule_id:lower()
    if vim.ui.open then
      vim.ui.open(url)
    else
      local sysname = vim.loop.os_uname().sysname
      local cmd
      if sysname == 'Darwin' then
        cmd = { 'open', url }
      elseif sysname:find('Windows') then
        cmd = { 'cmd', '/c', 'start', '""', url }
      else
        cmd = { 'xdg-open', url }
      end
      vim.fn.jobstart(cmd, { detach = true })
    end
  end, { nargs = 1, desc = 'Open documentation for an agnix rule in the browser' })
end

--- Rule categories used by the rules picker and fallback UI.
M.rule_categories = {
  { prefix = 'AS-', name = 'Agent Skills' },
  { prefix = 'CC-SK-', name = 'Claude Code Skills' },
  { prefix = 'CC-HK-', name = 'Claude Code Hooks' },
  { prefix = 'CC-AG-', name = 'Claude Code Agents' },
  { prefix = 'CC-PL-', name = 'Claude Code Plugins' },
  { prefix = 'PE-', name = 'Prompt Engineering' },
  { prefix = 'MCP-', name = 'MCP' },
  { prefix = 'AGM-', name = 'Memory Files' },
  { prefix = 'COP-', name = 'GitHub Copilot' },
  { prefix = 'CUR-', name = 'Cursor' },
  { prefix = 'XML-', name = 'XML' },
  { prefix = 'XP-', name = 'Cross-Platform' },
}

--- Fallback rule browser when Telescope is not available.
function M._show_rules_fallback()
  local items = {}
  for _, cat in ipairs(M.rule_categories) do
    items[#items + 1] = cat.prefix .. '* - ' .. cat.name
  end
  vim.ui.select(items, { prompt = 'agnix rule categories:' }, function(choice)
    if choice then
      local prefix = choice:match('^([^*]+)%*')
      if prefix then
        local url = 'https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md#'
          .. prefix:lower():gsub('%-$', '')
        vim.notify('[agnix] See: ' .. url, vim.log.levels.INFO)
      end
    end
  end)
end

--- Request code actions from the LSP and apply them.
--- @param preferred_only boolean When true, only apply actions marked isPreferred
function M._apply_code_actions(preferred_only)
  local bufnr = vim.api.nvim_get_current_buf()
  if not lsp.client_id then
    vim.notify('[agnix] LSP server is not running', vim.log.levels.WARN)
    return
  end

  local params = vim.lsp.util.make_range_params()
  params.context = {
    diagnostics = vim.tbl_filter(
      function(d) return d.source == 'agnix' end,
      vim.diagnostic.get(bufnr)
    ),
    only = { 'quickfix' },
  }

  vim.lsp.buf_request(bufnr, 'textDocument/codeAction', params, function(err, result)
    if err then
      vim.notify('[agnix] Code action request failed: ' .. tostring(err), vim.log.levels.ERROR)
      return
    end
    if not result or #result == 0 then
      vim.notify('[agnix] No code actions available', vim.log.levels.INFO)
      return
    end

    local applied = 0
    for _, action in ipairs(result) do
      if not preferred_only or action.isPreferred then
        if action.edit then
          vim.lsp.util.apply_workspace_edit(action.edit, 'utf-8')
          applied = applied + 1
        end
      end
    end

    local label = preferred_only and 'safe fixes' or 'fixes'
    vim.notify(
      '[agnix] Applied ' .. applied .. ' ' .. label,
      applied > 0 and vim.log.levels.INFO or vim.log.levels.WARN
    )
  end)
end

--- Add a rule to the disabled_rules list in .agnix.toml.
--- Creates the file if it does not exist.
--- @param rule_id string Rule identifier (e.g. "AS-004")
function M._ignore_rule(rule_id)
  local root = util.get_root_dir(vim.fn.expand('%:p')) or vim.fn.getcwd()
  local toml_path = root .. util.path_sep .. '.agnix.toml'

  local content = ''
  local f = io.open(toml_path, 'r')
  if f then
    content = f:read('*a')
    f:close()
  end

  -- Check if the rule is already disabled (plain text search, not pattern)
  if content:find('"' .. rule_id .. '"', 1, true) or content:find("'" .. rule_id .. "'", 1, true) then
    vim.notify('[agnix] Rule ' .. rule_id .. ' is already disabled', vim.log.levels.INFO)
    return
  end

  -- Check if there is an uncommented disabled_rules line
  local has_disabled_rules = false
  for line in content:gmatch('[^\n]+') do
    local stripped = line:match('^%s*(.*)')
    if stripped and not stripped:match('^#') and stripped:find('disabled_rules', 1, true) then
      has_disabled_rules = true
      break
    end
  end

  local ok, write_err = pcall(function()
    if has_disabled_rules then
      local new_content, subs = content:gsub('(disabled_rules%s*=%s*%[)', '%1"' .. rule_id .. '", ')
      if subs == 0 then
        vim.notify('[agnix] Could not locate disabled_rules array in .agnix.toml', vim.log.levels.WARN)
        return
      end
      local out = io.open(toml_path, 'w')
      if not out then error('Cannot write to ' .. toml_path) end
      out:write(new_content)
      out:close()
    else
      local out = io.open(toml_path, 'a')
      if not out then error('Cannot write to ' .. toml_path) end
      if not content:find('[rules]', 1, true) then
        out:write('\n[rules]\n')
      end
      out:write('disabled_rules = ["' .. rule_id .. '"]\n')
      out:close()
    end
  end)

  if not ok then
    vim.notify('[agnix] Failed to update .agnix.toml: ' .. tostring(write_err), vim.log.levels.ERROR)
    return
  end

  vim.notify('[agnix] Disabled rule ' .. rule_id .. ' in .agnix.toml', vim.log.levels.INFO)
end

return M
