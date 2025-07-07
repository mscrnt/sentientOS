#!/usr/bin/env python3
"""
Dependency audit for SentientOS
Finds actually used vs declared dependencies
"""

import ast
import os
from pathlib import Path
from typing import Set, Dict
import re

def extract_imports_from_py(filepath: Path) -> Set[str]:
    """Extract all imports from a Python file"""
    imports = set()
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            tree = ast.parse(f.read())
        
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    imports.add(alias.name.split('.')[0])
            elif isinstance(node, ast.ImportFrom):
                if node.module:
                    imports.add(node.module.split('.')[0])
    except:
        pass
    
    return imports

def scan_python_imports(root_dir: Path) -> Set[str]:
    """Scan all Python files for imports"""
    all_imports = set()
    
    for py_file in root_dir.rglob("*.py"):
        # Skip venv
        if ".venv" in str(py_file):
            continue
        
        imports = extract_imports_from_py(py_file)
        all_imports.update(imports)
    
    return all_imports

def map_imports_to_packages() -> Dict[str, str]:
    """Map common import names to package names"""
    return {
        'sklearn': 'scikit-learn',
        'cv2': 'opencv-python',
        'PIL': 'Pillow',
        'yaml': 'PyYAML',
        # Add more mappings as needed
    }

def main():
    root = Path(".")
    
    print("🔍 Scanning Python imports...")
    imports = scan_python_imports(root)
    
    # Filter out stdlib modules
    stdlib_modules = {
        'os', 'sys', 'json', 'time', 'datetime', 'pathlib', 'typing',
        'collections', 'itertools', 'functools', 'asyncio', 'subprocess',
        'logging', 'argparse', 're', 'math', 'random', 'pickle', 'shutil',
        'signal', 'dataclasses', 'enum', 'abc', 'io', 'tempfile', 'hashlib',
        'warnings', 'traceback', 'inspect', 'copy', 'glob', 'concurrent',
        'multiprocessing', 'threading', 'queue', 'socket', 'http', 'urllib',
        'email', 'csv', 'sqlite3', 'gzip', 'zipfile', 'tarfile', 'base64',
        'binascii', 'struct', 'array', 'decimal', 'fractions', 'statistics',
        'builtins', '__future__', 'importlib', 'contextlib', 'operator',
        'weakref', 'gc', 'atexit', 'platform', 'locale', 'gettext', 'string',
        'textwrap', 'unicodedata', 'codecs', 'encodings', 'ftplib', 'dis',
        'curses'
    }
    
    third_party = imports - stdlib_modules
    
    print(f"\n📦 Found {len(third_party)} third-party imports:")
    for imp in sorted(third_party):
        print(f"  - {imp}")
    
    # Read requirements.txt
    req_file = Path("requirements.txt")
    if req_file.exists():
        with open(req_file, 'r') as f:
            declared = set()
            for line in f:
                line = line.strip()
                if line and not line.startswith('#'):
                    # Extract package name
                    pkg = re.split(r'[<>=!]', line)[0].strip()
                    declared.add(pkg.lower())
        
        print(f"\n📋 Declared in requirements.txt: {len(declared)} packages")
        
        # Map imports to packages
        import_map = map_imports_to_packages()
        used_packages = set()
        for imp in third_party:
            pkg = import_map.get(imp, imp)
            used_packages.add(pkg.lower())
        
        # Find unused
        unused = declared - used_packages
        if unused:
            print(f"\n❌ Potentially unused packages in requirements.txt:")
            for pkg in sorted(unused):
                print(f"  - {pkg}")
        
        # Find missing
        missing = used_packages - declared
        if missing:
            print(f"\n⚠️  Used but not in requirements.txt:")
            for pkg in sorted(missing):
                print(f"  - {pkg}")

if __name__ == "__main__":
    main()