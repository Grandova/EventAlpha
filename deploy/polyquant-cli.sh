#!/usr/bin/env bash
# ==============================================================================
# PolyQuant 5M - Global Command Line Management Utility
# ==============================================================================

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

APP_DIR="/opt/polyquant"
CONFIG_FILE="$APP_DIR/config/config.yaml"
SERVICE_NAME="polyquant"
BIN_PATH="$APP_DIR/bin/poly_quant_backend"

function print_banner() {
    echo -e "${CYAN}${BOLD}"
    echo "============================================================"
    echo "   POLYQUANT 5M - QUANTITATIVE SIMULATION SYSTEM (LINUX)   "
    echo "============================================================"
    echo -e "${NC}"
}

function check_installed() {
    if [ ! -f "$BIN_PATH" ] && ! systemctl list-unit-files | grep -q "$SERVICE_NAME.service"; then
        echo -e "${RED}[ERROR] PolyQuant is not installed in $APP_DIR.${NC}"
        echo -e "${YELLOW}Please run the installer first: bash install.sh${NC}"
        exit 1
    fi
}

function get_status() {
    print_banner
    if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
        echo -e "Service Status:  ${GREEN}${BOLD}RUNNING (Active)${NC}"
        PID=$(systemctl show --property MainPID --value "$SERVICE_NAME" 2>/dev/null || echo "Unknown")
        echo -e "Main Process PID: ${CYAN}$PID${NC}"
    elif pgrep -f "poly_quant_backend" >/dev/null; then
        PID=$(pgrep -f "poly_quant_backend" | head -n 1)
        echo -e "Process Status:  ${GREEN}${BOLD}RUNNING (PID: $PID, Standalone)${NC}"
    else
        echo -e "Service Status:  ${RED}${BOLD}STOPPED (Inactive)${NC}"
    fi

    PORT=$(grep -E '^\s*port:' "$CONFIG_FILE" 2>/dev/null | awk '{print $2}' || echo "8080")
    HOST=$(grep -E '^\s*host:' "$CONFIG_FILE" 2>/dev/null | awk '{print $2}' | tr -d '"' || echo "0.0.0.0")

    echo -e "Listen Address:  ${YELLOW}http://$HOST:$PORT${NC}"
    echo -e "Health Endpoint: ${BLUE}http://127.0.0.1:$PORT/api/v1/health${NC}"
    echo -e "WebSocket Feed:  ${BLUE}ws://127.0.0.1:$PORT/api/v1/ws${NC}"
    echo -e "Config Location: ${YELLOW}$CONFIG_FILE${NC}"
    echo -e "Data Directory:  ${YELLOW}$APP_DIR/data${NC}"
    echo ""

    # Check health endpoint if curl is available
    if command -v curl >/dev/null 2>&1; then
        HEALTH_RES=$(curl -s --connect-timeout 2 "http://127.0.0.1:$PORT/api/v1/health" || true)
        if [ -n "$HEALTH_RES" ]; then
            echo -e "${GREEN}✓ Backend Health Check: OK${NC}"
            echo -e "  $HEALTH_RES"
        else
            echo -e "${YELLOW}⚠ Service is running but HTTP endpoint not responding yet (or starting up).${NC}"
        fi
    fi
}

function start_service() {
    print_banner
    echo -e "${YELLOW}Starting PolyQuant service...${NC}"
    if command -v systemctl >/dev/null 2>&1 && [ -f "/etc/systemd/system/$SERVICE_NAME.service" ]; then
        systemctl start "$SERVICE_NAME"
        sleep 1
        if systemctl is-active --quiet "$SERVICE_NAME"; then
            echo -e "${GREEN}${BOLD}✓ PolyQuant started successfully via systemd!${NC}"
        else
            echo -e "${RED}✗ Failed to start service. Check logs: polyquant log${NC}"
            exit 1
        fi
    else
        nohup "$BIN_PATH" > "$APP_DIR/logs/polyquant.log" 2>&1 &
        echo -e "${GREEN}${BOLD}✓ PolyQuant started in background (PID: $!).${NC}"
    fi
}

function stop_service() {
    print_banner
    echo -e "${YELLOW}Stopping PolyQuant service...${NC}"
    if command -v systemctl >/dev/null 2>&1 && [ -f "/etc/systemd/system/$SERVICE_NAME.service" ]; then
        systemctl stop "$SERVICE_NAME"
    fi
    pkill -f "poly_quant_backend" 2>/dev/null || true
    echo -e "${GREEN}✓ PolyQuant service stopped.${NC}"
}

