
# Requirements Document

## Introduction

This document specifies the requirements for Phase 1 performance optimizations in dx-py-package-manager, targeting 5x+ faster performance than uv through three core features: O(1) Virtual Environment Layout Cache, Binary Lock File (DPL Format), and Memory-Mapped Package Store. These features form the foundation for achieving 100x faster warm installs and dramatically reduced cold install times.

## Glossary

- Layout_Cache: A binary-indexed cache system that stores pre-built virtual environment layouts for O(1) retrieval
- DPL_File: Binary lock file format (DX-Py Lock) replacing JSON/TOML for O(1) package lookup
- Package_Store: Memory-mapped storage system for wheel packages enabling zero-copy access
- Project_Hash: A unique identifier computed from project dependencies and constraints
- Warm_Install: Installation where the layout cache contains a matching pre-built environment
- Cold_Install: Installation requiring fresh resolution and layout building
- Symlink_Junction: Platform-appropriate filesystem link (symlink on Unix, junction on Windows)
- Blake3_Hash: Cryptographic hash algorithm used for content addressing
- Site_Packages: Python directory containing installed packages within a virtual environment

## Requirements

### Requirement 1: O(1) Virtual Environment Layout Cache

User Story: As a developer, I want instant virtual environment setup on repeated installs, so that I can iterate quickly without waiting for package installation.

#### Acceptance Criteria

- WHEN a project hash matches an existing cached layout, THE Layout_Cache SHALL complete installation in under 10ms by creating a single symlink or junction
- WHEN a project hash does not exist in cache, THE Layout_Cache SHALL build the layout, cache it, and complete installation
- THE Layout_Cache SHALL store layout index in a binary format at ~/.dx-py/layouts.dxc for O(1) lookup
- WHEN the cache index is loaded, THE Layout_Cache SHALL use memory-mapping to avoid loading the entire index into memory
- THE Layout_Cache SHALL compute project hashes using Blake3 from the resolved dependency set
- WHEN creating filesystem links, THE Layout_Cache SHALL use symlinks on Unix and junctions on Windows
- IF the cached layout is corrupted or missing files, THEN THE Layout_Cache SHALL rebuild the layout and update the cache
- THE Layout_Cache SHALL support concurrent access from multiple dx-py processes without corruption

### Requirement 2: Binary Lock File Format (DPL)

User Story: As a developer, I want lock file operations to be instant, so that dependency resolution and installation can start immediately without parsing overhead.

#### Acceptance Criteria

- THE DPL_File SHALL use a binary format with magic bytes "DXPL" for identification
- THE DPL_File SHALL store package entries in a hash table structure for O(1) lookup by package name
- WHEN reading a package entry, THE DPL_File SHALL return the result without parsing the entire file
- THE DPL_File SHALL store pre-computed name hashes to enable direct memory access
- THE DPL_File SHALL include version information as packed integers (major, minor, patch)
- THE DPL_File SHALL store extras as a bitmap for efficient feature flag checking
- THE DPL_File SHALL include Blake3 wheel hashes for integrity verification
- THE DPL_File_Writer SHALL serialize lock data to binary format
- THE DPL_File_Reader SHALL deserialize binary format back to structured data
- FOR ALL valid DPL structures, serializing then deserializing SHALL produce an equivalent structure (round-trip property)
- WHEN the DPL file is corrupted or has invalid magic bytes, THE DPL_File_Reader SHALL return a descriptive error

### Requirement 3: Memory-Mapped Package Store

User Story: As a developer, I want package access without disk I/O overhead, so that installation and package loading are as fast as possible.

#### Acceptance Criteria

- THE Package_Store SHALL store packages in a content-addressed directory structure using Blake3 hashes
- THE Package_Store SHALL memory-map package files for zero-copy access
- WHEN accessing a file within a package, THE Package_Store SHALL return a slice into the memory-mapped region without copying
- THE Package_Store SHALL maintain a file index within each package for O(1) file lookup
- WHEN installing to a virtual environment, THE Package_Store SHALL create symlinks to the store rather than copying files
- THE Package_Store SHALL use the path format ~/.dx-py/store/{hash[0:2]}/{hash[2:4]}/{hash}.dxpkg
- IF a package file is not found in the store, THEN THE Package_Store SHALL return an appropriate error
- THE Package_Store SHALL support concurrent read access from multiple processes
- WHEN a package is added to the store, THE Package_Store SHALL verify its integrity using Blake3 hash
- THE Package_Store SHALL reduce disk usage by sharing packages across all projects

### Requirement 4: Integration and CLI Support

User Story: As a developer, I want the performance features to work seamlessly with existing dx-py commands, so that I get faster performance without changing my workflow.

#### Acceptance Criteria

- WHEN running `dx-py install`, THE CLI SHALL use the Layout_Cache for warm installs when available
- WHEN running `dx-py lock`, THE CLI SHALL generate a DPL binary lock file
- WHEN running `dx-py install`, THE CLI SHALL read from DPL lock files for dependency information
- THE CLI SHALL fall back to standard installation if cache or store is unavailable
- WHEN running `dx-py cache clean`, THE CLI SHALL provide options to clear layout cache, package store, or both
- THE CLI SHALL display cache hit/miss statistics when running with verbose flag
- WHEN the Layout_Cache provides a warm install, THE CLI SHALL complete the install command in under 50ms total
