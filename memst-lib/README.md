# memst-lib

FFI/binding library for MemSt - provides cdylib and rlib targets for integration.

## Overview

`memst-lib` is a thin wrapper around `memst-core` designed for:

- **C/FFI bindings** - via cdylib output
- **Python integration** - optional PyO3 bindings
- **Library embedding** - rlib for Rust consumers

## Usage

For most use cases, prefer `memst-core` directly or use `memst-py` for Python.

### As Rust Library

```rust
use memst_lib::*;
```

### With Python Feature

Enable the `python` feature for PyO3 bindings:

```toml
[dependencies]
memst-lib = { path = ".", features = ["python"] }
```

## Features

- `python` - Enable PyO3 Python bindings

## License

MIT OR Apache-2.0