function restart_service() {
    print_banner
    echo -e "${YELLOW}Restarting PolyQuant service...${NC}"
    if command -v systemctl >/dev/null 2>&1 && [ -f "/etc/systemd/system/$SERVICE_NAME.service" ]; then
        systemctl restart "$SERVICE_NAME"
        sleep 1
        if systemctl is-active --quiet "$SERVICE_NAME"; then
            echo -e "${GREEN}${BOLD}✓ PolyQuant restarted successfully!${NC}"
        else
            echo -e "${RED}✗ Failed to restart service. Check logs: polyquant log${NC}"
            exit 1
        fi
    else
        stop_service
        start_service
    fi
}

function show_log() {
    if command -v journalctl >/dev/null 2>&1 && [ -f "/etc/systemd/system/$SERVICE_NAME.service" ]; then
        journalctl -u "$SERVICE_NAME" -f -n "${1:-100}"
    elif [ -f "$APP_DIR/logs/polyquant.log" ]; then
        tail -f -n "${1:-100}" "$APP_DIR/logs/polyquant.log"
    else
        echo -e "${RED}No log file found.${NC}"
    fi
}

function update_app() {
    print_banner
    echo -e "${YELLOW}Updating PolyQuant to latest version...${NC}"
    SRC_DIR=$(cat "$APP_DIR/.source_dir" 2>/dev/null || echo "")

    if [ -z "$SRC_DIR" ] || [ ! -d "$SRC_DIR" ]; then
        echo -e "${RED}Source directory not found. Please pull git repo and run bash install.sh again.${NC}"
        exit 1
    fi

    cd "$SRC_DIR"
    echo -e "${CYAN}Pulling latest git changes...${NC}"
    git pull || true

    echo -e "${CYAN}Rebuilding Frontend...${NC}"
    cd "$SRC_DIR/frontend"
    npm run build

    echo -e "${CYAN}Rebuilding Backend...${NC}"
    cd "$SRC_DIR/backend"
    cargo build --release

    echo -e "${CYAN}Deploying binaries and assets...${NC}"
    NEW_BIN=""
    for CANDIDATE in \
        "$SRC_DIR/target/release/poly_quant_backend" \
        "$SRC_DIR/target/release/poly-quant-backend" \
        "$SRC_DIR/backend/target/release/poly_quant_backend" \
        "$SRC_DIR/backend/target/release/poly-quant-backend"
    do
        if [ -f "$CANDIDATE" ]; then
            NEW_BIN="$CANDIDATE"
            break
        fi
    done

    if [ -z "$NEW_BIN" ]; then
        NEW_BIN=$(find "$SRC_DIR/target/release" "$SRC_DIR/backend/target/release" -maxdepth 1 -type f \( -name "poly_quant_backend" -o -name "poly-quant-backend" \) 2>/dev/null | head -n 1 || true)
    fi

    if [ -n "$NEW_BIN" ]; then
        cp "$NEW_BIN" "$BIN_PATH"
        chmod +x "$BIN_PATH"
        ln -sf "$BIN_PATH" "$APP_DIR/bin/poly-quant-backend"
    fi
    rm -rf "$APP_DIR/frontend/dist"
    cp -r "$SRC_DIR/frontend/dist" "$APP_DIR/frontend/dist"

    restart_service
    echo -e "${GREEN}${BOLD}✓ Update and reload completed!${NC}"
}

function edit_config() {
    EDITOR=${EDITOR:-nano}
    if ! command -v "$EDITOR" >/dev/null 2>&1; then
        EDITOR=vim
    fi
    "$EDITOR" "$CONFIG_FILE"
    echo -e "${YELLOW}Config file modified. Restart service to apply: polyquant restart${NC}"
}

case "$1" in
    start)
        check_installed
        start_service
        ;;
    stop)
        stop_service
        ;;
    restart)
        check_installed
        restart_service
        ;;
    status)
        check_installed
        get_status
        ;;
    log|logs)
        show_log "$2"
        ;;
    update)
        check_installed
        update_app
        ;;
    config)
        edit_config
        ;;
    *)
        print_banner
        echo "Usage: polyquant {start|stop|restart|status|log|update|config}"
        echo ""
        echo "Commands:"
        echo "  status   - View service status, port, health check, and runtime info"
        echo "  start    - Start PolyQuant service"
        echo "  stop     - Stop PolyQuant service"
        echo "  restart  - Restart PolyQuant service"
        echo "  log      - Tail real-time service logs"
        echo "  update   - Pull latest git version, recompile, and hot-restart"
        echo "  config   - Edit system configuration file"
        echo ""
        exit 1
        ;;
esac
