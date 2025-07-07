#!/bin/bash
# Phase 2-3 Demo: RAG-Tool Fusion and RL Tracing

echo "🧠 SentientOS Phase 2-3 Demo: Intelligent Action + Learning"
echo "=========================================================="
echo

echo "📚 Example 1: Pure Query (RAG only)"
echo "$ sentient-shell"
echo "> rag_tool \"What is the purpose of SentientOS?\" --explain"
echo

echo "🔧 Example 2: Pure Action (Tool only)"
echo "> rag_tool \"check disk space\" --explain"
echo

echo "🔄 Example 3: Query Then Action (Conditional execution)"
echo "> rag_tool \"My system feels slow, what should I check?\" --explain"
echo

echo "💡 Example 4: Complex Hybrid Query"
echo "> rag_tool \"Check memory usage and explain if I need to clean cache\" --explain"
echo

echo "📊 Example 5: View RL Traces"
echo "> rl trace summary"
echo "> rl trace list -n 5"
echo "> rl trace best"
echo

echo "🎯 Key Features Demonstrated:"
echo "- Intelligent intent detection (Pure Query vs Action vs Hybrid)"
echo "- RAG system provides knowledge before tool execution"
echo "- Conditional tool execution based on RAG results"
echo "- User feedback collection for reinforcement learning"
echo "- Trace logging for all executions"
echo "- Performance analysis and model selection insights"
echo

echo "📝 Configuration Files:"
echo "- config/conditions.yaml - Tool execution conditions"
echo "- config/rewards.yaml - Reward policy definitions"
echo "- logs/rl_trace.jsonl - Execution traces with rewards"
echo

echo "🚀 Next Steps (Phase 3):"
echo "- Train PPO agent on collected traces"
echo "- Optimize model/tool selection based on rewards"
echo "- Implement exploration vs exploitation strategies"
echo "- Auto-tune condition thresholds based on feedback"