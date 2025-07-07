#!/usr/bin/env python3
"""
Tool Chaining and Output Passing for SentientOS

This module manages structured data flow between tools,
enabling complex multi-step operations with automatic
data transformation and validation.
"""

import json
import logging
from typing import Dict, Any, List, Optional, Union, Callable, Tuple
from dataclasses import dataclass, field
from enum import Enum
import re

logger = logging.getLogger(__name__)


class DataType(Enum):
    """Supported data types for tool I/O"""
    STRING = "string"
    NUMBER = "number"
    BOOLEAN = "boolean"
    OBJECT = "object"
    ARRAY = "array"
    FILE_PATH = "file_path"
    JSON = "json"
    BINARY = "binary"


@dataclass
class IOSchema:
    """Schema definition for tool input/output"""
    name: str
    data_type: DataType
    required: bool = True
    description: str = ""
    default: Any = None
    constraints: Dict[str, Any] = field(default_factory=dict)
    
    def validate(self, value: Any) -> bool:
        """Validate value against schema"""
        if value is None:
            return not self.required
        
        # Type validation
        type_validators = {
            DataType.STRING: lambda v: isinstance(v, str),
            DataType.NUMBER: lambda v: isinstance(v, (int, float)),
            DataType.BOOLEAN: lambda v: isinstance(v, bool),
            DataType.OBJECT: lambda v: isinstance(v, dict),
            DataType.ARRAY: lambda v: isinstance(v, list),
            DataType.FILE_PATH: lambda v: isinstance(v, str) and len(v) > 0,
            DataType.JSON: lambda v: self._is_json_serializable(v),
            DataType.BINARY: lambda v: isinstance(v, bytes)
        }
        
        if not type_validators[self.data_type](value):
            return False
        
        # Constraint validation
        if "min" in self.constraints and value < self.constraints["min"]:
            return False
        if "max" in self.constraints and value > self.constraints["max"]:
            return False
        if "pattern" in self.constraints and isinstance(value, str):
            if not re.match(self.constraints["pattern"], value):
                return False
        if "enum" in self.constraints and value not in self.constraints["enum"]:
            return False
        
        return True
    
    def _is_json_serializable(self, value: Any) -> bool:
        try:
            json.dumps(value)
            return True
        except:
            return False


@dataclass
class ToolSignature:
    """Complete signature for a tool"""
    name: str
    description: str
    inputs: List[IOSchema]
    outputs: List[IOSchema]
    tags: List[str] = field(default_factory=list)
    
    def get_required_inputs(self) -> List[str]:
        return [inp.name for inp in self.inputs if inp.required]
    
    def get_output_names(self) -> List[str]:
        return [out.name for out in self.outputs]


class DataTransformer:
    """Transform data between tool formats"""
    
    def __init__(self):
        self.transformers: Dict[str, Callable] = {
            "json_to_object": json.loads,
            "object_to_json": json.dumps,
            "string_to_number": float,
            "number_to_string": str,
            "array_to_string": lambda a: "\n".join(str(i) for i in a),
            "string_to_array": lambda s: s.split("\n"),
            "extract_field": lambda obj, field: obj.get(field) if isinstance(obj, dict) else None,
            "merge_objects": lambda *objs: {k: v for obj in objs for k, v in obj.items()},
            "filter_array": lambda arr, key, value: [i for i in arr if i.get(key) == value]
        }
    
    def transform(self, data: Any, transformation: str, *args) -> Any:
        """Apply transformation to data"""
        if transformation not in self.transformers:
            raise ValueError(f"Unknown transformation: {transformation}")
        
        transformer = self.transformers[transformation]
        try:
            return transformer(data, *args)
        except Exception as e:
            logger.error(f"Transformation failed: {e}")
            raise
    
    def register_transformer(self, name: str, func: Callable):
        """Register custom transformer"""
        self.transformers[name] = func


