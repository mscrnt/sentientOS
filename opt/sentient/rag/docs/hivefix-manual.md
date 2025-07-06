# HiveFix Manual

## Overview

HiveFix is SentientOS's self-healing system that automatically detects and repairs system issues. It uses AI-powered analysis to identify problems and apply safe patches.

## How HiveFix Recovery Works

1. **Detection Phase**: HiveFix continuously monitors system logs and metrics for anomalies
2. **Analysis Phase**: When an issue is detected, the AI agent analyzes the root cause
3. **Patch Generation**: A fix is generated based on the analysis
4. **Sandbox Testing**: The patch is tested in an isolated environment
5. **Rollback Support**: All changes can be reverted if issues arise
6. **Application**: If tests pass, the patch is applied to the live system

## Key Features

- Automatic error detection
- AI-powered root cause analysis
- Safe sandbox testing before deployment
- Full audit logging
- Rollback capabilities
- Integration with sentient-shell

## Common Commands

- `hivefix enable` - Enable HiveFix monitoring
- `hivefix status` - Check current status
- `hivefix history` - View patch history
- `hivefix rollback <id>` - Rollback a specific patch

## Safety Mechanisms

HiveFix includes multiple safety layers:
- All patches are tested in sandbox first
- Dangerous operations require confirmation
- Full audit trail of all changes
- Automatic rollback on failure
- Integration with sentient-schema for validation