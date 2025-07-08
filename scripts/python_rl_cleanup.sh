#!/bin/bash
# Python RL Infrastructure Cleanup Script
# Removes all Python-based RL components in favor of native Rust implementation

set -e

echo "🧹 Starting Python RL Infrastructure Cleanup..."
echo "================================================"

# Define directories and files to remove
PYTHON_RL_DIRS=(
    "rl_agent"
    "rl_data"
    "policies"  # Old Python policies
    ".venv"     # Python virtual environment
)

PYTHON_RL_FILES=(
    "requirements.txt"
    "scripts/install-continuous-learning.sh"
    "scripts/sentient-learning.service"
    "scripts/sentient-rl"
    "*test*agent*.py"
    "*train*.py"
    "*learning*.py"
    "*ppo*.py"
)

# Backup directory
BACKUP_DIR="python_rl_backup_$(date +%Y%m%d_%H%M%S)"

echo "📦 Creating backup directory: $BACKUP_DIR"
mkdir -p "$BACKUP_DIR"

# Backup Python RL directories
echo ""
echo "📂 Backing up Python RL directories..."
for dir in "${PYTHON_RL_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        echo "  - Backing up $dir"
        cp -r "$dir" "$BACKUP_DIR/" 2>/dev/null || true
    fi
done

# Backup Python RL files
echo ""
echo "📄 Backing up Python RL files..."
for pattern in "${PYTHON_RL_FILES[@]}"; do
    find . -name "$pattern" -type f 2>/dev/null | while read -r file; do
        echo "  - Backing up $file"
        mkdir -p "$BACKUP_DIR/$(dirname "$file")"
        cp "$file" "$BACKUP_DIR/$file" 2>/dev/null || true
    done
done

# Remove Python RL directories
echo ""
echo "🗑️  Removing Python RL directories..."
for dir in "${PYTHON_RL_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        echo "  - Removing $dir"
        rm -rf "$dir"
    fi
done

# Remove Python RL files
echo ""
echo "🗑️  Removing Python RL files..."
for pattern in "${PYTHON_RL_FILES[@]}"; do
    find . -name "$pattern" -type f 2>/dev/null | while read -r file; do
        echo "  - Removing $file"
        rm -f "$file"
    done
done

# Clean up Docker images with Python
echo ""
echo "🐳 Cleaning up Docker images..."
docker images | grep -E "(python|pytorch|tensorflow)" | awk '{print $3}' | xargs -r docker rmi -f 2>/dev/null || true

# Update .gitignore to exclude Python artifacts
echo ""
echo "📝 Updating .gitignore..."
cat >> .gitignore << EOF

# Python RL artifacts (removed in Phase 10)
*.pyc
__pycache__/
.venv/
venv/
*.egg-info/
.pytest_cache/
.ipynb_checkpoints/
*.pkl
*.pth
*.h5
rl_agent/
rl_data/
EOF

echo ""
echo "✅ Python RL cleanup complete!"
echo "📁 Backup saved to: $BACKUP_DIR"
echo ""
echo "🦀 The system now uses only Rust-based RL components:"
echo "   - sentient-rl-core"
echo "   - sentient-rl-agent"
echo "   - sentient-rl-env"
echo "   - sentient-memory"
echo ""
echo "🚀 To verify the Rust RL system:"
echo "   sentientctl rl train --agent random --env cartpole --episodes 10"