class OutputMapper:
    """Map outputs from one tool to inputs of another"""
    
    def __init__(self):
        self.mappings: Dict[str, List[Dict[str, Any]]] = {}
        self.transformer = DataTransformer()
    
    def add_mapping(self, 
                   source_tool: str,
                   source_output: str,
                   target_tool: str,
                   target_input: str,
                   transformation: Optional[str] = None,
                   transform_args: List[Any] = None):
        """Add output-to-input mapping"""
        mapping_key = f"{source_tool}:{target_tool}"
        
        if mapping_key not in self.mappings:
            self.mappings[mapping_key] = []
        
        self.mappings[mapping_key].append({
            "source_output": source_output,
            "target_input": target_input,
            "transformation": transformation,
            "transform_args": transform_args or []
        })
    
    def map_outputs(self,
                   source_tool: str,
                   source_outputs: Dict[str, Any],
                   target_tool: str) -> Dict[str, Any]:
        """Map outputs to inputs for target tool"""
        mapping_key = f"{source_tool}:{target_tool}"
        
        if mapping_key not in self.mappings:
            # Default mapping - pass through matching names
            return source_outputs
        
        target_inputs = {}
        
        for mapping in self.mappings[mapping_key]:
            source_value = source_outputs.get(mapping["source_output"])
            
            if source_value is not None:
                # Apply transformation if specified
                if mapping["transformation"]:
                    source_value = self.transformer.transform(
                        source_value,
                        mapping["transformation"],
                        *mapping["transform_args"]
                    )
                
                target_inputs[mapping["target_input"]] = source_value
        
        return target_inputs


class ToolChain:
    """Manages tool execution chains with data flow"""
    
    def __init__(self):
        self.tools: Dict[str, ToolSignature] = {}
        self.mapper = OutputMapper()
        self.transformer = DataTransformer()
        self._setup_default_mappings()
    
    def register_tool(self, signature: ToolSignature):
        """Register a tool with its signature"""
        self.tools[signature.name] = signature
        logger.info(f"Registered tool: {signature.name}")
    
    def _setup_default_mappings(self):
        """Setup common default mappings"""
        # Memory check -> memory clean
        self.mapper.add_mapping(
            "memory_check", "usage_percent",
            "memory_clean", "threshold"
        )
        
        # Log fetch -> log filter
        self.mapper.add_mapping(
            "log_fetch", "log_entries",
            "log_filter", "entries"
        )
        
        # Log filter -> summarize
        self.mapper.add_mapping(
            "log_filter", "filtered_entries",
            "llm_summarize", "content",
            "array_to_string"
        )
    
    def validate_chain(self, tool_sequence: List[str]) -> Tuple[bool, List[str]]:
        """Validate a tool execution chain"""
        issues = []
        
        for i in range(len(tool_sequence)):
            tool_name = tool_sequence[i]
            
            # Check tool exists
            if tool_name not in self.tools:
                issues.append(f"Unknown tool: {tool_name}")
                continue
            
            tool = self.tools[tool_name]
            
            # Check input availability for non-first tools
            if i > 0:
                prev_tool_name = tool_sequence[i-1]
                prev_tool = self.tools.get(prev_tool_name)
                
                if prev_tool:
                    # Check if outputs can satisfy inputs
                    mapped_inputs = self.mapper.map_outputs(
                        prev_tool_name,
                        {out.name: None for out in prev_tool.outputs},
                        tool_name
                    )
                    
                    required_inputs = set(tool.get_required_inputs())
                    provided_inputs = set(mapped_inputs.keys())
                    missing = required_inputs - provided_inputs
                    
                    if missing:
                        issues.append(
                            f"Missing inputs for {tool_name}: {missing}"
                        )
        
        return len(issues) == 0, issues
    
    def create_chain_signature(self, tool_sequence: List[str]) -> Optional[ToolSignature]:
        """Create a composite signature for a tool chain"""
        if not tool_sequence:
            return None
        
        # Validate chain
        valid, issues = self.validate_chain(tool_sequence)
        if not valid:
            logger.error(f"Invalid chain: {issues}")
            return None
        
        # First tool inputs are chain inputs
        first_tool = self.tools[tool_sequence[0]]
        chain_inputs = first_tool.inputs
        
        # Last tool outputs are chain outputs
        last_tool = self.tools[tool_sequence[-1]]
        chain_outputs = last_tool.outputs
        
        # Create composite signature
        chain_signature = ToolSignature(
            name=f"chain_{'_'.join(tool_sequence)}",
            description=f"Chain: {' -> '.join(tool_sequence)}",
            inputs=chain_inputs,
            outputs=chain_outputs,
            tags=["composite", "chain"]
        )
        
        return chain_signature


