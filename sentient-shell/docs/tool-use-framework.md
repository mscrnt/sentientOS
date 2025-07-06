# SentientOS Tool Use Framework

## Overview

The Tool Use Framework enables LLMs to execute system functions through a secure, schema-validated interface. It supports multiple execution modes, automatic discovery from AI responses, and comprehensive safety controls.

## Architecture

### Components

1. **Tool Registry** (`tools/registry.rs`)
   - Central repository for all available tools
   - Metadata management (privileges, timeouts, schemas)
   - Category-based organization
   - Default system tools pre-registered

2. **Tool Executor** (`tools/exec.rs`)
   - Secure command execution with privilege checking
   - Multiple execution modes (Safe, Privileged, Background, Sandboxed)
   - Timeout management
   - Confirmation prompts for dangerous operations

3. **LLM Function Parser** (`llm/functions.rs`)
   - Parses multiple function call formats
   - Command format: `!@ tool_name {args}`
   - JSON format: `{"tool": "disk_info", "args": {}}`
   - Natural language: "Please call disk_info"
   - Structured: `<function>disk_info()</function>`

4. **Shell Integration** (`shell/tools.rs`)
   - CLI commands for tool management
   - Automatic discovery from AI responses
   - Result display and error handling

## Command Prefixes

- `!@` - Validated execution (default)
- `!#` - Dangerous operation (requires confirmation)
- `!$` - System/privileged execution
- `!&` - Background execution
- `!~` - Sandboxed execution

## Usage

### Shell Commands

```bash
# List all available tools
tool list

# Get detailed information about a tool
tool info disk_info

# Execute a tool directly
tool call disk_info
tool call kill_process pid=1234

# Search for tools
tool search network

# Show help
tool help
```

### AI Integration

When an AI response contains tool calls, they are automatically detected and executed:

```bash
sentient> ask Show me disk usage
Response:
I'll check the disk usage for you.

!@ call disk_info

🔧 Detected 1 tool call(s) in response

📋 Tool: disk_info
   ✅ Success

Output:
  Filesystem      Size  Used Avail Use% Mounted on
  /dev/sda1       100G   45G   50G  48% /
  /dev/sda2       500G  200G  275G  43% /home
```

### Defining Custom Tools

```rust
use sentient_shell::tools::registry::{Tool, get_tool_registry};
use sentient_shell::schema::schema::SchemaBuilder;

// Define a custom tool
let custom_tool = Tool {
    id: "my_tool".to_string(),
    name: "My Custom Tool".to_string(),
    description: "Does something useful".to_string(),
    command: "/usr/bin/mytool".to_string(),
    requires_privilege: false,
    requires_confirmation: true,
    schema: Some(
        SchemaBuilder::new("MyToolArgs")
            .string_field("input")
                .min_length(1)
                .and()
            .boolean_field("verbose")
                .default_value(json!(false))
                .and()
            .build()
    ),
    tags: vec!["custom".to_string()],
    examples: vec![
        r#"!@ call my_tool {"input": "data"}"#.to_string(),
    ],
    timeout: 30,
};

// Register the tool
get_tool_registry().register(custom_tool)?;
```

## Security Features

### Privilege Management
- Tools requiring elevated privileges must be marked as such
- Executor validates privilege requirements before execution
- User confirmation required for dangerous operations

### Sandboxing
- Sandboxed execution mode using firejail (if available)
- Network isolation for sandboxed tools
- Restricted filesystem access

### Validation
- Schema validation for all tool arguments
- Command injection protection via shell escaping
- Timeout enforcement to prevent hanging

### Audit Trail
- All tool executions are logged
- Execution results include timing information
- Failed executions capture error details

## Default Tools

### System Information
- `disk_info` - Display disk usage
- `memory_info` - Show memory statistics
- `network_status` - Network interface information

### Process Management
- `process_list` - List running processes
- `kill_process` - Terminate a process (privileged)

### Service Management
- `service_status` - Check service status

### Recovery Tools
- `safe_mode` - Enter recovery mode (privileged)
- `reset_network` - Reset network configuration (privileged)

### HiveFix Integration
- `hivefix_status` - Check self-healing status
- `hivefix_analyze` - Analyze system logs

## LLM Integration Guide

### System Prompt

Include available tools in the LLM system prompt:

```
You have access to the following system tools:

## Disk Information
ID: disk_info
Description: Get disk space usage information
Examples:
  !@ call disk_info

## Kill Process
ID: kill_process
Description: Terminate a process by PID
⚡ Requires elevated privileges
Arguments: {"pid": integer, "force": boolean}
Examples:
  !$ call kill_process {"pid": 1234}
  !$ call kill_process {"pid": 5678, "force": true}

To call a tool, use one of these formats:
- Command: !@ tool_id {"arg": "value"}
- JSON: {"tool": "tool_id", "args": {"arg": "value"}}
- Natural: Please call tool_id with arg value
```

### Response Processing

The framework automatically:
1. Detects tool calls in LLM responses
2. Validates arguments against schemas
3. Requests confirmation if needed
4. Executes tools with appropriate permissions
5. Displays results to the user

## Testing

Run the test script to verify functionality:

```bash
./scripts/test-tools.sh
```

This tests:
- Tool listing and discovery
- Tool information display
- Direct tool execution
- Tool search functionality
- AI response processing

## Future Enhancements

1. **Tool Chaining** - Allow tools to call other tools
2. **Async Execution** - Non-blocking tool execution
3. **Result Caching** - Cache frequently called tool results
4. **Tool Versioning** - Support multiple versions of tools
5. **Remote Tools** - Execute tools on remote systems
6. **Tool Marketplace** - Download and install community tools