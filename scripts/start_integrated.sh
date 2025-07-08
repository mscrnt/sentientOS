#!/bin/bash
# Start SentientOS with integrated Rust and Python services

echo "🚀 SentientOS Integrated Runtime Starting..."
echo "========================================"

# Activate Python virtual environment
source /sentientos/.venv/bin/activate

# Start core services
echo "Starting core services..."

# 1. Start Goal Processor (Python for now, Rust when ready)
echo "✅ Starting Goal Processor..."
python3 /sentientos/scripts/fast_goal_processor.py > /sentientos/logs/goal_processor.log 2>&1 &

# 2. Start LLM Observer  
echo "✅ Starting LLM Observer..."
python3 /sentientos/scripts/controlled_llm_observer.py > /sentientos/logs/llm_observer.log 2>&1 &

# 3. Start Activity Feed Bridge
echo "✅ Starting Activity Feed Bridge..."
python3 /sentientos/scripts/activity_feed_bridge.py > /sentientos/logs/activity_feed_bridge.log 2>&1 &

# 4. Start Reflective Analyzer
echo "✅ Starting Reflective Analyzer..."
python3 /sentientos/scripts/reflective_analyzer.py > /sentientos/logs/reflective_analyzer.log 2>&1 &

# 5. Start Self Improvement Loop
echo "✅ Starting Self Improvement Loop..."
python3 /sentientos/scripts/self_improvement_loop.py > /sentientos/logs/self_improvement.log 2>&1 &

# 6. Start Admin Panel
echo "✅ Starting Admin Panel..."
python3 /sentientos/scripts/unified_admin_panel.py > /sentientos/logs/admin_panel.log 2>&1 &

# Start Rust shell for interactive mode (if needed)
if [ "$SENTIENT_MODE" = "interactive" ]; then
    echo ""
    echo "🎮 Starting interactive shell..."
    echo "Type 'help' for available commands."
    echo ""
    
    # Try to build and run Rust shell, fallback to Python if needed
    if [ -f /sentientos/sentient-shell/target/release/sentient-shell ]; then
        exec /sentientos/sentient-shell/target/release/sentient-shell
    else
        echo "⚠️  Rust shell not built, using Python services only"
        # Keep container running
        tail -f /dev/null
    fi
else
    echo ""
    echo "✅ All services started!"
    echo "   Admin Panel: http://localhost:8081"
    echo "   Logs: /sentientos/logs/"
    echo ""
    
    # Keep container running and show logs
    tail -f /sentientos/logs/goal_processor.log
fi