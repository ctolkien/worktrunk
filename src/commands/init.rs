use worktrunk::shell;
use worktrunk::styling::println;

pub fn handle_init(shell: shell::Shell, command_name: String) -> Result<(), String> {
    let init = shell::ShellInit::new(shell, command_name);

    // Generate shell integration code (includes dynamic completion registration)
    let integration_output = init
        .generate()
        .map_err(|e| format!("Failed to generate shell code: {}", e))?;

    println!("{}", integration_output);

    Ok(())
}
