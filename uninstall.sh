#!/usr/bin/env bash
set -euo pipefail

# ============================================================================
# easy-ci uninstaller (Linux)
# Removes the binary, systemd service, and configuration files.
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

echo -e "${BOLD}Uninstalling easy-ci...${NC}"
echo ""

# ============================================================================
# Stop and remove systemd service
# ============================================================================
stop_service() {
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
# Done
# ============================================================================
echo ""
echo -e "${GREEN}${BOLD}easy-ci uninstalled successfully!${NC}"
echo ""
