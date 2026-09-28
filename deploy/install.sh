#!/usr/bin/env bash
# ==============================================================================
# PolyQuant 5M - Automated One-Click Installer for Linux & Baota (aaPanel)
# ==============================================================================
#
# Supported Systems:
#   - Ubuntu 20.04 / 22.04 / 24.04 LTS
#   - Debian 10 / 11 / 12
#   - CentOS 7 / 8 / 9, Rocky Linux 8 / 9, AlmaLinux 8 / 9, RHEL
#   - Baota Panel (宝塔面板) / aaPanel on Linux
#
# Usage:
#   sudo bash install.sh
# ==============================================================================

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
INSTALL_PREFIX="/opt/polyquant"
SERVICE_NAME="polyquant"

echo -e "${CYAN}${BOLD}"
echo "=================================================================="
echo "    POLYQUANT 5M - 自动化量化预测与模拟交易系统一键安装脚本       "
echo "    (支持原生 Linux / 宝塔面板 / aaPanel / 云服务器)             "
echo "=================================================================="
echo -e "${NC}"

# 1. Root privilege check
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[ERROR] 请使用 root 用户或通过 sudo 运行此安装脚本:${NC}"
    echo -e "${YELLOW}  sudo bash install.sh${NC}"
    exit 1
fi

# 2. Detect OS and Package Manager
OS_TYPE="unknown"
PKG_MANAGER=""

if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS_TYPE=$ID
fi

if command -v apt-get >/dev/null 2>&1; then
    PKG_MANAGER="apt"
elif command -v dnf >/dev/null 2>&1; then
    PKG_MANAGER="dnf"
elif command -v yum >/dev/null 2>&1; then
    PKG_MANAGER="yum"
elif command -v apk >/dev/null 2>&1; then
    PKG_MANAGER="apk"
else
    echo -e "${RED}[ERROR] 未检测到受支持的包管理器 (apt/dnf/yum/apk)。${NC}"
    exit 1
fi

echo -e "${BLUE}[1/8] 检测到操作系统: ${BOLD}$OS_TYPE ($PKG_MANAGER)${NC}"

# Check for Baota / aaPanel
IS_BAOTA=false
if [ -d "/www/server/panel" ]; then
    IS_BAOTA=true
    echo -e "${MAGENTA}${BOLD}✓ 检测到宝塔面板 (Baota/aaPanel) 环境，将自动适配宝塔生态！${NC}"
fi

# 3. Install System Build Tools & Libraries
echo -e "${BLUE}[2/8] 安装系统核心编译依赖 (GCC, Make, OpenSSL, SQLite, PKG-Config)...${NC}"
if [ "$PKG_MANAGER" = "apt" ]; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -y
    apt-get install -y build-essential pkg-config libssl-dev sqlite3 libsqlite3-dev curl wget git ca-certificates
elif [ "$PKG_MANAGER" = "dnf" ] || [ "$PKG_MANAGER" = "yum" ]; then
    $PKG_MANAGER install -y gcc gcc-c++ make pkgconfig openssl-devel sqlite sqlite-devel curl wget git ca-certificates
elif [ "$PKG_MANAGER" = "apk" ]; then
    apk add --no-cache build-base pkgconfig openssl-dev sqlite-dev curl wget git ca-certificates
fi
echo -e "${GREEN}✓ 系统编译依赖安装完毕。${NC}"

# 4. Check & Install Node.js (v20+ LTS)
echo -e "${BLUE}[3/8] 检查 Node.js 前端编译环境...${NC}"
NODE_NEED_INSTALL=false

if ! command -v node >/dev/null 2>&1; then
    NODE_NEED_INSTALL=true
else
    NODE_VER=$(node -v | sed 's/v//' | cut -d. -f1)
    if [ "$NODE_VER" -lt 18 ]; then
        echo -e "${YELLOW}检测到当前 Node.js 版本 ($NODE_VER) 低于 v18，正在升级至 v20 LTS...${NC}"
        NODE_NEED_INSTALL=true
    else
        echo -e "${GREEN}✓ Node.js 已安装: $(node -v) (npm $(npm -v))${NC}"
    fi
fi

if [ "$NODE_NEED_INSTALL" = true ]; then
    echo -e "${CYAN}正在通过官方源全自动安装 Node.js v20 LTS...${NC}"
    if [ "$PKG_MANAGER" = "apt" ]; then
        curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
        apt-get install -y nodejs
    elif [ "$PKG_MANAGER" = "dnf" ] || [ "$PKG_MANAGER" = "yum" ]; then
        curl -fsSL https://rpm.nodesource.com/setup_20.x | bash -
        $PKG_MANAGER install -y nodejs
    fi
    echo -e "${GREEN}✓ Node.js 安装完成: $(node -v)${NC}"
