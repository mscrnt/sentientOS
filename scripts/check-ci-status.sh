#!/bin/bash

# Script to check GitHub Actions CI status for SentientOS

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🔍 Checking CI status for SentientOS...${NC}"
echo "======================================="

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    echo -e "${YELLOW}⚠️  GitHub CLI (gh) not found${NC}"
    echo "Install with: https://cli.github.com/"
    echo ""
    echo "Alternatively, check manually at:"
    echo "https://github.com/mscrnt/sentientOS/actions"
    exit 1
fi

# Get workflow runs
echo -e "\n${BLUE}📊 Recent workflow runs:${NC}"
gh run list --limit 10 --repo mscrnt/sentientOS

# Get latest run details
LATEST_RUN=$(gh run list --limit 1 --repo mscrnt/sentientOS --json databaseId --jq '.[0].databaseId' 2>/dev/null || echo "")

if [ -n "$LATEST_RUN" ]; then
    echo -e "\n${BLUE}📋 Latest run details:${NC}"
    gh run view $LATEST_RUN --repo mscrnt/sentientOS
    
    # Check if we can view logs
    echo -e "\n${BLUE}📜 To view full logs:${NC}"
    echo "gh run view $LATEST_RUN --repo mscrnt/sentientOS --log"
    
    # Get job status
    echo -e "\n${BLUE}💼 Job statuses:${NC}"
    gh run view $LATEST_RUN --repo mscrnt/sentientOS --json jobs --jq '.jobs[] | "\(.name): \(.conclusion // .status)"'
fi

# Show badge status
echo -e "\n${BLUE}🏷️  Badge URL:${NC}"
echo "https://github.com/mscrnt/sentientOS/actions/workflows/qemu-test.yml/badge.svg"

echo -e "\n${GREEN}✅ Check complete${NC}"