# Tool signature definitions
def create_standard_tools() -> Dict[str, ToolSignature]:
    """Create standard tool signatures"""
    tools = {}
    
    # Memory check tool
    tools["memory_check"] = ToolSignature(
        name="memory_check",
        description="Check system memory usage",
        inputs=[],
        outputs=[
            IOSchema("total_memory", DataType.NUMBER, description="Total memory in MB"),
            IOSchema("used_memory", DataType.NUMBER, description="Used memory in MB"),
            IOSchema("free_memory", DataType.NUMBER, description="Free memory in MB"),
            IOSchema("usage_percent", DataType.NUMBER, description="Usage percentage",
                    constraints={"min": 0, "max": 100})
        ],
        tags=["system", "monitoring"]
    )
    
    # Memory clean tool
    tools["memory_clean"] = ToolSignature(
        name="memory_clean",
        description="Clean system memory",
        inputs=[
            IOSchema("threshold", DataType.NUMBER, required=False, default=80,
                    description="Usage threshold to trigger cleaning",
                    constraints={"min": 0, "max": 100})
        ],
        outputs=[
            IOSchema("freed_memory", DataType.NUMBER, description="Freed memory in MB"),
            IOSchema("success", DataType.BOOLEAN, description="Cleaning success")
        ],
        tags=["system", "maintenance"]
    )
    
    # Log fetch tool
    tools["log_fetch"] = ToolSignature(
        name="log_fetch",
        description="Fetch log entries",
        inputs=[
            IOSchema("time_range", DataType.STRING, required=False, default="24h",
                    description="Time range (e.g., '24h', '7d')")
        ],
        outputs=[
            IOSchema("log_entries", DataType.ARRAY, description="Array of log entries"),
            IOSchema("count", DataType.NUMBER, description="Number of entries")
        ],
        tags=["logging", "monitoring"]
    )
    
    # Log filter tool
    tools["log_filter"] = ToolSignature(
        name="log_filter",
        description="Filter log entries",
        inputs=[
            IOSchema("entries", DataType.ARRAY, description="Log entries to filter"),
            IOSchema("filter", DataType.STRING, description="Filter pattern")
        ],
        outputs=[
            IOSchema("filtered_entries", DataType.ARRAY, description="Filtered entries"),
            IOSchema("count", DataType.NUMBER, description="Number of matches")
        ],
        tags=["logging", "analysis"]
    )
    
    # LLM summarize tool
    tools["llm_summarize"] = ToolSignature(
        name="llm_summarize",
        description="Summarize content using LLM",
        inputs=[
            IOSchema("content", DataType.STRING, description="Content to summarize"),
            IOSchema("max_length", DataType.NUMBER, required=False, default=500,
                    description="Maximum summary length")
        ],
        outputs=[
            IOSchema("summary", DataType.STRING, description="Generated summary"),
            IOSchema("key_points", DataType.ARRAY, description="Key points extracted")
        ],
        tags=["llm", "analysis"]
    )
    
    return tools


def demo_tool_chaining():
    """Demonstrate tool chaining capabilities"""
    # Create tool chain manager
    chain_manager = ToolChain()
    
    # Register standard tools
    for tool in create_standard_tools().values():
        chain_manager.register_tool(tool)
    
    # Example 1: Memory monitoring chain
    print("\n📊 Memory Monitoring Chain")
    memory_chain = ["memory_check", "memory_clean"]
    valid, issues = chain_manager.validate_chain(memory_chain)
    print(f"  Valid: {valid}")
    if issues:
        print(f"  Issues: {issues}")
    
    # Example 2: Log analysis chain
    print("\n📋 Log Analysis Chain")
    log_chain = ["log_fetch", "log_filter", "llm_summarize"]
    valid, issues = chain_manager.validate_chain(log_chain)
    print(f"  Valid: {valid}")
    
    # Create composite signature
    chain_sig = chain_manager.create_chain_signature(log_chain)
    if chain_sig:
        print(f"  Chain: {chain_sig.name}")
        print(f"  Inputs: {[i.name for i in chain_sig.inputs]}")
        print(f"  Outputs: {[o.name for o in chain_sig.outputs]}")
    
    # Example 3: Data flow
    print("\n🔄 Data Flow Example")
    
    # Simulate tool outputs
    memory_output = {
        "total_memory": 16384,
        "used_memory": 14000,
        "usage_percent": 85.4
    }
    
    # Map to next tool
    clean_inputs = chain_manager.mapper.map_outputs(
        "memory_check", memory_output, "memory_clean"
    )
    print(f"  Memory check output: {memory_output}")
    print(f"  Mapped to clean inputs: {clean_inputs}")


if __name__ == "__main__":
    demo_tool_chaining()