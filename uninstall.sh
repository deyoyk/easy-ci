#!/usr/bin/env bash
set -euo pipefail

# ============================================================================
# easy-ci uninstaller
# Removes the binary, service, and configuration files.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/uninstall.sh | bash
# ============================================================================

BINARY_NAME="eci"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

info()    { echo -e "${BLUE}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[OK]${NC} $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; }

# ============================================================================
# Detect OS
# ============================================================================
detect_os() {
    local os
    os="$(uname -s)"
    case "$os" in
        Linux*)     echo "linux" ;;
        Darwin*)    echo "darwin" ;;
        MINGW*|MSYS*|CYGWIN*)  echo "windows" ;;
        *)          echo "unknown" ;;
    esac
}

OS="$(detect_os)"

echo -e "${BOLD}Uninstalling easy-ci...${NC}"
echo ""

# ============================================================================
# Stop and remove service
# ============================================================================
stop_service() {
    case "$OS" in
        linux)
            if systemctl --user is-active "${BINARY_NAME}.service" &>/dev/null; then
                info "Stopping systemd service..."
                systemctl --user stop "${BINARY_NAME}.service"
            fi
            if systemctl --user is-enabled "${BINARY_NAME}.service" &>/dev/null; then
                info "Disabling systemd service..."
                systemctl --user disable "${BINARY_NAME}.service"
            fi
            local service_file="${HOME}/.config/systemd/user/${BINARY_NAME}.service"
            if [[ -f "$service_file" ]]; then
                info "Removing service file..."
                rm -f "$service_file"
                systemctl --user daemon-reload
            fi
            ;;
        darwin)
            local service_label="com.deyoyk.${BINARY_NAME}"
            local service_file="${HOME}/Library/LaunchAgents/${service_label}.plist"
            if launchctl list | grep -q "$service_label" 2>/dev/null; then
                info "Stopping launchd agent..."
                launchctl stop "$service_label" 2>/dev/null || true
                launchctl unload "$service_file" 2>/dev/null || true
            fi
            if [[ -f "$service_file" ]]; then
                info "Removing LaunchAgent..."
                rm -f "$service_file"
            fi
            ;;
    esac
}

stop_service

# ============================================================================
# Remove binary
# ============================================================================
remove_binary() {
    local binary_paths=(
        "/usr/local/bin/${BINARY_NAME}"
        "/usr/bin/${BINARY_NAME}"
        "${HOME}/.local/bin/${BINARY_NAME}"
    )

    for binary_path in "${binary_paths[@]}"; do
        if [[ -f "$binary_path" ]]; then
            info "Removing ${binary_path}..."
            if [[ -w "$(dirname "$binary_path")" ]]; then
                rm -f "$binary_path"
            else
                sudo rm -f "$binary_path"
            fi
            success "Removed ${binary_path}"
        fi
    done
}

remove_binary

# ============================================================================
# Remove configuration (optional)
# ============================================================================
remove_config() {
    local config_dir="${HOME}/.eci"

    if [[ -d "$config_dir" ]]; then
        echo ""
        warn "Configuration directory found at ${config_dir}"
        read -rp "Remove configuration? (y/N): " confirm
        if [[ "$confirm" =~ ^[Yy]$ ]]; then
            info "Removing configuration..."
            rm -rf "$config_dir"
            success "Configuration removed"
        else
            info "Keeping configuration at ${config_dir}"
        fi
    fi
}

remove_config

# ============================================================================
# Remove logs
# ============================================================================
remove_logs() {
    local log_dir="${HOME}/.eci/logs"
    if [[ -d "$log_dir" ]]; then
        info "Removing logs..."
        rm -rf "$log_dir"
    fi
}

remove_logs

# ============================================================================
# Done
# ============================================================================
echo ""
echo -e "${GREEN}${BOLD}easy-ci uninstalled successfully!${NC}"
echo ""
