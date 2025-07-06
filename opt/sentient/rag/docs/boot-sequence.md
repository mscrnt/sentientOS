# SentientOS Boot Sequence

## Stage 1: UEFI Initialization
1. UEFI firmware loads
2. Secure boot verification
3. Load sentient-bootloader from ESP

## Stage 2: Bootloader
1. Initialize basic hardware
2. Load kernel from /boot/kernel.efi
3. Load boot AI model (phi-2) if configured
4. Pass control to kernel

## Stage 3: Kernel Initialization
1. Initialize memory management
2. Set up interrupt handlers
3. Initialize serial console (if enabled)
4. Load boot LLM subsystem
5. Initialize core subsystems

## Stage 4: Service Manager
1. sentd (service manager) starts
2. Load service manifests from /etc/sentient/services/
3. Start autostart services in dependency order
4. Initialize health monitoring

## Stage 5: Shell Initialization
1. sentient-shell starts
2. Initialize AI router
3. Connect to Ollama (if available)
4. Fall back to boot LLM if offline
5. Ready for user interaction

## Recovery Mode
If boot fails, recovery mode activates:
1. Boot with minimal services
2. Use phi-2 boot model for guidance
3. Enable safe mode restrictions
4. Provide recovery commands