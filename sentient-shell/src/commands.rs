use crate::ai::AiClient;
use crate::package;
use anyhow::Result;

pub fn show_help() {
    println!("SentientShell Commands:");
    println!("  help       - Show this help message");
    println!("  status     - Show system status and connected AI models");
    println!("  ask <prompt> - Query AI model with a prompt");
    println!("  models     - List available AI models");
    println!("  image <prompt> - Generate image from prompt");
    println!("  pkg        - Package management commands");
    println!("  service    - Service management commands");
    println!("  ai         - AI router commands");
    println!("  rag        - Retrieval-augmented generation");
    println!("  tool       - Tool execution framework");
    println!("  llm        - LLM routing and model management");
    println!("  exit       - Exit the shell");
    println!();
    println!("Package Commands:");
    println!("  pkg list       - List available packages");
    println!("  pkg installed  - List installed packages");
    println!("  pkg install <name> - Install a package");
    println!("  pkg uninstall <name> - Uninstall a package");
    println!("  pkg search <query> - Search for packages");
    println!();
    println!("Examples:");
    println!("  ask What is the meaning of life?");
    println!("  image A beautiful sunset over mountains");
    println!("  pkg install calc");
    println!("  calc 2 + 2");
}

pub fn show_status(ai_client: &AiClient) -> Result<()> {
    println!("System Status:");
    println!("  Shell Version: {}", crate::SHELL_VERSION);
    println!("  Ollama Server: {}", ai_client.ollama_url());
    println!("  SD Server: {}", ai_client.sd_url());

    // Check connectivity
    print!("  Ollama Status: ");
    match ai_client.check_ollama_connection() {
        Ok(true) => println!("Connected ✓"),
        Ok(false) => println!("Not reachable ✗"),
        Err(e) => println!("Error: {}", e),
    }

    print!("  SD Status: ");
    match ai_client.check_sd_connection() {
        Ok(true) => println!("Connected ✓"),
        Ok(false) => println!("Not reachable ✗"),
        Err(e) => println!("Error: {}", e),
    }

    // Show preferred model
    match ai_client.get_preferred_model() {
        Ok(Some(model)) => println!("  Preferred Model: {}", model),
        Ok(None) => println!("  Preferred Model: None available"),
        Err(e) => println!("  Preferred Model: Error - {}", e),
    }

    Ok(())
}

pub fn ask_ai(ai_client: &mut AiClient, prompt: &str) -> Result<()> {
    println!("Thinking...");

    match ai_client.generate_text(prompt) {
        Ok(response) => {
            println!("\nResponse:");
            println!("{}", response);
            
            // Process response for tool calls
            if let Err(e) = crate::shell::tools::process_ai_response_for_tools(&response) {
                log::warn!("Failed to process tool calls: {}", e);
            }
        }
        Err(e) => {
            println!("Failed to get AI response: {}", e);

            // Try local inference if available
            #[cfg(feature = "local-inference")]
            {
                println!("Attempting local inference fallback...");
                // Note: In production, we'd pass the local_inference instance from ShellState
                println!("Local inference is available but not connected in this context");
            }

            // If all else fails, provide a demo response
            println!("\n[Demo Mode] Query: '{}'", prompt);
            println!("AI services are not available. In a production environment,");
            println!("this would connect to Ollama at http://192.168.69.197:11434");
        }
    }

    Ok(())
}

pub fn list_models(ai_client: &AiClient) -> Result<()> {
    println!("Available Models:");

    // List Ollama models
    println!("\nOllama Models:");
    match ai_client.list_ollama_models() {
        Ok(models) => {
            if models.is_empty() {
                println!("  No models found");
            } else {
                for model in models {
                    println!("  - {}", model);
                }
            }
        }
        Err(e) => {
            println!("  Error listing models: {}", e);
        }
    }

    // List SD models
    println!("\nStable Diffusion Models:");
    match ai_client.list_sd_models() {
        Ok(models) => {
            if models.is_empty() {
                println!("  No models found");
            } else {
                for model in models {
                    println!("  - {}", model);
                }
            }
        }
        Err(e) => {
            println!("  Error listing models: {}", e);
        }
    }

    Ok(())
}

pub fn generate_image(ai_client: &mut AiClient, prompt: &str) -> Result<()> {
    println!("Generating image...");

    match ai_client.generate_image(prompt) {
        Ok(image_info) => {
            println!("\nImage generated successfully!");
            println!("  Prompt: {}", prompt);
            println!("  Hash: {}", image_info.hash);
            println!("  Size: {} bytes", image_info.size);

            // In a real implementation, we might save the image or display a preview
            // For now, we just show the info
        }
        Err(e) => {
            println!("Failed to generate image: {}", e);

            // Provide demo response when SD is not available
            println!("\n[Demo Mode] Image prompt: '{}'", prompt);
            println!("SD WebUI is not available. In a production environment,");
            println!("this would connect to Stable Diffusion at http://192.168.69.197:7860");
            println!("Demo hash: a1b2c3d4e5f6789...");
        }
    }

    Ok(())
}

