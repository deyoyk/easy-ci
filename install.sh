#!/usr/bin/env bash
set -euo pipefail

# ============================================================================
# easy-ci installer
# Detects OS/arch, downloads the correct binary, installs it system-wide,
# and optionally sets up a systemd user service for background operation.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/install.sh | bash -s -- --help
#
# Options:
#   --help              Show this help message
#   --no-service        Skip systemd service setup
#   --version VERSION   Install a specific version (default: latest)
#   --prefix DIR        Install prefix (default: /usr/local)
# ============================================================================

REPO="deyoyk/easy-ci"
BINARY_NAME="eci"
INSTALL_PREFIX="/usr/local"
SETUP_SERVICE=true
VERSION=""

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
error()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }
die()     { error "$*"; exit 1; }

# ============================================================================
# Parse arguments
# ============================================================================
show_help() {
    cat <<EOF
easy-ci installer — https://github.com/deyoyk/easy-ci

Usage:
  curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/install.sh | bash -s -- [OPTIONS]

Options:
  --help              Show this help message
  --no-service        Skip systemd/launchd service setup
  --version VERSION   Install a specific version (default: latest)
  --prefix DIR        Install prefix (default: /usr/local)

Examples:
  # Install latest version
  curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/install.sh | bash

  # Install specific version
  curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/install.sh | bash -s -- --version 0.1.42

  # Install without service setup
  curl -fsSL https://raw.githubusercontent.com/deyoyk/easy-ci/main/install.sh | bash -s -- --no-service
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --help|-h)
            show_help
            ;;
        --no-service)
            SETUP_SERVICE=false
            shift
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --prefix)
            INSTALL_PREFIX="$2"
            shift 2
            ;;
        *)
            die "Unknown option: $1"
            ;;
    esac
done

# ============================================================================
# Detect OS and architecture
# ============================================================================
detect_os() {
    local os
    os="$(uname -s)"
    case "$os" in
        Linux*)     echo "linux" ;;
        Darwin*)    echo "darwin" ;;
        MINGW*|MSYS*|CYGWIN*)  echo "windows" ;;
        *)          die "Unsupported OS: $os" ;;
    esac
}

detect_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64)   echo "x86_64" ;;
        aarch64|arm64)   echo "aarch64" ;;
        *)               die "Unsupported architecture: $arch" ;;
    esac
}

OS="$(detect_os)"
ARCH="$(detect_arch)"

info "Detected platform: ${BOLD}${OS}/${ARCH}${NC}"

# ============================================================================
# Check dependencies
# ============================================================================
check_deps() {
    local missing=()

    if ! command -v curl &>/dev/null && ! command -v wget &>/dev/null; then
        missing+=("curl or wget")
    fi

    if ! command -v tar &>/dev/null; then
        missing+=("tar")
    fi

    if [[ "$OS" == "linux" ]] && [[ "$SETUP_SERVICE" == "true" ]]; then
        if ! command -v systemctl &>/dev/null; then
            warn "systemctl not found — will skip service setup"
            SETUP_SERVICE=false
        fi
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        die "Missing required dependencies: ${missing[*]}\n\nInstall them and try again."
    fi
}

check_deps

# ============================================================================
# Determine version
# ============================================================================
get_latest_version() {
    info "Fetching latest version..."
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    local version

    if command -v curl &>/dev/null; then
        version=$(curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed -E 's/.*"v([^"]+)".*/\1/')
    else
        version=$(wget -qO- "$url" | grep '"tag_name"' | head -1 | sed -E 's/.*"v([^"]+)".*/\1/')
    fi

    if [[ -z "$version" ]]; then
        die "Failed to fetch latest version from GitHub"
    fi

    echo "$version"
}

if [[ -z "$VERSION" ]]; then
    VERSION="$(get_latest_version)"
fi

info "Installing version: ${BOLD}${VERSION}${NC}"

# ============================================================================
# Download binary
# ============================================================================
download_binary() {
    local version="$1"
    local os="$2"
    local arch="$3"

    local archive_name="eci-${os}-${arch}.tar.gz"
    local url="https://github.com/${REPO}/releases/download/v${version}/${archive_name}"

    local tmp_dir
    tmp_dir="$(mktemp -d)"
    local archive_path="${tmp_dir}/${archive_name}"

    info "Downloading ${archive_name}..."
    if command -v curl &>/dev/null; then
        curl -fsSL -o "$archive_path" "$url" || die "Failed to download from $url"
    else
        wget -qO "$archive_path" "$url" || die "Failed to download from $url"
    fi

    info "Extracting..."
    tar xzf "$archive_path" -C "$tmp_dir" || die "Failed to extract archive"

    local binary_path="${tmp_dir}/eci"
    if [[ ! -f "$binary_path" ]]; then
        # Try the artifact name pattern
        binary_path="${tmp_dir}/eci-${os}-${arch}"
    fi

    if [[ ! -f "$binary_path" ]]; then
        # Find any eci binary in the tmp dir
        binary_path="$(find "$tmp_dir" -maxdepth 1 -name 'eci*' -type f | head -1)"
    fi

    if [[ -z "$binary_path" ]] || [[ ! -f "$binary_path" ]]; then
        die "Could not find eci binary in downloaded archive"
    fi

    chmod +x "$binary_path"
    echo "$binary_path"
}

BINARY_PATH="$(download_binary "$VERSION" "$OS" "$ARCH")"

