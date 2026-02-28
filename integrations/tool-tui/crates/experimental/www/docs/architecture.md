
# dx-www Architecture

This document describes the architecture of the dx-www framework, a high-performance web runtime that compiles TSX/JSX to optimized binary artifacts.

## Overview

dx-www transforms React-like components into a binary streaming format (HTIP) that enables: -Zero-parse rendering - Binary format requires no parsing on the client -Delta patching - Incremental updates send only changed bytes -Automatic runtime selection - Micro (338B) or Macro (7.5KB) based on complexity

## System Architecture

@flow:TD[]

## Compilation Pipeline

The compilation pipeline transforms TSX source through multiple stages: @seq[]

## HTIP Binary Format

The Hyper Text Interchange Protocol (HTIP) is a compact binary format:
```mermaid
block-beta columns 1 block:header["HEADER (77 bytes)"]
columns 4 magic["Magic<br/>DXB1<br/>4 bytes"]
version["Version<br/>1<br/>1 byte"]
signature["Ed25519 Signature<br/>64 bytes"]
counts["Counts<br/>8 bytes"]
end block:strings["STRING TABLE (variable)"]
columns 1 st["Length-prefixed UTF-8 strings"]
end block:templates["TEMPLATE DICTIONARY (variable)"]
columns 1 td["Bincode-encoded template definitions"]
end block:opcodes["OPCODE STREAM (variable)"]
columns 1 os["u8 opcode + bincode payload"]
end ```


## Request Flow


How a browser request flows through the system: @seq[]


## State Update Flow


How state changes propagate to the DOM: @flow:LR[]


## Crate Dependencies


@flow:TD[]


## Security Model


@flow:TD[]


## Performance Characteristics


+--------+---------+-------+
| Metric | Value   | Notes |
+========+=========+=======+
| Full   | payload | ~10   |
+--------+---------+-------+


## Directory Structure


@tree:dx-www[]
