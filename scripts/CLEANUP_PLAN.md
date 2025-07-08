# Script Cleanup Plan for Phase 9

## Current State
The scripts directory contains a mix of Python scripts, test files, and utilities. Most core functionality has been migrated to Rust.

## Scripts to Keep (Temporarily)
These will remain until their Rust equivalents are fully tested:

1. **fast_goal_processor.py** → Keep until `activity_loop.rs` is validated
2. **controlled_llm_observer.py** → Keep until `llm_observer.rs` is validated
3. **unified_admin_panel.py** → Remove (replaced by native `web_ui.rs`)
4. **reflective_analyzer.py** → Keep as reference for Rust implementation
5. **self_improvement_loop.py** → Keep as reference for Rust implementation
6. **activity_feed_bridge.py** → Remove (functionality integrated into Rust services)
7. **inject_goal.py** → Remove (replaced by `sentientctl inject-goal`)
8. **start_integrated.sh** → Keep (Docker startup script)
9. **start_services.sh** → Remove (replaced by service manager)

## Scripts to Archive
Move to `scripts/archive/phase8/`:

- All `test_*.py` files
- All validation scripts (`validate_*.py`)
- `generate_bootstrap_traces.py`
- `trace_collector.py`
- `demo_llm_pipeline.py`
- `list_ollama_models.py`

## Scripts to Delete
Already superseded:

- `unified_admin_panel.py` (replaced by Rust web UI)
- `activity_feed_bridge.py` (integrated into services)
- `inject_goal.py` (replaced by sentientctl)
- Old shell scripts (`*.sh` except `start_integrated.sh`)

## New Structure
```
scripts/
├── README.md
├── start_integrated.sh     # Docker entrypoint
├── archive/
│   ├── phase8/            # Python implementations
│   └── old_scripts/       # Legacy scripts
└── dev/                   # Development utilities only
```

## Migration Status

| Component | Python | Rust | Status |
|-----------|--------|------|---------|
| Goal Processor | fast_goal_processor.py | activity_loop.rs | ✅ Migrated |
| LLM Observer | controlled_llm_observer.py | llm_observer.rs | ✅ Migrated |
| Admin Panel | unified_admin_panel.py | web_ui.rs | ✅ Migrated |
| Reflective Analyzer | reflective_analyzer.py | reflective.rs | 🔄 TODO |
| Self Improvement | self_improvement_loop.py | improvement.rs | 🔄 TODO |
| Goal Injection | inject_goal.py | sentientctl | ✅ Migrated |
| Log Viewing | Various | sentientctl logs | ✅ Migrated |
| Monitoring | monitor_*.py | sentientctl monitor | ✅ Migrated |

## Cleanup Commands
```bash
# Archive old scripts
mkdir -p scripts/archive/phase8
mv scripts/test_*.py scripts/archive/phase8/
mv scripts/validate_*.py scripts/archive/phase8/
mv scripts/generate_bootstrap_traces.py scripts/archive/phase8/
mv scripts/trace_collector.py scripts/archive/phase8/

# Remove replaced scripts
rm scripts/unified_admin_panel.py
rm scripts/activity_feed_bridge.py
rm scripts/inject_goal.py
rm scripts/start_services.sh

# Keep only essential scripts
ls scripts/*.py
# Should show only: fast_goal_processor.py, controlled_llm_observer.py, 
# reflective_analyzer.py, self_improvement_loop.py
```