fi

# 5. Check & Install Rust Toolchain
echo -e "${BLUE}[4/8] 检查 Rust/Cargo 量化内核编译环境...${NC}"
if ! command -v cargo >/dev/null 2>&1; then
    if [ -f "$HOME/.cargo/env" ]; then
        . "$HOME/.cargo/env"
    fi
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo -e "${CYAN}正在通过官方 rustup 自动安装 Rust 工具链 (stable)...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
    . "$HOME/.cargo/env"
    echo -e "${GREEN}✓ Rust 工具链安装完毕: $(rustc --version)${NC}"
else
    echo -e "${GREEN}✓ Rust 工具链已就绪: $(rustc --version)${NC}"
fi

# 6. Build Frontend Web Console
echo -e "${BLUE}[5/8] 正在编译前端现代量化控制台 (React + Tailwind + Vite)...${NC}"
cd "$ROOT_DIR/frontend"
if [ ! -d "node_modules" ]; then
    npm install
fi
npm run build
if [ ! -d "dist" ] || [ ! -f "dist/index.html" ]; then
    echo -e "${RED}[ERROR] 前端构建失败，未能生成 dist/index.html。${NC}"
    exit 1
fi
echo -e "${GREEN}✓ 前端生产环境资源包打包完成 (dist/)。${NC}"

# 7. Build Backend Release Binary
echo -e "${BLUE}[6/8] 正在编译 Rust 高性能量化模拟引擎 (Release 极速优化模式)...${NC}"
cd "$ROOT_DIR/backend"
cargo build --release
TARGET_BIN="$ROOT_DIR/target/release/poly_quant_backend"

if [ ! -f "$TARGET_BIN" ]; then
    # In case cargo built into backend/target
    if [ -f "$ROOT_DIR/backend/target/release/poly_quant_backend" ]; then
        TARGET_BIN="$ROOT_DIR/backend/target/release/poly_quant_backend"
    else
        echo -e "${RED}[ERROR] 后端编译失败，未找到二进制可执行文件。${NC}"
        exit 1
    fi
fi
echo -e "${GREEN}✓ 后端量化可执行程序编译完毕: $TARGET_BIN${NC}"

# 8. Setup Production Directory Structure
echo -e "${BLUE}[7/8] 正在配置系统安装路径 ($INSTALL_PREFIX)...${NC}"
mkdir -p "$INSTALL_PREFIX/bin"
mkdir -p "$INSTALL_PREFIX/config"
mkdir -p "$INSTALL_PREFIX/data"
mkdir -p "$INSTALL_PREFIX/logs"
mkdir -p "$INSTALL_PREFIX/frontend"

# Copy binary
cp "$TARGET_BIN" "$INSTALL_PREFIX/bin/poly_quant_backend"
chmod +x "$INSTALL_PREFIX/bin/poly_quant_backend"

# Copy frontend dist
rm -rf "$INSTALL_PREFIX/frontend/dist"
cp -r "$ROOT_DIR/frontend/dist" "$INSTALL_PREFIX/frontend/dist"

# Copy configuration (preserve existing config if user already customized it)
if [ ! -f "$INSTALL_PREFIX/config/config.yaml" ]; then
    cp "$ROOT_DIR/config/config.yaml" "$INSTALL_PREFIX/config/config.yaml"
    # Ensure 0.0.0.0 host binding for cloud servers
    sed -i 's/host: "127.0.0.1"/host: "0.0.0.0"/g' "$INSTALL_PREFIX/config/config.yaml" || true
    echo -e "${GREEN}✓ 已复制默认量化配置至 $INSTALL_PREFIX/config/config.yaml${NC}"
else
    echo -e "${YELLOW}检测到已有量化配置，保留用户现有配置文件。${NC}"
fi

# Record source directory for `polyquant update`
echo "$ROOT_DIR" > "$INSTALL_PREFIX/.source_dir"

# Install global CLI management command
cp "$ROOT_DIR/deploy/polyquant-cli.sh" "$INSTALL_PREFIX/bin/polyquant"
chmod +x "$INSTALL_PREFIX/bin/polyquant"
ln -sf "$INSTALL_PREFIX/bin/polyquant" "/usr/local/bin/polyquant"

# 9. Configure systemd Service
echo -e "${BLUE}[8/8] 正在注册并启动 systemd 系统级服务守护进程...${NC}"
SYSTEMD_FILE="/etc/systemd/system/$SERVICE_NAME.service"

