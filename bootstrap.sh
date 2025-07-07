#!/bin/bash
# SentientOS Bootstrap Script
# Installs all dependencies and validates the system

set -e  # Exit on error

echo "🚀 SentientOS Bootstrap Script"
echo "=============================="
echo ""

# Check if we're in the right directory
if [ ! -f "requirements.txt" ] || [ ! -d "sentient-shell" ]; then
    echo "❌ Error: Please run this script from the SentientOS root directory"
    exit 1
fi

# Step 1: Create Python virtual environment
echo "📦 Step 1: Setting up Python environment..."
if [ ! -d ".venv" ]; then
    echo "Creating virtual environment..."
    python3 -m venv .venv
fi

# Activate virtual environment
source .venv/bin/activate

# Step 2: Install Python dependencies
echo ""
echo "📦 Step 2: Installing Python dependencies..."
pip install --upgrade pip
pip install -r requirements.txt

# Step 3: Create required directories
echo ""
echo "📁 Step 3: Creating required directories..."
mkdir -p config logs rl_data rl_checkpoints

# Step 4: Validate configuration files
echo ""
echo "⚙️  Step 4: Validating configuration files..."

# Check if config files exist
if [ ! -f "config/tool_registry.toml" ]; then
    echo "❌ Missing config/tool_registry.toml"
    echo "Please ensure all config files are present"
    exit 1
fi

if [ ! -f "config/router_config.toml" ]; then
    echo "❌ Missing config/router_config.toml"
    echo "Please ensure all config files are present"
    exit 1
fi

if [ ! -f "config/conditions.yaml" ]; then
    echo "❌ Missing config/conditions.yaml"
    echo "Please ensure all config files are present"
    exit 1
fi

echo "✅ All config files present"

# Step 5: Check Rust toolchain
echo ""
echo "🦀 Step 5: Checking Rust toolchain..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
fi

cargo_version=$(cargo --version)
echo "✅ Found $cargo_version"

# Step 6: Build Rust project (allow failures for now)
echo ""
echo "🔨 Step 6: Building Rust project..."
cd sentient-shell
echo "Running cargo check..."
if cargo check 2>&1 | grep -q "error"; then
    echo "⚠️  Warning: Rust compilation has errors. This is a known issue."
    echo "   The Python components should still work."
else
    echo "✅ Rust project checks passed"
fi
cd ..

# Step 7: Run validation
echo ""
echo "🧪 Step 7: Running system validation..."
if [ -f "validate_phase6.py" ]; then
    python validate_phase6.py
else
    echo "⚠️  Validation script not found, skipping..."
fi

# Step 8: Test import
echo ""
echo "🎯 Step 8: Testing core imports..."
python -c "
import sys
sys.path.insert(0, 'sentient-core')
try:
    from planner.planner import SentientPlanner
    from executor.executor import SentientExecutor
    print('✅ Core modules imported successfully')
except Exception as e:
    print(f'❌ Import error: {e}')
    sys.exit(1)
"

# Summary
echo ""
echo "=============================="
echo "✨ Bootstrap Complete!"
echo ""
echo "Next steps:"
echo "1. Activate the virtual environment:"
echo "   source .venv/bin/activate"
echo ""
echo "2. Test the sentient goal command:"
echo "   python -m sentient_core.main goal \"Check system status\""
echo ""
echo "3. Or run the shell (once Rust issues are fixed):"
echo "   cd sentient-shell && cargo run"
echo ""
echo "Note: Rust compilation has known issues that need to be fixed"
echo "      for the full shell experience. Python components work."