pub fn pkg_usage() {
    println!("Usage: pkg <subcommand>");
    println!("Subcommands:");
    println!("  list       - List available packages");
    println!("  installed  - List installed packages");
    println!("  install <name> - Install a package");
    println!("  uninstall <name> - Uninstall a package");
    println!("  search <query> - Search for packages");
}

pub fn handle_pkg_command(
    registry: &mut package::PackageRegistry,
    _ai_client: &mut AiClient,
    args: &[&str],
) -> Result<()> {
    if args.is_empty() {
        pkg_usage();
        return Ok(());
    }

    match args[0] {
        "list" => {
            let available = registry.list_available();
            if available.is_empty() {
                println!("No packages available for installation.");
            } else {
                println!("Available packages:");
                for pkg in available {
                    println!("  {} ({}) - {}", pkg.name, pkg.version, pkg.description);
                }
            }
        }
        "installed" => {
            let installed = registry.list_installed();
            if installed.is_empty() {
                println!("No packages installed.");
            } else {
                println!("Installed packages:");
                for pkg in installed {
                    println!("  {} ({}) - {}", pkg.name, pkg.version, pkg.description);
                }
            }
        }
        "install" => {
            if args.len() < 2 {
                println!("Usage: pkg install <name>");
                return Ok(());
            }
            match registry.install(args[1]) {
                Ok(msg) => println!("{}", msg),
                Err(e) => println!("Error: {}", e),
            }
        }
        "uninstall" => {
            if args.len() < 2 {
                println!("Usage: pkg uninstall <name>");
                return Ok(());
            }
            match registry.uninstall(args[1]) {
                Ok(msg) => println!("{}", msg),
                Err(e) => println!("Error: {}", e),
            }
        }
        "search" => {
            if args.len() < 2 {
                println!("Usage: pkg search <query>");
                return Ok(());
            }
            let query = args[1..].join(" ");
            let results = registry.search(&query);
            if results.is_empty() {
                println!("No packages found matching '{}'", query);
            } else {
                println!("Search results for '{}':", query);
                for pkg in results {
                    let status = if pkg.installed { "[installed]" } else { "" };
                    println!("  {} ({}) {} - {}", pkg.name, pkg.version, status, pkg.description);
                }
            }
        }
        _ => {
            println!("Unknown subcommand: {}", args[0]);
            pkg_usage();
        }
    }

    Ok(())
}

pub fn run_package(ai_client: &mut AiClient, package_name: &str, args: &[&str]) -> Result<()> {
    match package_name {
        "calc" => {
            if args.is_empty() {
                println!("Usage: calc <expression>");
                println!("Example: calc 2 + 2");
                return Ok(());
            }
            let expression = args.join(" ");
            match package::core::calc::run(&expression) {
                Ok(result) => println!("{}", result),
                Err(e) => println!("Error: {}", e),
            }
        }
        "neofetch" => {
            println!("{}", package::core::neofetch::run());
        }
        "joke" => {
            println!("Fetching a joke...");
            let runtime = tokio::runtime::Runtime::new()?;
            match runtime.block_on(package::core::joke::run(ai_client)) {
                Ok(joke) => println!("{}", joke),
                Err(e) => {
                    println!("Failed to get joke: {}", e);
                    println!("Why don't programmers like nature? It has too many bugs!");
                }
            }
        }
        "todo" => {
            match package::core::todo::run(args) {
                Ok(output) => println!("{}", output),
                Err(e) => println!("Error: {}", e),
            }
        }
        "ask" => {
            let runtime = tokio::runtime::Runtime::new()?;
            match runtime.block_on(package::core::ask::run(ai_client, args)) {
                Ok(response) => println!("{}", response),
                Err(e) => println!("Error: {}", e),
            }
        }
        "timer" => {
            match package::core::timer::run(args) {
                Ok(output) => println!("{}", output),
                Err(e) => println!("Error: {}", e),
            }
        }
        "scratch" => {
            match package::core::scratch::run(args) {
                Ok(output) => println!("{}", output),
                Err(e) => println!("Error: {}", e),
            }
        }
        "df" => {
            match package::core::df::run(args) {
                Ok(output) => println!("{}", output),
                Err(e) => println!("Error: {}", e),
            }
        }
        "top" => {
            match package::core::top::run(args) {
                Ok(output) => println!("{}", output),
                Err(e) => println!("Error: {}", e),
            }
        }
        "ps" => {
            match package::core::ps::run(args) {
                Ok(output) => println!("{}", output),
                Err(e) => println!("Error: {}", e),
            }
        }
        "hivefix" => {
            match package::core::hivefix::run(args) {
                Ok(output) => println!("{}", output),
                Err(e) => println!("Error: {}", e),
            }
        }
        _ => {
            println!("Package '{}' not found or not executable", package_name);
        }
    }
    Ok(())
}
