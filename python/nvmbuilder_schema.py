from __future__ import annotations

import json
import os
from collections import OrderedDict
from enum import Enum
from typing import Any, Dict, List, Mapping, MutableMapping, Optional, Tuple, Union

try:  # Python 3.11+
    import tomllib as _toml_loader  # type: ignore[assignment]
except Exception:  # pragma: no cover - fallback for <3.11
    import tomli as _toml_loader  # type: ignore[no-redef]

import yaml
from pydantic import BaseModel, ConfigDict, Field, RootModel, ValidationError, model_validator


# ----------------------------
# Schemas (mirror Rust serde)
# ----------------------------


class Endianness(str, Enum):
    little = "little"
    big = "big"


class CrcData(BaseModel):
    polynomial: int
    start: int
    xor_out: int
    ref_in: bool
    ref_out: bool


# CrcLocation is an untagged enum in Rust: either a string keyword or an address (u32)
CrcLocation = Union[str, int]


class Header(BaseModel):
    start_address: int
    length: int
    crc_location: CrcLocation
    padding: int = Field(default=0xFF)


class ScalarType(str, Enum):
    u8 = "u8"
    u16 = "u16"
    u32 = "u32"
    u64 = "u64"
    i8 = "i8"
    i16 = "i16"
    i32 = "i32"
    i64 = "i64"
    f32 = "f32"
    f64 = "f64"


# SizeSource: either single dimension (usize) or two-dim [usize;2]
class SizeSource(RootModel[Union[int, Tuple[int, int]]]):
    @model_validator(mode="before")
    @classmethod
    def _coerce(cls, v: Any) -> Any:
        if isinstance(v, list):
            if len(v) != 2:
                raise ValueError("size must be length-2 list when array")
            a, b = v
            if not isinstance(a, int) or not isinstance(b, int):
                raise ValueError("size list must contain integers")
            return (a, b)
        return v


# DataValue is untagged in Rust: u64, i64, f64, or string
DataValue = Union[int, float, str]


class _ValueArray(RootModel[List[DataValue]]):
    pass


class ValueSource(RootModel[Union[DataValue, _ValueArray]]):
    @property
    def is_single(self) -> bool:
        return not isinstance(self.root, _ValueArray)

    @property
    def single(self) -> DataValue:
        if isinstance(self.root, _ValueArray):
            raise ValueError("ValueSource contains an array, not a single value")
        return self.root

    @property
    def array(self) -> List[DataValue]:
        if isinstance(self.root, _ValueArray):
            return self.root.root
        return [self.root]


class _NameSource(BaseModel):
    name: str


class _ValueLiteral(BaseModel):
    value: ValueSource


EntrySource = Union[_NameSource, _ValueLiteral]


class LeafEntry(BaseModel):
    model_config = ConfigDict(extra="forbid")

    scalar_type: ScalarType = Field(alias="type")
    size: Optional[SizeSource] = None
    source: EntrySource

    @model_validator(mode="before")
    @classmethod
    def _flatten_source(cls, v: Any) -> Any:
        if not isinstance(v, Mapping):
            return v
        v = dict(v)
        has_name = "name" in v
        has_value = "value" in v
        if has_name and has_value:
            raise ValueError("LeafEntry accepts either 'name' or 'value', not both")
        if not has_name and not has_value:
            raise ValueError("LeafEntry requires either 'name' or 'value'")
        # Convert into a dedicated 'source' field while keeping aliases intact
        if has_name:
            name_val = v.pop("name")
            v["source"] = {"name": name_val}
        else:
            value_val = v.pop("value")
            v["source"] = {"value": value_val}
        return v


class Block(BaseModel):
    header: Header
    data: "Entry"


class Entry(RootModel[Union[LeafEntry, Dict[str, "Entry"]]]):
    pass


class Settings(BaseModel):
    endianness: Endianness
    crc: CrcData


class Config(BaseModel):
    # Accept extra keys at the top-level; we'll capture them into blocks
    model_config = ConfigDict(extra="allow")

    settings: Settings
    blocks: Dict[str, Block]

    @model_validator(mode="before")
    @classmethod
    def _flatten_blocks(cls, v: Any) -> Any:
        if not isinstance(v, MutableMapping):
            return v
        input_dict: Dict[str, Any] = dict(v)
        if "settings" not in input_dict:
            raise ValueError("Missing required 'settings' at top-level")
        blocks: "OrderedDict[str, Any]" = OrderedDict()
        for key, value in input_dict.items():
            if key == "settings":
                continue
            # Treat any other top-level key as a block definition
            blocks[key] = value
        return {"settings": input_dict["settings"], "blocks": blocks}


# Resolve forward references
Entry.model_rebuild()
Block.model_rebuild()


# ----------------------------
# Loader API (mirror Rust load)
# ----------------------------


def _load_text_or_bytes(path: str) -> Any:
    ext = os.path.splitext(path)[1].lower()
    if ext == ".toml":
        with open(path, "rb") as f:
            return _toml_loader.load(f)
    if ext in (".yaml", ".yml"):
        with open(path, "r", encoding="utf-8") as f:
            return yaml.safe_load(f)
    if ext == ".json":
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    raise ValueError("Unsupported file format: " + ext)


def load_layout(path: str) -> Config:
    data = _load_text_or_bytes(path)
    try:
        return Config.model_validate(data)
    except ValidationError as e:
        # Re-raise with filename context similar to Rust error formatting
        raise ValueError(f"failed to parse file {path}: {e}") from e


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Deserialize layout into Pydantic models")
    parser.add_argument("file", help="Path to layout file (.toml/.yaml/.yml/.json)")
    args = parser.parse_args()

    cfg = load_layout(args.file)
    print(cfg.model_dump_json(indent=2))

