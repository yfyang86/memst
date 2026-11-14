#!/bin/bash

# MemSt Server Management Script
# Usage: ./server.sh [OPTIONS]
#   --start         Start the backend server
#   --stop          Stop the backend server
#   --maintain      Check config, status, logs, etc.
#   --help          Show this help message

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVER_DIR="$SCRIPT_DIR/memst-server"
UI_DIR="$SCRIPT_DIR/memst-ui"
PID_FILE="$SERVER_DIR/server.pid"
LOG_FILE="$SERVER_DIR/server.log"
CONFIG_FILE="$SERVER_DIR/config.toml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored message
print_msg() {
    local color=$1
    local msg=$2
    echo -e "${color}${msg}${NC}"
}

# Print usage
usage() {
    cat << EOF
MemSt Server Management Script

Usage: $0 [OPTIONS]

OPTIONS:
    --start         Start the backend server
    --stop          Stop the backend server
    --restart       Restart the backend server
    --maintain      Check config, status, logs, etc.
    --status        Show server status
    --logs          Show recent server logs
    --help          Show this help message

EXAMPLES:
    $0 --start              # Start the server
    $0 --stop               # Stop the server
    $0 --maintain           # Run maintenance checks
    $0 --logs               # View server logs
EOF
}

# Check if server is running
is_server_running() {
    if [ -f "$PID_FILE" ]; then
        local pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            return 0
        else
            # Stale PID file
            rm -f "$PID_FILE"
            return 1
        fi
    fi
    return 1
}

# Start the server
start_server() {
    print_msg "$BLUE" "Starting MemSt server..."
    
    # Check if already running
    if is_server_running; then
        print_msg "$YELLOW" "Server is already running (PID: $(cat $PID_FILE))"
        return 1
    fi
    
    # Check if venv exists
    if [ ! -d "$SERVER_DIR/.venv" ]; then
        print_msg "$RED" "Virtual environment not found at $SERVER_DIR/.venv"
        print_msg "$YELLOW" "Please run: cd $SERVER_DIR && uv venv --python 3.12"
        return 1
    fi
    
    # Check if config.toml exists
    if [ ! -f "$CONFIG_FILE" ]; then
        print_msg "$YELLOW" "Warning: config.toml not found at $CONFIG_FILE"
        print_msg "$YELLOW" "Using default configuration..."
    fi
    
    # Change to server directory and start
    cd "$SERVER_DIR"
    
    # Start the server in background
    PYTHONPATH=src .venv/bin/python -m memst_server.main >> "$LOG_FILE" 2>&1 &
    local pid=$!
    
    # Save PID
    echo "$pid" > "$PID_FILE"
    
    # Wait a moment and check if started
    sleep 2
    if kill -0 "$pid" 2>/dev/null; then
        print_msg "$GREEN" "Server started successfully (PID: $pid)"
        print_msg "$GREEN" "Server running at: http://127.0.0.1:$(get_server_port)"
        print_msg "$BLUE" "Logs: $LOG_FILE"
    else
        print_msg "$RED" "Failed to start server"
        print_msg "$RED" "Check logs: tail $LOG_FILE"
        rm -f "$PID_FILE"
        return 1
    fi
}

# Stop the server
stop_server() {
    print_msg "$BLUE" "Stopping MemSt server..."
    
    if [ ! -f "$PID_FILE" ]; then
        print_msg "$YELLOW" "No PID file found. Server may not be running."
        # Try to find and kill by process name
        local pids=$(pgrep -f "memst_server.main")
        if [ -n "$pids" ]; then
            print_msg "$YELLOW" "Found running server processes: $pids"
            echo "$pids" | xargs kill 2>/dev/null || true
            print_msg "$GREEN" "Server stopped"
        else
            print_msg "$YELLOW" "No running server found"
        fi
        return 1
    fi
    
    local pid=$(cat "$PID_FILE")
    
    if kill -0 "$pid" 2>/dev/null; then
        kill "$pid"
        sleep 1
        
        # Force kill if still running
        if kill -0 "$pid" 2>/dev/null; then
            kill -9 "$pid" 2>/dev/null || true
            sleep 1
        fi
        
        print_msg "$GREEN" "Server stopped (PID: $pid)"
    else
        print_msg "$YELLOW" "Server not running (stale PID file)"
    fi
    
    # Clean up PID file
    rm -f "$PID_FILE"
}

# Get server port from config
get_server_port() {
    if [ -f "$CONFIG_FILE" ]; then
        grep "^port" "$CONFIG_FILE" | sed 's/.*= *//' | tr -d ' '
    else
        echo "8193"
    fi
}

