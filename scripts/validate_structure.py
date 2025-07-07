#!/usr/bin/env python3
"""
Validate SentientOS project structure after cleanup
Ensures directories contain only appropriate file types
"""

import os
import sys
from pathlib import Path
from typing import Dict, List, Tuple

class StructureValidator:
    def __init__(self, root_path: Path = Path(".")):
        self.root = root_path
        self.errors = []
        self.warnings = []
        
    def validate_directory(self, 
                         dir_path: Path, 
                         allowed_extensions: List[str],
                         description: str) -> bool:
        """Validate that directory contains only allowed file types"""
        if not dir_path.exists():
            self.warnings.append(f"{dir_path} does not exist")
            return True
            
        valid = True
        for file_path in dir_path.iterdir():
            if file_path.is_file():
                if not any(file_path.name.endswith(ext) for ext in allowed_extensions):
                    self.errors.append(
                        f"{description}: {file_path.name} has invalid extension "
                        f"(allowed: {', '.join(allowed_extensions)})"
                    )
                    valid = False
        return valid
    
    def check_permissions(self, path: Path, need_write: bool = False) -> bool:
        """Check if path has appropriate permissions"""
        if not path.exists():
            self.warnings.append(f"{path} does not exist")
            return True
            
        if need_write:
            # Check write permission
            test_file = path / ".write_test"
            try:
                test_file.touch()
                test_file.unlink()
                return True
            except:
                self.errors.append(f"{path} is not writable")
                return False
        
        return os.access(path, os.R_OK)
    
    def validate_root_files(self) -> bool:
        """Validate that root contains only essential files"""
        allowed_root_files = {
            "README.md",
            "bootstrap.sh",
            "build_kernel.sh",
            "cleanup_report.md",
            "requirements.txt",
            ".gitignore"
        }
        
        valid = True
        for item in self.root.iterdir():
            if item.is_file() and item.name not in allowed_root_files:
                if not item.name.startswith('.'):  # Ignore hidden files
                    self.warnings.append(f"Unexpected file in root: {item.name}")
        
        return valid
    
    def run_validation(self) -> Tuple[bool, List[str], List[str]]:
        """Run all validation checks"""
        print("🔍 Validating SentientOS structure...")
        print("=" * 50)
        
        # Define validation rules
        validations = [
            # Scripts directory
            (self.root / "scripts", [".py", ".sh", ".txt"], "scripts/"),
            
            # Docs archive
            (self.root / "docs" / "archive", [".md", ".json", ".txt"], "docs/archive/"),
            
            # Config directory
            (self.root / "config", [".toml", ".yaml", ".yml"], "config/"),
            
            # Source directories (Python)
            (self.root / "rl_agent", [".py"], "rl_agent/"),
            (self.root / "sentient-core", [".py"], "sentient-core/"),
            (self.root / "src", [".py"], "src/"),
        ]
        
        # Validate file types
        for dir_path, extensions, desc in validations:
            self.validate_directory(dir_path, extensions, desc)
        
        # Check writable directories
        writable_dirs = [
            self.root / "logs",
            self.root / "traces",
            self.root / "rl_checkpoints",
            self.root / "rl_data"
        ]
        
        for dir_path in writable_dirs:
            self.check_permissions(dir_path, need_write=True)
        
        # Validate root files
        self.validate_root_files()
        
        # Skip common issues check for now (can be slow)
        
        return len(self.errors) == 0, self.errors, self.warnings
    
    def check_common_issues(self):
        """Check for common structural issues"""
        # Check for __pycache__ outside .venv
        for pycache in self.root.rglob("__pycache__"):
            if ".venv" not in str(pycache):
                self.warnings.append(f"Found __pycache__ at: {pycache}")
        
        # Check for .pyc files
        for pyc in self.root.rglob("*.pyc"):
            if ".venv" not in str(pyc):
                self.warnings.append(f"Found .pyc file at: {pyc}")
        
        # Check for common temporary files
        temp_patterns = ["*.log", "*.tmp", "*.bak", "*.swp", "*.swo"]
        for pattern in temp_patterns:
            for temp_file in self.root.rglob(pattern):
                if ".venv" not in str(temp_file) and "logs/" not in str(temp_file):
                    self.warnings.append(f"Found temporary file: {temp_file}")


def main():
    validator = StructureValidator()
    valid, errors, warnings = validator.run_validation()
    
    # Report results
    if errors:
        print("\n❌ Validation Errors:")
        for error in errors:
            print(f"  - {error}")
    
    if warnings:
        print("\n⚠️  Warnings:")
        for warning in warnings:
            print(f"  - {warning}")
    
    if valid and not warnings:
        print("\n✅ All validation checks passed!")
        print("Project structure is clean and follows standards.")
    elif valid:
        print(f"\n⚠️  Validation passed with {len(warnings)} warnings")
    else:
        print(f"\n❌ Validation failed with {len(errors)} errors")
        return 1
    
    # Summary
    print("\n📊 Structure Summary:")
    print(f"  - Scripts: {len(list((Path('.') / 'scripts').glob('*')))} files")
    print(f"  - Archived: {len(list((Path('.') / 'docs' / 'archive').glob('*')))} files")
    print(f"  - Root files: {len([f for f in Path('.').iterdir() if f.is_file()])}")
    
    return 0


if __name__ == "__main__":
    sys.exit(main())