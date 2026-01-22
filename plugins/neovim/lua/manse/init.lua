local M = {}

M.config = {
  manse_cmd = "manse",
}

local function update_term_desc()
  local filename = vim.fn.expand("%:t")
  if filename == "" then
    filename = "[No Name]"
  end

  local desc = filename
  vim.fn.jobstart({ M.config.manse_cmd, "term-desc", desc }, { detach = true })
end

function M.setup(opts)
  opts = opts or {}
  M.config = vim.tbl_deep_extend("force", M.config, opts)

  local group = vim.api.nvim_create_augroup("Manse", { clear = true })

  vim.api.nvim_create_autocmd({ "BufEnter", "FocusGained" }, {
    group = group,
    callback = update_term_desc,
  })

  vim.api.nvim_create_autocmd("VimLeave", {
    group = group,
    callback = function()
      vim.fn.jobstart({ M.config.manse_cmd, "term-desc", "" }, { detach = true })
    end,
  })
end

return M