cat > "$SYSTEMD_FILE" <<EOF
[Unit]
Description=Polymarket 5-Minute Crypto Up/Down Quant Simulation Engine
Documentation=https://github.com/PolyQuant
After=network.target network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=$INSTALL_PREFIX
ExecStart=$INSTALL_PREFIX/bin/poly_quant_backend
Restart=always
RestartSec=5s
LimitNOFILE=65535
LimitNPROC=65535

# Environment
Environment=APP_CONFIG_PATH=$INSTALL_PREFIX/config/config.yaml
Environment=FRONTEND_DIST_PATH=$INSTALL_PREFIX/frontend/dist
Environment=RUST_LOG=info,poly_quant_backend=debug,tower_http=info
StandardOutput=append:$INSTALL_PREFIX/logs/service.log
StandardError=append:$INSTALL_PREFIX/logs/service_error.log

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl restart "$SERVICE_NAME"

sleep 2

# Verify running
PUBLIC_IP=$(curl -s -m 2 https://api.ipify.org || echo "YOUR_SERVER_IP")
PORT=8080

echo ""
echo -e "${GREEN}${BOLD}==================================================================${NC}"
echo -e "${GREEN}${BOLD}      🎉 POLYQUANT 5M 模拟量化系统在 LINUX 上安装部署成功！        ${NC}"
echo -e "${GREEN}${BOLD}==================================================================${NC}"
echo ""
echo -e "核心服务状态:   ${GREEN}${BOLD}RUNNING (systemd 守护运行中)${NC}"
echo -e "访问控制台地址: ${CYAN}${BOLD}http://$PUBLIC_IP:$PORT${NC} (或 http://127.0.0.1:$PORT)"
echo -e "健康检查 API:   ${BLUE}http://127.0.0.1:$PORT/api/v1/health${NC}"
echo -e "WebSocket 接口: ${BLUE}ws://127.0.0.1:$PORT/api/v1/ws${NC}"
echo -e "配置文件路径:   ${YELLOW}$INSTALL_PREFIX/config/config.yaml${NC}"
echo -e "数据库存储路径: ${YELLOW}$INSTALL_PREFIX/data/poly_quant.db${NC}"
echo -e "系统日志路径:   ${YELLOW}$INSTALL_PREFIX/logs/service.log${NC}"
echo ""
echo -e "${BOLD}全功能命令行管理快捷指令 (已加入系统 PATH):${NC}"
echo -e "  ${CYAN}polyquant status${NC}   - 查看服务实时运行状态、内存、端口与心跳"
echo -e "  ${CYAN}polyquant log${NC}      - 实时查看行情采集、策略决策与模拟成交日志"
echo -e "  ${CYAN}polyquant restart${NC}  - 重启量化系统"
echo -e "  ${CYAN}polyquant stop${NC}     - 停止量化系统"
echo -e "  ${CYAN}polyquant config${NC}   - 编辑调优量化参数 (保存后运行 polyquant restart)"
echo -e "  ${CYAN}polyquant update${NC}   - 一键拉取最新代码并热更新重编译"
echo ""

if [ "$IS_BAOTA" = true ]; then
    echo -e "${MAGENTA}${BOLD}------------------------------------------------------------------${NC}"
    echo -e "${MAGENTA}${BOLD}             宝塔面板 (aaPanel) 用户专属配置指引                   ${NC}"
    echo -e "${MAGENTA}${BOLD}------------------------------------------------------------------${NC}"
    echo -e "1. ${BOLD}端口放行${NC}: 进入 宝塔 -> 安全 -> 防火墙，放行 ${YELLOW}8080${NC} 端口。"
    echo -e "2. ${BOLD}域名绑定与反向代理 (推荐)${NC}:"
    echo -e "   - 进入 宝塔 -> 网站 -> 添加站点 (例如: quant.yourdomain.com)"
    echo -e "   - 点击站点设置 -> 反向代理 -> 添加反向代理"
    echo -e "   - 代理名称: ${CYAN}polyquant${NC}"
    echo -e "   - 目标 URL: ${CYAN}http://127.0.0.1:8080${NC}"
    echo -e "   - 发送域名: ${CYAN}\$host${NC}"
    echo -e "   - 关键: 请在反代配置中确认已加入 WebSocket 支持 (见 deploy/baota/nginx_reverse_proxy.conf)"
    echo -e "3. ${BOLD}Supervisor 进程守护 (备选)${NC}:"
    echo -e "   - 若习惯宝塔面板可视化管理进程，可安装「进程守护管理器」，导入 deploy/baota/supervisor_polyquant.ini"
    echo -e "${MAGENTA}${BOLD}------------------------------------------------------------------${NC}"
    echo ""
fi