# Check server status
check_status() {
    print_msg "$BLUE" "=== MemSt Server Status ==="
    
    # Check if running
    if is_server_running; then
        local pid=$(cat "$PID_FILE")
        print_msg "$GREEN" "Server: RUNNING (PID: $pid)"
        print_msg "$GREEN" "URL: http://127.0.0.1:$(get_server_port)"
    else
        print_msg "$RED" "Server: NOT RUNNING"
    fi
    
    # Check config file
    echo ""
    print_msg "$BLUE" "=== Configuration ==="
    if [ -f "$CONFIG_FILE" ]; then
        print_msg "$GREEN" "Config file: $CONFIG_FILE"
        echo "Port: $(get_server_port)"
    else
        print_msg "$YELLOW" "Config file: NOT FOUND"
    fi
    
    # Check venv
    echo ""
    print_msg "$BLUE" "=== Environment ==="
    if [ -d "$SERVER_DIR/.venv" ]; then
        print_msg "$GREEN" "Virtual env: EXISTS"
    else
        print_msg "$RED" "Virtual env: NOT FOUND"
    fi
    
    # Check memst module
    if [ -d "$SERVER_DIR/.venv" ]; then
        cd "$SERVER_DIR"
        if PYTHONPATH=src .venv/bin/python -c "import memst" 2>/dev/null; then
            print_msg "$GREEN" "memst module: AVAILABLE"
        else
            print_msg "$RED" "memst module: NOT AVAILABLE"
        fi
    fi
    
    # Check log file
    echo ""
    print_msg "$BLUE" "=== Log File ==="
    if [ -f "$LOG_FILE" ]; then
        print_msg "$GREEN" "Log file: $LOG_FILE"
        echo "Size: $(du -h "$LOG_FILE" | cut -f1)"
    else
        print_msg "$YELLOW" "Log file: NOT FOUND"
    fi
}

# Check configuration for errors
check_config() {
    print_msg "$BLUE" "=== Configuration Check ==="
    
    local errors=0
    
    # Check config file exists
    if [ ! -f "$CONFIG_FILE" ]; then
        print_msg "$RED" "Error: config.toml not found at $CONFIG_FILE"
        errors=$((errors + 1))
    else
        print_msg "$GREEN" "Config file: OK"
        
        # Check port
        local port=$(get_server_port)
        if [ -z "$port" ]; then
            print_msg "$RED" "Error: Port not configured"
            errors=$((errors + 1))
        else
            print_msg "$GREEN" "Port: $port"
            
            # Check if port is in use
            if lsof -i :"$port" >/dev/null 2>&1; then
                print_msg "$YELLOW" "Warning: Port $port is already in use"
            else
                print_msg "$GREEN" "Port $port: Available"
            fi
        fi
        
        # Check CORS
        if grep -q "cors_origins" "$CONFIG_FILE"; then
            local cors=$(grep "cors_origins" "$CONFIG_FILE" | head -1)
            print_msg "$GREEN" "CORS: Configured ($cors)"
        else
            print_msg "$YELLOW" "Warning: CORS not configured - frontend may be blocked"
        fi
        
        # Check store_path
        if grep -q "store_path" "$CONFIG_FILE"; then
            local store_path=$(grep "store_path" "$CONFIG_FILE" | sed 's/.*= *//' | tr -d '"')
            if [ -d "$store_path" ]; then
                print_msg "$GREEN" "Store path: $store_path (exists)"
            else
                print_msg "$YELLOW" "Store path: $store_path (will be created)"
            fi
        fi
    fi
    
    # Check venv
    if [ ! -d "$SERVER_DIR/.venv" ]; then
        print_msg "$RED" "Error: Virtual environment not found"
        print_msg "$YELLOW" "Run: cd $SERVER_DIR && uv venv --python 3.12"
        errors=$((errors + 1))
    else
        print_msg "$GREEN" "Virtual env: OK"
    fi
    
    # Check PYTHONPATH requirement
    print_msg "$BLUE" "=== Runtime Requirements ==="
    print_msg "$GREEN" "PYTHONPATH=src is required when running the server"
    
    # Summary
    echo ""
    if [ $errors -eq 0 ]; then
        print_msg "$GREEN" "Configuration check: PASSED"
        return 0
    else
        print_msg "$RED" "Configuration check: FAILED ($errors error(s))"
        return 1
    fi
}

# Show logs
show_logs() {
    if [ -f "$LOG_FILE" ]; then
        print_msg "$BLUE" "=== Recent Logs (last 50 lines) ==="
        tail -50 "$LOG_FILE"
    else
        print_msg "$YELLOW" "No log file found at $LOG_FILE"
    fi
}

# Maintenance function
maintain() {
    print_msg "$BLUE" "=== MemSt Maintenance ==="
    echo ""
    
    # Run config check
    check_config
    echo ""
    
    # Show status
    check_status
    echo ""
    
    # Show recent logs if running
    if is_server_running; then
        echo ""
        print_msg "$BLUE" "=== Recent Logs ==="
        tail -20 "$LOG_FILE"
    fi
}

# Main
case "${1:-}" in
    --start)
        start_server
        ;;
    --stop)
        stop_server
        ;;
    --restart)
        stop_server
        sleep 1
        start_server
        ;;
    --maintain)
        maintain
        ;;
    --status)
        check_status
        ;;
    --logs)
        show_logs
        ;;
    --check)
        check_config
        ;;
    --help|-h)
        usage
        ;;
    *)
        usage
        exit 1
        ;;
esac
