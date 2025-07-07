#!/bin/bash
# Install SentientOS Continuous Learning System

set -e

echo "🔄 Installing SentientOS Continuous Learning System"
echo "=================================================="

# Check prerequisites
if [ ! -d ".venv" ]; then
    echo "❌ Virtual environment not found. Please run bootstrap.sh first"
    exit 1
fi

# Activate virtual environment
source .venv/bin/activate

# Install additional dependencies
echo "📦 Installing watchdog for file monitoring..."
pip install watchdog

# Create required directories
echo "📁 Creating directories..."
mkdir -p logs policies rl_checkpoints experiments

# Make scripts executable
echo "🔧 Setting up scripts..."
chmod +x rl_agent/trace_monitor.py
chmod +x rl_agent/continuous_learning.py
chmod +x rl_agent/dashboard.py
chmod +x rl_agent/orchestrator.py
chmod +x scripts/sentient-rl

# Add to PATH
echo "🛣️  Adding sentient-rl to PATH..."
if [ -d "$HOME/.local/bin" ]; then
    ln -sf "$PWD/scripts/sentient-rl" "$HOME/.local/bin/sentient-rl" 2>/dev/null || true
fi

# Test import
echo "🧪 Testing imports..."
python -c "
from rl_agent.trace_monitor import TraceMonitor
from rl_agent.continuous_learning import ContinuousLearner
print('✅ All imports successful')
"

echo ""
echo "✅ Continuous Learning System Installed!"
echo ""
echo "Available commands:"
echo "  sentient-rl status         - Check system status"
echo "  sentient-rl start          - Start continuous learning"
echo "  sentient-rl stop           - Stop continuous learning"
echo "  sentient-rl dashboard      - Launch monitoring dashboard"
echo "  sentient-rl trigger-retrain - Manually trigger retraining"
echo ""
echo "To start the system:"
echo "  ./scripts/sentient-rl start"
echo ""
echo "To monitor in real-time:"
echo "  ./scripts/sentient-rl dashboard"