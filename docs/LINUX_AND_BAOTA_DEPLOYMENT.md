# PolyQuant 5M - Linux 服务器与宝塔面板 (aaPanel) 一键部署指南

本文档提供在 **任意主流 Linux 发行版（Ubuntu / Debian / CentOS / Rocky Linux / AlmaLinux）** 以及 **宝塔面板（aaPanel）** 上一键安装与生产化托管运行 **Polymarket 5 分钟 Crypto Up/Down 量化预测与模拟交易系统** 的完整指南。

---

## 目录

1. [架构与环境要求](#一架构与环境要求)
2. [方式一：原生 Linux 一键全自动安装 (最推荐)](#二方式一原生-linux-一键全自动安装-最推荐)
3. [方式二：宝塔面板 (aaPanel) 极速部署实操](#三方式二宝塔面板-aapanel-极速部署实操)
   - [步骤 1：终端一键执行安装](#步骤-1终端一键执行安装)
   - [步骤 2：安全组与宝塔防火墙放行 8080 端口](#步骤-2安全组与宝塔防火墙放行-8080-端口)
   - [步骤 3：绑定域名与 Nginx 反向代理 (带 WebSocket 支持)](#步骤-3绑定域名与-nginx-反向代理-带-websocket-支持)
   - [步骤 4：宝塔「进程守护管理器」托管 (备选)](#步骤-4宝塔进程守护管理器托管-备选)
4. [方式三：Docker / Docker Compose 容器化一键部署](#四方式三docker--docker-compose-容器化一键部署)
5. [系统管理与常用运维命令 (`polyquant`)](#五系统管理与常用运维命令-polyquant)
6. [配置文件与参数调优说明](#六配置文件与参数调优说明)
7. [常见排错问答 (FAQ)](#七常见排错问答-faq)

---

## 一、架构与环境要求

- **操作系统**：Ubuntu 20.04/22.04/24.04 LTS、Debian 11/12、CentOS 7/8/9、Rocky Linux、AlmaLinux
- **最低配置**：1 核 CPU，1 GB 内存，10 GB 磁盘空间
- **推荐配置**：2 核 CPU，2 GB 内存，20 GB 磁盘空间
- **网络环境**：服务器需能访问 Binance、OKX、Bybit、Coinbase、Polymarket 的公网 API 与 WebSocket 节点（建议使用香港、日本、新加坡或欧美节点云服务器，如阿里云国际、腾讯云海外、AWS、Google Cloud、DigitalOcean 等）
- **核心安全守则**：系统原生强制只读仿真模式（`real_trading_enabled: false`），零真金风险，禁止加载任何私钥。

---

## 二、方式一：原生 Linux 一键全自动安装 (最推荐)

安装脚本会自动识别您的操作系统、安装 GCC/OpenSSL/SQLite 编译库、全自动安装 Node.js v20 LTS 与 Rust 稳定版工具链、编译前端生产资源包与 Release 后端执行文件，并自动注册并启动 `systemd` 系统服务。

### 1. 登录服务器并克隆代码

```bash
# 进入安装工作区
cd /root

# 拉取仓库（若使用压缩包上传，解压后进入目录即可）
git clone <您的仓库URL> Poly量化
cd Poly量化
```

### 2. 执行一键安装脚本

```bash
sudo bash install.sh
```

### 3. 安装完成后立即输出结果

安装脚本完成后会自动执行健康检测并输出：

```text
==================================================================
      🎉 POLYQUANT 5M 模拟量化系统在 LINUX 上安装部署成功！        
==================================================================

核心服务状态:   RUNNING (systemd 守护运行中)
访问控制台地址: http://<您的服务器IP>:8080 (或 http://127.0.0.1:8080)
健康检查 API:   http://127.0.0.1:8080/api/v1/health
WebSocket 接口: ws://127.0.0.1:8080/api/v1/ws
配置文件路径:   /opt/polyquant/config/config.yaml
数据库存储路径: /opt/polyquant/data/poly_quant.db
系统日志路径:   /opt/polyquant/logs/service.log
```

---

## 三、方式二：宝塔面板 (aaPanel) 极速部署实操

如果您习惯使用宝塔面板管理网站与服务器，只需按以下步骤操作：

### 步骤 1：终端一键执行安装

1. 打开宝塔面板，点击左侧菜单 **「终端」**（输入服务器 root 密码登录）。
2. 在终端中直接运行：
   ```bash
   cd /root
   git clone <您的仓库URL> Poly量化
   cd Poly量化
   sudo bash install.sh
   ```
   脚本会自动检测到宝塔环境，自动编译前后端，部署至 `/opt/polyquant`，并在系统后台以守护进程运行。

### 步骤 2：安全组与宝塔防火墙放行 8080 端口

- **宝塔面板内**：点击左侧 **「安全」** -> **「防火墙」** -> 放行端口 `8080`，备注 `PolyQuant`。
- **云服务商后台**：如果您使用的是阿里云、腾讯云、华为云等，请在控制台对应的 **「安全组」** 中添加入方向规则，允许 TCP `8080` 端口访问。

> 完成后，直接在浏览器中打开 `http://您的服务器IP:8080` 即可直接看到量化控制台！

---

### 步骤 3：绑定域名与 Nginx 反向代理 (带 WebSocket 支持)

如果您希望使用自有域名访问（如 `quant.yourdomain.com`）并配置 SSL（HTTPS），可通过宝塔的反向代理一键完成：

1. **添加站点**：
   - 宝塔面板 -> **「网站」** -> **「添加站点」**。
   - 域名填写您的解析域名（如 `quant.yourdomain.com`）。
   - FTP、数据库选择“不创建”，PHP 版本选择“纯静态”。
2. **配置 SSL 证书 (可选，推荐)**：
   - 在该站点设置中，点击 **「SSL」** -> 申请 Let's Encrypt 免费证书并开启强制 HTTPS。
3. **添加反向代理**：
   - 点击该站点设置中的 **「反向代理」** -> **「添加反向代理」**。
   - 代理名称：`polyquant`
   - 目标 URL：`http://127.0.0.1:8080`
   - 发送域名：`$host`
4. **开启 WebSocket 全双工支持 (核心)**：
   - 点击刚才添加的反向代理条目右侧的 **「配置文件」**。
   - 将内容完整替换为以下配置（已提供在 `deploy/baota/nginx_reverse_proxy.conf`）：

```nginx
location / {
    proxy_pass http://127.0.0.1:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header REMOTE-HOST $remote_addr;
    proxy_set_header X-Forwarded-Proto $scheme;

    # --------------------------------------------------------------------------
    # 核心：全双工 WebSocket 长连接升级支持 (用于 /api/v1/ws 毫秒级推流)
    # --------------------------------------------------------------------------
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";

    # 关闭反代缓冲，保证 500ms 行情与策略决策信号零延迟直达浏览器
    proxy_buffering off;
    proxy_cache off;

    # 防止长连接被 Nginx 超时中断 (设置 24 小时心跳保活)
    proxy_connect_timeout 60s;
    proxy_read_timeout 86400s;
    proxy_send_timeout 86400s;

    proxy_intercept_errors off;
}
```

保存后，您就可以直接通过 `https://quant.yourdomain.com` 极速访问量化控制台，毫秒级订单薄、37 维特征与实时交易决策全部秒级推送！

---

### 步骤 4：宝塔「进程守护管理器」托管 (备选)

如果您更偏好在宝塔面板的可视化界面中管理进程开关与日志：

1. 宝塔面板 -> **「软件商店」** -> 搜索 **「进程守护管理器」** (Supervisor) 并安装。
2. 打开进程守护管理器，点击 **「添加守护进程」**：
   - **名称**：`polyquant`
   - **启动用户**：`root`
   - **运行目录**：`/opt/polyquant`
   - **启动命令**：`/opt/polyquant/bin/poly_quant_backend`
   - **进程数量**：`1`
3. 保存后即可在宝塔界面中实时查看 CPU 占用、内存消耗、一键重启与查看输出日志。

---

## 四、方式三：Docker / Docker Compose 容器化一键部署

如果您希望环境彻底容器化隔离，系统已预置多阶段精简镜像 Dockerfile 与 docker-compose.yml：

### 一键启动容器

```bash
# 在项目根目录下执行
docker compose up -d --build
```

- 容器会自动构建前端并编译后端二进制。
- 数据持久化目录：`./data` (SQLite 数据库) 与 `./config` (配置文件)。
- 检查运行状态：
  ```bash
  docker compose ps
  docker compose logs -f
  ```

---

## 五、系统管理与常用运维命令 (`polyquant`)

安装脚本在系统 `/usr/local/bin/polyquant` 自动创建了全局管理快捷指令，支持随时随地操作：

| 指令 | 作用说明 |
| :--- | :--- |
| `polyquant status` | 查看服务运行状态、进程 PID、监听端口及健康检查响应 |
| `polyquant log` | 实时追踪查看后端行情采集、模型预测、策略评估与模拟成交日志 |
| `polyquant restart` | 重启量化引擎服务 |
| `polyquant stop` | 停止量化系统 |
| `polyquant start` | 启动量化系统 |
| `polyquant config` | 打开并编辑量化系统配置文件（退出保存后自动提示重启） |
| `polyquant update` | 一键 `git pull` 拉取最新代码、自动重新编译并热重启 |

```bash
# 查看实时日志示例
polyquant log

# 查看服务状态示例
polyquant status
```

---

## 六、配置文件与参数调优说明

配置文件位于 `/opt/polyquant/config/config.yaml`。

您可以直接在前端控制台的 **「Strategy 调优与训练」** 选项卡中热修改生效（无需重启）；或者通过命令行编辑后重启：

```bash
polyquant config
polyquant restart
```

### 关键量化风控参数：

```yaml
# 资金管理与锁盈模式
bankroll:
  initial: 10.0          # 初始模拟本金 10 USDC
  cap: 10.0              # 资金池上限 10 USDC，超出利润 100% 自动划转至锁盈池
  minimum: 2.0           # 熔断线：低于 2 USDC 立即熔断停机
  mode: "capital_recovery" # Mode B: 亏损时优先回本充能，回满后超额利润锁定

# 策略硬过滤门槛
strategy:
  min_probability: 0.70  # 胜率门槛：低于 70% 自动跳过（SKIP）
  min_net_edge: 0.05     # 净正期望门槛：扣除手续费与滑点后必须拥有 >= 5% 净数学优势
  max_entry_price: 0.85  # 入场价格天花板：防止追高赔率极差的合约
  max_spread: 0.04       # 最大盘口点差：点差超过 4¢ 时坚决放弃
  min_liquidity: 300.0   # 最低盘口深度：流动性不足 300 USDC 时放弃

# 严格仿真撮合与摩擦成本
execution:
  latency_ms: 250        # 网络与链上反应时延仿真 (250ms)
  orderbook_depth_fill: true # 真实订单薄深度吃单与冲击成本模拟
  fee_rate: 0.012        # 交易费率 1.2%
  slippage_rate: 0.005   # 滑点估算 0.5%
```

---

## 七、常见排错问答 (FAQ)

### Q1: 浏览器打开 `http://服务器IP:8080` 提示无法连接？
1. 检查服务是否处于运行状态：
   ```bash
   polyquant status
   ```
2. 检查 8080 端口是否在监听：
   ```bash
   netstat -tlpn | grep 8080
   ```
3. 检查云服务商安全组和宝塔防火墙是否已放行 `8080` 端口。

### Q2: 为什么控制台右上角的交易所延时（Binance / OKX 等）显示红标？
- 请检查您的云服务器是否位于国内大陆地区。由于国内大陆网络策略限制，Binance 和 OKX 的境外 WebSocket 会被阻断。
- **解决方案**：量化服务器强烈推荐部署于**香港、日本、新加坡或欧美**云节点（如阿里云香港、AWS 等），即可获得 20~50ms 极低行情延迟。

### Q3: 为什么反向代理后 WebSocket 经常断开？
- 宝塔默认的 Nginx 反代配置中可能缺少长连接升级协议。
- 请检查 Nginx 站点反代配置中是否包含以下关键三行（详见本文档方式二步骤 3）：
  ```nginx
  proxy_http_version 1.1;
  proxy_set_header Upgrade $http_upgrade;
  proxy_set_header Connection "upgrade";
  proxy_read_timeout 86400s;
  ```

### Q4: 如何迁移或备份模拟交易数据与特征库？
- 系统使用高性能 SQLite WAL 数据库，所有数据集中保存在：
  `/opt/polyquant/data/poly_quant.db`
- 仅需备份该单个文件，即可完整保留所有历史订单、盘口特征样本、回测收益曲线与锁盈池记录！
