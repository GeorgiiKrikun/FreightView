vim.api.nvim_create_user_command("DdiveDebug", function()
	vim.cmd(":GdbStart gdb -q --args target/debug/ddive ddive-test-img:latest")
end, {})
