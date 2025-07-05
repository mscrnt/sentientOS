#!/bin/bash

# Boot validation script for SentientOS
# Can be used both locally and in CI environments

set -e

SERIAL_LOG="${1:-serial.log}"
VERBOSE="${2:-false}"

# ANSI color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "📊 Validating SentientOS boot sequence..."
echo "📄 Serial log: $SERIAL_LOG"
echo ""

if [ ! -f "$SERIAL_LOG" ]; then
    echo -e "${RED}❌ Serial log file not found: $SERIAL_LOG${NC}"
    exit 1
fi

# Define boot phases with their expected markers
declare -A BOOT_PHASES=(
    ["01_serial_init"]="Serial initialized"
    ["02_bootloader_start"]="SentientOS UEFI Bootloader"
    ["03_boot_phase"]="\\[BOOT\\]"
    ["04_hardware_detect"]="Hardware detection complete"
    ["05_ai_phase"]="\\[AI\\]"
    ["06_model_load"]="Model loaded successfully"
    ["07_load_phase"]="\\[LOAD\\]"
    ["08_kernel_found"]="kernel.efi"
    ["09_exec_phase"]="\\[EXEC\\]"
    ["10_kernel_start"]="SentientOS Kernel"
    ["11_bootinfo_parse"]="BootInfo parsed successfully"
    ["12_memory_init"]="Memory management initialized"
    ["13_ai_init"]="AI subsystem initialized"
    ["14_kernel_runtime"]="Entering AI-driven kernel runtime"
)

# Additional warning patterns to check
declare -A WARNING_PATTERNS=(
    ["panic"]="PANIC"
    ["error"]="ERROR"
    ["failed"]="Failed"
    ["missing"]="Missing"
    ["not_found"]="not found"
)

# Track results
PASSED=0
FAILED=0
WARNINGS=0

echo "🔍 Checking boot phases..."
echo "=========================="

# Check each boot phase
for phase in $(echo "${!BOOT_PHASES[@]}" | tr ' ' '\n' | sort); do
    pattern="${BOOT_PHASES[$phase]}"
    phase_name=$(echo "$phase" | cut -d'_' -f2-)
    
    if grep -q "$pattern" "$SERIAL_LOG"; then
        echo -e "${GREEN}✅ $phase_name: Found${NC}"
        ((PASSED++))
        
        if [ "$VERBOSE" = "true" ]; then
            echo "   └─ $(grep "$pattern" "$SERIAL_LOG" | head -1)"
        fi
    else
        echo -e "${RED}❌ $phase_name: Missing${NC}"
        ((FAILED++))
    fi
done

echo ""
echo "🔍 Checking for errors..."
echo "========================"

# Check for warning patterns
for warning in "${!WARNING_PATTERNS[@]}"; do
    pattern="${WARNING_PATTERNS[$warning]}"
    count=$(grep -c "$pattern" "$SERIAL_LOG" || true)
    
    if [ "$count" -gt 0 ]; then
        echo -e "${YELLOW}⚠️  Found $count occurrence(s) of: $pattern${NC}"
        ((WARNINGS+=count))
        
        if [ "$VERBOSE" = "true" ]; then
            grep "$pattern" "$SERIAL_LOG" | head -5 | while read -r line; do
                echo "   └─ $line"
            done
        fi
    fi
done

if [ "$WARNINGS" -eq 0 ]; then
    echo -e "${GREEN}✅ No error patterns detected${NC}"
fi

echo ""
echo "📈 Boot Validation Summary"
echo "========================="
echo -e "Passed phases: ${GREEN}$PASSED${NC}"
echo -e "Failed phases: ${RED}$FAILED${NC}"
echo -e "Warnings:      ${YELLOW}$WARNINGS${NC}"

# Extract timing information if available
echo ""
echo "⏱️  Boot Timing Analysis"
echo "======================="

# Try to extract boot time from logs
BOOT_START=$(grep -E "Serial initialized|UEFI Bootloader" "$SERIAL_LOG" | head -1 | cut -d' ' -f1 || echo "N/A")
KERNEL_START=$(grep "SentientOS Kernel" "$SERIAL_LOG" | head -1 | cut -d' ' -f1 || echo "N/A")
RUNTIME_START=$(grep "kernel runtime" "$SERIAL_LOG" | head -1 | cut -d' ' -f1 || echo "N/A")

echo "Boot start:    $BOOT_START"
echo "Kernel start:  $KERNEL_START"
echo "Runtime start: $RUNTIME_START"

# Final verdict
echo ""
if [ "$FAILED" -eq 0 ]; then
    echo -e "${GREEN}🎉 Boot validation PASSED!${NC}"
    echo "SentientOS booted successfully with all expected phases."
    exit 0
else
    echo -e "${RED}💥 Boot validation FAILED!${NC}"
    echo "$FAILED critical boot phase(s) missing."
    
    # Provide debugging hints
    echo ""
    echo "🔧 Debugging hints:"
    echo "  - Check if kernel.efi exists in ESP directory"
    echo "  - Verify bootloader can find and load the kernel"
    echo "  - Check for memory allocation issues"
    echo "  - Review model loading process"
    
    exit 1
fi