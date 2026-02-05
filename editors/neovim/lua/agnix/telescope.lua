--- Optional Telescope integration for agnix.
--- All functions are guarded so the plugin works without telescope.nvim.
-- @module agnix.telescope
local M = {}

local commands = require('agnix.commands')
local lsp = require('agnix.lsp')

local has_telescope, telescope = pcall(require, 'telescope')

--- Browse agnix rule categories with Telescope or vim.ui.select fallback.
function M.pick_rules()
  if not has_telescope then
    commands._show_rules_fallback()
    return
  end

  local pickers = require('telescope.pickers')
  local finders = require('telescope.finders')
  local conf = require('telescope.config').values
  local actions = require('telescope.actions')
  local action_state = require('telescope.actions.state')

  local items = {}
  for _, cat in ipairs(commands.rule_categories) do
    items[#items + 1] = {
      display = cat.prefix .. '* - ' .. cat.name,
      prefix = cat.prefix,
      name = cat.name,
    }
  end

  pickers
    .new({}, {
      prompt_title = 'agnix Rule Categories',
      finder = finders.new_table({
        results = items,
        entry_maker = function(entry)
          return {
            value = entry,
            display = entry.display,
            ordinal = entry.display,
          }
        end,
      }),
      sorter = conf.generic_sorter({}),
      attach_mappings = function(prompt_bufnr)
        actions.select_default:replace(function()
          actions.close(prompt_bufnr)
          local selection = action_state.get_selected_entry()
          if selection then
            local prefix = selection.value.prefix
            local url = 'https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md#'
              .. prefix:lower():gsub('%-$', '')
            vim.notify('[agnix] See: ' .. url, vim.log.levels.INFO)
          end
        end)
        return true
      end,
    })
    :find()
end

--- Show agnix diagnostics for the current buffer via Telescope.
function M.pick_diagnostics()
  if not has_telescope then
    -- Fallback: use built-in diagnostic list
    vim.diagnostic.setloclist()
    return
  end

  local bufnr = vim.api.nvim_get_current_buf()
  local diagnostics = vim.diagnostic.get(bufnr)

  -- Filter to agnix diagnostics only (source == "agnix")
  local agnix_diags = {}
  for _, d in ipairs(diagnostics) do
    if d.source == 'agnix' then
      agnix_diags[#agnix_diags + 1] = d
    end
  end

  if #agnix_diags == 0 then
    vim.notify('[agnix] No agnix diagnostics in current buffer', vim.log.levels.INFO)
    return
  end

  local pickers = require('telescope.pickers')
  local finders = require('telescope.finders')
  local conf = require('telescope.config').values
  local actions = require('telescope.actions')
  local action_state = require('telescope.actions.state')

  local severity_label = {
    [vim.diagnostic.severity.ERROR] = 'ERROR',
    [vim.diagnostic.severity.WARN] = 'WARN',
    [vim.diagnostic.severity.INFO] = 'INFO',
    [vim.diagnostic.severity.HINT] = 'HINT',
  }

  pickers
    .new({}, {
      prompt_title = 'agnix Diagnostics',
      finder = finders.new_table({
        results = agnix_diags,
        entry_maker = function(entry)
          local sev = severity_label[entry.severity] or 'UNKNOWN'
          local code = entry.code or ''
          local display = string.format(
            '%d:%d [%s] %s: %s',
            entry.lnum + 1,
            entry.col,
            sev,
            code,
            entry.message
          )
          return {
            value = entry,
            display = display,
            ordinal = display,
            lnum = entry.lnum + 1,
            col = entry.col,
            filename = vim.api.nvim_buf_get_name(bufnr),
          }
        end,
      }),
      sorter = conf.generic_sorter({}),
      attach_mappings = function(prompt_bufnr)
        actions.select_default:replace(function()
          actions.close(prompt_bufnr)
          local selection = action_state.get_selected_entry()
          if selection then
            vim.api.nvim_win_set_cursor(0, { selection.lnum, selection.col })
          end
        end)
        return true
      end,
    })
    :find()
end

--- Register as a Telescope extension (called by telescope.load_extension('agnix')).
if has_telescope then
  telescope.register_extension({
    exports = {
      rules = M.pick_rules,
      diagnostics = M.pick_diagnostics,
    },
  })
end

return M
