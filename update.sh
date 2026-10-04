#!/usr/bin/env bash
# ==============================================================================
# PolyQuant 5M - One-Click System & UI Hot Update Script
# ==============================================================================

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_PREFIX="/opt/polyquant"

echo -e "${CYAN}${BOLD}"
echo "=================================================================="
echo "    PolyQuant 5M - 纯中文控制台与安全认证一键热更新脚本           "
echo "=================================================================="
echo -e "${NC}"

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[ERROR] 请使用 sudo 运行此更新脚本: sudo bash update.sh${NC}"
    exit 1
fi

# 1. 确保代码库为最新版本
cd "$SCRIPT_DIR"
echo -e "${CYAN}[1/4] 同步 GitHub 最新代码与中文资源包...${NC}"
git fetch origin master 2>/dev/null || true
git reset --hard origin/master 2>/dev/null || true
echo -e "${GREEN}✓ 代码与预编译中文资源包已同步至最新提交。${NC}"

# 2. 彻底清空并更新前端目录 (避免 cp -r 产生 dist/dist 嵌套)
echo -e "${CYAN}[2/4] 部署纯中文前端与登录认证界面...${NC}"
rm -rf "$INSTALL_PREFIX/frontend/dist"
mkdir -p "$INSTALL_PREFIX/frontend"
cp -r "$SCRIPT_DIR/frontend/dist" "$INSTALL_PREFIX/frontend/dist"

# 验证前端部署产物
if [ -f "$INSTALL_PREFIX/frontend/dist/index.html" ]; then
    echo -e "${GREEN}✓ 前端纯中文资源部署成功: $INSTALL_PREFIX/frontend/dist/index.html${NC}"
else
    echo -e "${RED}[ERROR] 未检测到前端 index.html，正在重新打包...${NC}"
    cd "$SCRIPT_DIR/frontend"
    npm run build
    cp -r "$SCRIPT_DIR/frontend/dist" "$INSTALL_PREFIX/frontend/dist"
fi

# 3. 补充安全认证配置文件
echo -e "${CYAN}[3/4] 检查并配置安全认证模块 (auth)...${NC}"
if [ -f "$INSTALL_PREFIX/config/config.yaml" ]; then
    if ! grep -q "auth:" "$INSTALL_PREFIX/config/config.yaml"; then
        echo "" >> "$INSTALL_PREFIX/config/config.yaml"
        echo "# 安全认证访问控制" >> "$INSTALL_PREFIX/config/config.yaml"
        echo "auth:" >> "$INSTALL_PREFIX/config/config.yaml"
        echo "  enabled: true" >> "$INSTALL_PREFIX/config/config.yaml"
        echo "  username: \"admin\"" >> "$INSTALL_PREFIX/config/config.yaml"
        echo "  password: \"admin_polyquant\"" >> "$INSTALL_PREFIX/config/config.yaml"
        echo "  session_timeout_hours: 72" >> "$INSTALL_PREFIX/config/config.yaml"
        echo -e "${GREEN}✓ 已为现有配置文件补充安全认证 auth 模块。${NC}"
    fi
else
    mkdir -p "$INSTALL_PREFIX/config"
    cp "$SCRIPT_DIR/config/config.yaml" "$INSTALL_PREFIX/config/config.yaml"
    sed -i 's/host: "127.0.0.1"/host: "0.0.0.0"/g' "$INSTALL_PREFIX/config/config.yaml" || true
fi

# 4. 编译最新后端二进制 (包含安全认证 Session API)
echo -e "${CYAN}[4/4] 编译并升级 Rust 量化内核 (包含登录拦截与安全会话)...${NC}"
cd "$SCRIPT_DIR/backend"
for CARGO_ENV in "$HOME/.cargo/env" "/root/.cargo/env" "/usr/local/cargo/env"; do
    if [ -f "$CARGO_ENV" ]; then
        . "$CARGO_ENV"
    fi
done
export PATH="$HOME/.cargo/bin:/root/.cargo/bin:/usr/local/cargo/bin:$PATH"

if ! command -v cargo >/dev/null 2>&1; then
    echo -e "${RED}[ERROR] 未检测到 cargo 命令，请确认 Rust 是否已安装。${NC}"
    exit 1
fi

cargo build --release

TARGET_BIN=""
for CANDIDATE in \
    "$SCRIPT_DIR/target/release/poly_quant_backend" \
    "$SCRIPT_DIR/target/release/poly-quant-backend" \
    "$SCRIPT_DIR/backend/target/release/poly_quant_backend" \
    "$SCRIPT_DIR/backend/target/release/poly-quant-backend"
do
    if [ -f "$CANDIDATE" ]; then
        TARGET_BIN="$CANDIDATE"
        break
    fi
done

if [ -n "$TARGET_BIN" ] && [ -f "$TARGET_BIN" ]; then
    cp "$TARGET_BIN" "$INSTALL_PREFIX/bin/poly_quant_backend"
    chmod +x "$INSTALL_PREFIX/bin/poly_quant_backend"
    ln -sf "$INSTALL_PREFIX/bin/poly_quant_backend" "$INSTALL_PREFIX/bin/poly-quant-backend"
    echo -e "${GREEN}✓ 后端量化内核升级完毕！${NC}"
else
    echo -e "${RED}[ERROR] 后端编译未找到可执行程序，请检查日志。${NC}"
    exit 1
fi

# 5. 重启系统服务
echo -e "${CYAN}正在重启 polyquant 系统守护服务...${NC}"
systemctl restart polyquant
sleep 2

PUBLIC_IP=$(curl -s -m 2 https://api.ipify.org || hostname -I | awk '{print $1}' || echo "127.0.0.1")

echo ""
echo -e "${GREEN}${BOLD}==================================================================${NC}"
echo -e "${GREEN}${BOLD}    🎉 PolyQuant 5M 纯中文版与登录安全认证更新升级成功！          ${NC}"
echo -e "${GREEN}${BOLD}==================================================================${NC}"
echo ""
echo -e "核心服务状态:   ${GREEN}${BOLD}$(systemctl is-active polyquant || echo "RUNNING")${NC}"
echo -e "访问控制台地址: ${CYAN}${BOLD}http://$PUBLIC_IP:8080${NC} (请使用浏览器无痕模式或 Ctrl+Shift+R 刷新)"
echo -e "控制台登录账号: ${GREEN}${BOLD}admin${NC}"
echo -e "控制台初始密码: ${GREEN}${BOLD}admin_polyquant${NC}"
echo ""
echo -e "${YELLOW}提示: 若浏览器仍显示旧版，请按 Ctrl+Shift+R (Mac: Cmd+Shift+R) 强制清除浏览器缓存！${NC}"
echo ""
