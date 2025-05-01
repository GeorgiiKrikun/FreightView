vim.api.nvim_create_user_command("DdiveDebug", function()
	vim.cmd(":GdbStart gdb -q --args target/debug/ddive ddive-test-img:latest")
end, {})

vim.keymap.set("n", "<leader>bb", ":copen | AsyncRun cargo build<CR>", { noremap = true, silent = true })
vim.keymap.set("n", "<leader>bt", ":copen | AsyncRun cargo test<CR>", { noremap = true, silent = true })
