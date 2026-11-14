# MemSt apps (web server, web UI, desktop shell)

Everything in this directory is an **application built on top of MemSt**, not
part of the memory engine. It is isolated from the Rust workspace on purpose:

- `cargo build` / `cargo test --workspace` at the repo root never touch it.
- Nothing in the core crates (`memst-core`, `memst-repo`, `memst-sleep`,
  `memst-mcp`, `memst-graph-helix`, `memst-cli`, `memst-py`, ...) depends on it.
- The dependency points one way only: apps -> `memst` Python module
  (built from `memst-py`) -> core crates.

| Path | What it is | Stack |
|------|------------|-------|
| `memst-server/` | REST backend with nanobot agent integration | Python, FastAPI, DuckDB |
| `memst-ui/` | Web UI (chat, search, KG extraction, canvas) | React, TypeScript, Vite |
| `memst-ui/src-tauri/` | Optional desktop shell around the web UI | Tauri 2 (own Cargo project) |
| `third/nanobot/` | Agent framework used by the server | git submodule |
| `third/agent-skills/` | Web-search skills loaded by the agent | Python scripts |
| `server.sh` | start/stop/status/logs wrapper for the server | bash |
| `memst-server-api.md` | REST API reference | |
| `assets/figures/` | UI screenshots | |

![ui-chat](./assets/figures/memst-ui-chat.png)

## Run

All commands below are written from the **repository root**.

![ui-chat](./assets/figures/memst-ui-chat.png)

```bash
# Terminal 1: Start the backend server
cd apps/memst-server
PYTHONPATH=src .venv/bin/python -m memst_server.main

# Terminal 2: Start the frontend (from the repo root)
cd apps/memst-ui

# Configure the frontend API URL (optional - see below)
# By default, frontend connects to http://127.0.0.1:8193

npm run dev
```

The frontend will be available at `http://localhost:3000` and will communicate with the backend at `http://127.0.0.1:8193`.

**Frontend Configuration:**

The frontend reads the backend API URL from environment variables. Create a `.env.local` file in the `memst-ui` directory to override defaults:

```bash
cd apps/memst-ui
echo 'VITE_API_HOST=127.0.0.1' > .env.local
echo 'VITE_API_PORT=8193' >> .env.local
echo 'VITE_DEV_PORT=3000' >> .env.local
```

Example `.env.local` for a remote backend:
```
VITE_API_HOST=192.168.1.100
VITE_API_PORT=8193
VITE_DEV_PORT=3001
```

Available environment variables:
- `VITE_API_HOST` - Backend API host (default: `127.0.0.1`)
- `VITE_API_PORT` - Backend API port (default: `8193`)
- `VITE_DEV_PORT` - Frontend dev server port (default: `3000`)

**Note:** Ensure `cors_origins` is set in server's `config.toml` to allow frontend access.


For the REST API see [memst-server-api.md](memst-server-api.md); server
configuration is described in [memst-server/CONFIG.md](memst-server/CONFIG.md).

## Setup

**Prerequisites:**

```bash
# Clone submodules (nanobot)
git submodule update --init --recursive
```

**Setup using uv (recommended):**

```bash
# Create a fresh virtual environment with Python 3.13
cd apps/memst-server
uv venv --python 3.13

# Install server dependencies
uv pip install fastapi uvicorn duckdb python-dotenv pydantic pydantic-settings httpx toml python-multipart

# Install nanobot (from submodule)
uv pip install -e ../third/nanobot

# Build and install memst-py using maturin
cd ../../memst-py

# Build release wheel
unset CONDA_PREFIX
uv run maturin build --release

# Install the wheel in server venv
cd ../apps/memst-server
uv pip install ../../target/wheels/memst_py-*.whl

# Verify
PYTHONPATH=src .venv/bin/python -c "import memst; print(memst.__version__)"
```

**Running the server:**

```bash
# Option 1: Using the management script (recommended)
./apps/server.sh --start      # Start server
./apps/server.sh --stop       # Stop server
./apps/server.sh --restart    # Restart server
./apps/server.sh --status     # Check status
./apps/server.sh --check      # Validate configuration
./apps/server.sh --maintain   # Run maintenance checks (config validation, status, logs)
./apps/server.sh --logs       # View server logs

# Option 2: Manual start
cd apps/memst-server

# Set PYTHONPATH to include the src directory
PYTHONPATH=src .venv/bin/python -m memst_server.main
```

**Configuration:**

Create or edit `config.toml` in `apps/memst-server/`:

```toml
[server]
port = 8193
store_path = "/tmp/data"
# Add CORS origins for frontend access
cors_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]
```

**Troubleshooting:**

- **Python version**: Python 3.13 is recommended. If you have issues with Python 3.14, use 3.13: `uv venv --python 3.13`
- **Module not found**: If you get `ModuleNotFoundError: No module named 'memst_server'`, ensure `PYTHONPATH=src` is set
- **CORS errors**: If frontend CORS errors occur, add your frontend origin to `cors_origins` in config.toml
- **Frontend connection**: If frontend can't connect to backend, check the API URL in `apps/memst-ui/.env.local`
- **Nanobot not available**: Ensure you ran `git submodule update --init --recursive` and installed nanobot with `uv pip install -e apps/third/nanobot`
- **memst-py build fails**: Make sure to `unset CONDA_PREFIX` before building with maturin
- **SQLite/DuckDB issues**: Ensure you have the proper build tools installed for your platform

## Desktop build (optional)

```bash
cd apps/memst-ui
npm install
npm run tauri build     # uses src-tauri/, which has its own Cargo.lock/target
```

`src-tauri` is excluded from the root Cargo workspace (`exclude = ["apps"]`),
so its Tauri dependencies never enter the engine's lockfile.