# ============================================================================
# Install binary
# ============================================================================
install_binary() {
    local binary_path="$1"
    local install_dir="${INSTALL_PREFIX}/bin"
    local dest="${install_dir}/${BINARY_NAME}"

    info "Installing to ${dest}..."

    # Create install directory if it doesn't exist
    if [[ ! -d "$install_dir" ]]; then
        mkdir -p "$install_dir" || die "Failed to create ${install_dir} (try with sudo)"
    fi

    # Try installing without sudo first, fall back to sudo
    if [[ -w "$install_dir" ]]; then
        cp "$binary_path" "$dest"
    else
        info "Need elevated permissions to install to ${install_dir}"
        sudo cp "$binary_path" "$dest"
    fi

    chmod +x "$dest"
    success "Binary installed to ${dest}"
}

install_binary "$BINARY_PATH"

# ============================================================================
# Verify installation
# ============================================================================
info "Verifying installation..."
if command -v "$BINARY_NAME" &>/dev/null; then
    INSTALLED_VERSION=$("$BINARY_NAME" --version 2>/dev/null || echo "unknown")
    success "Installed successfully: ${BOLD}${INSTALLED_VERSION}${NC}"
else
    warn "Binary installed but not found in PATH"
    warn "You may need to add ${INSTALL_PREFIX}/bin to your PATH"
fi

# ============================================================================
# Setup systemd service (Linux)
# ============================================================================
setup_systemd_service() {
    local service_dir="${HOME}/.config/systemd/user"
    local service_file="${service_dir}/${BINARY_NAME}.service"

    info "Setting up systemd user service..."

    mkdir -p "$service_dir"

    cat > "$service_file" <<EOF
[Unit]
Description=easy-ci deployment service
After=network-online.target docker.service
Wants=network-online.target

[Service]
Type=simple
ExecStart=${INSTALL_PREFIX}/bin/${BINARY_NAME} dashboard
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF

    # Enable and start the service
    systemctl --user daemon-reload
    systemctl --user enable "${BINARY_NAME}.service"

    success "Systemd service created and enabled"
    info "Start with: systemctl --user start ${BINARY_NAME}"
    info "Stop with:  systemctl --user stop ${BINARY_NAME}"
    info "View logs:  journalctl --user -u ${BINARY_NAME} -f"
}

# ============================================================================
# Setup launchd service (macOS)
# ============================================================================
setup_launchd_service() {
    local service_dir="${HOME}/Library/LaunchAgents"
    local service_file="${service_dir}/com.deyoyk.${BINARY_NAME}.plist"

    info "Setting up launchd agent..."

    mkdir -p "$service_dir"

    cat > "$service_file" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.deyoyk.${BINARY_NAME}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${INSTALL_PREFIX}/bin/${BINARY_NAME}</string>
        <string>dashboard</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${HOME}/.eci/logs/service.log</string>
    <key>StandardErrorPath</key>
    <string>${HOME}/.eci/logs/service.err</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>info</string>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin</string>
    </dict>
</dict>
</plist>
EOF

    mkdir -p "${HOME}/.eci/logs"

    # Load the service
    launchctl unload "$service_file" 2>/dev/null || true
    launchctl load "$service_file"

    success "LaunchAgent created and loaded"
    info "Start with: launchctl start com.deyoyk.${BINARY_NAME}"
    info "Stop with:  launchctl stop com.deyoyk.${BINARY_NAME}"
    info "View logs:  tail -f ${HOME}/.eci/logs/service.log"
}

# ============================================================================
# Setup service based on OS
# ============================================================================
if [[ "$SETUP_SERVICE" == "true" ]]; then
    case "$OS" in
        linux)
            setup_systemd_service
            ;;
        darwin)
            setup_launchd_service
            ;;
        windows)
            warn "Windows service setup not supported yet"
            warn "You can run '${BINARY_NAME} dashboard' manually"
            ;;
    esac
fi

# ============================================================================
# Print post-install instructions
# ============================================================================
echo ""
echo -e "${GREEN}${BOLD}========================================${NC}"
echo -e "${GREEN}${BOLD}  easy-ci installed successfully!${NC}"
echo -e "${GREEN}${BOLD}========================================${NC}"
echo ""
echo -e "  Get started:"
echo -e "    ${BOLD}${BINARY_NAME} --help${NC}          Show all commands"
echo -e "    ${BOLD}${BINARY_NAME} init${NC}            Configure GitHub token & Docker"
echo -e "    ${BOLD}${BINARY_NAME} deploy <repo>${NC}   Deploy a GitHub repo"
echo -e "    ${BOLD}${BINARY_NAME} dashboard${NC}       Launch the TUI dashboard"
echo ""
if [[ "$SETUP_SERVICE" == "true" ]]; then
    case "$OS" in
        linux)
            echo -e "  Service management:"
            echo -e "    ${BOLD}systemctl --user start ${BINARY_NAME}${NC}"
            echo -e "    ${BOLD}systemctl --user stop ${BINARY_NAME}${NC}"
            echo -e "    ${BOLD}systemctl --user status ${BINARY_NAME}${NC}"
            ;;
        darwin)
            echo -e "  Service management:"
            echo -e "    ${BOLD}launchctl start com.deyoyk.${BINARY_NAME}${NC}"
            echo -e "    ${BOLD}launchctl stop com.deyoyk.${BINARY_NAME}${NC}"
            ;;
    esac
fi
echo ""
echo -e "  Docs: ${BLUE}https://github.com/deyoyk/easy-ci${NC}"
echo ""
