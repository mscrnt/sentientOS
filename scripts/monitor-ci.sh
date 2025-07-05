#!/bin/bash

# Monitor GitHub Actions CI runs for SentientOS
# Updates every 30 seconds until the run completes

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}👀 Monitoring CI for SentientOS...${NC}"
echo "Press Ctrl+C to stop monitoring"
echo ""

# Get the latest commit SHA
LATEST_COMMIT=$(git rev-parse HEAD)
echo -e "${BLUE}📍 Latest commit: ${NC}$LATEST_COMMIT"

# Function to check run status
check_status() {
    # Try using gh CLI if available
    if command -v gh &> /dev/null; then
        # Get runs for this commit
        STATUS=$(gh run list --commit $LATEST_COMMIT --repo mscrnt/sentientOS --limit 1 --json status,conclusion,name --jq '.[0] | "\(.name): \(.conclusion // .status)"' 2>/dev/null || echo "No runs found")
        echo -e "$(date '+%H:%M:%S') - $STATUS"
        
        # Check if completed
        if [[ $STATUS == *"completed"* ]] || [[ $STATUS == *"success"* ]] || [[ $STATUS == *"failure"* ]]; then
            if [[ $STATUS == *"success"* ]]; then
                echo -e "\n${GREEN}✅ CI run completed successfully!${NC}"
            else
                echo -e "\n${RED}❌ CI run failed!${NC}"
            fi
            return 0
        fi
    else
        echo -e "${YELLOW}GitHub CLI not available. Check manually at:${NC}"
        echo "https://github.com/mscrnt/sentientOS/actions"
        return 0
    fi
    return 1
}

# Monitor loop
while true; do
    if check_status; then
        break
    fi
    sleep 30
done

echo -e "\n${BLUE}📊 Final status check:${NC}"
if command -v gh &> /dev/null; then
    gh run list --limit 5 --repo mscrnt/sentientOS
fi