#!/bin/bash
# Migration Script: Python RL → Rust RL
# Phase 10 Final Migration

set -e

echo "🦀 SentientOS RL Migration: Python → Rust"
echo "========================================="
echo ""

# Check if running as root/sudo
if [ "$EUID" -eq 0 ]; then 
   echo "✅ Running with appropriate permissions"
else
   echo "❌ Please run with sudo"
   exit 1
fi

# Step 1: Stop existing services
echo "📍 Step 1: Stopping existing services..."
docker-compose down 2>/dev/null || true
systemctl stop sentient-learning.service 2>/dev/null || true

# Step 2: Backup existing data
echo ""
echo "📍 Step 2: Creating backups..."
BACKUP_DIR="/var/backups/sentientos_python_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Backup Python checkpoints if they exist
if [ -d "/models/python_checkpoints" ]; then
    echo "  - Backing up Python checkpoints"
    cp -r /models/python_checkpoints "$BACKUP_DIR/" || true
fi

# Backup notebooks if they exist
if [ -d "/notebooks/rl" ]; then
    echo "  - Backing up RL notebooks"
    cp -r /notebooks/rl "$BACKUP_DIR/" || true
fi

# Step 3: Run Python cleanup
echo ""
echo "📍 Step 3: Removing Python RL infrastructure..."
cd /mnt/d/Projects/SentientOS
./scripts/python_rl_cleanup.sh

# Step 4: Build new Rust-only images
echo ""
echo "📍 Step 4: Building Rust-only Docker images..."
docker build -f Dockerfile.rust-only -t sentientos:rust-only .

# Step 5: Update active configuration
echo ""
echo "📍 Step 5: Updating configuration files..."
# Backup old docker-compose
cp docker-compose.yml docker-compose.yml.python-backup
# Use new Rust-only compose
cp docker-compose.rust-only.yml docker-compose.yml

# Step 6: Create Rust RL directories
echo ""
echo "📍 Step 6: Setting up Rust RL directories..."
mkdir -p /var/rl_checkpoints/{policies,replay_buffers}
chown -R 1000:1000 /var/rl_checkpoints

# Step 7: Remove Python systemd services
echo ""
echo "📍 Step 7: Removing Python systemd services..."
systemctl disable sentient-learning.service 2>/dev/null || true
rm -f /etc/systemd/system/sentient-learning.service
systemctl daemon-reload

# Step 8: Create new systemd service for Rust RL
echo ""
echo "📍 Step 8: Creating Rust RL systemd service..."
cat > /etc/systemd/system/sentient-rl.service << EOF
[Unit]
Description=SentientOS Rust RL Service
After=docker.service
Requires=docker.service

[Service]
Type=simple
Restart=always
RestartSec=10
ExecStart=/usr/bin/docker-compose up sentientos
ExecStop=/usr/bin/docker-compose down
WorkingDirectory=/mnt/d/Projects/SentientOS

[Install]
WantedBy=multi-user.target
EOF

systemctl enable sentient-rl.service

# Step 9: Start new services
echo ""
echo "📍 Step 9: Starting Rust-only services..."
docker-compose up -d

# Step 10: Run verification test
echo ""
echo "📍 Step 10: Running verification tests..."
sleep 5  # Wait for services to start

# Test 1: Check if sentientctl works
echo "  - Testing sentientctl..."
docker exec sentientos-runtime sentientctl --version || {
    echo "❌ sentientctl test failed"
    exit 1
}

# Test 2: Quick RL training test
echo "  - Testing RL training..."
docker exec sentientos-runtime sentientctl rl train --agent random --env cartpole --episodes 10 || {
    echo "❌ RL training test failed"
    exit 1
}

echo ""
echo "✅ Migration Complete!"
echo ""
echo "📊 Verification Summary:"
echo "  - Python RL components: REMOVED"
echo "  - Rust RL components: ACTIVE"
echo "  - Backup location: $BACKUP_DIR"
echo ""
echo "🚀 Next Steps:"
echo "  1. Access RL Dashboard: http://localhost:8081/rl"
echo "  2. Start training: docker-compose --profile training up rl-trainer"
echo "  3. Monitor logs: docker logs -f sentientos-runtime"
echo ""
echo "📝 To rollback if needed:"
echo "  - Restore from: $BACKUP_DIR"
echo "  - Use: docker-compose.yml.python-backup"