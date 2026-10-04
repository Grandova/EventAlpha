# EventAlpha - Polymarket 5-Minute Crypto Up/Down Quant System
### 工业级高精度量化预测、概率校准、模拟盘与实盘双模式、自主在线学习与自进化量化交易系统

![Mode](https://img.shields.io/badge/Trading%20Mode-Paper%20%7C%20Live%20CLOB-brightgreen)
![Security](https://img.shields.io/badge/Security-Session%20Auth%20Protected-green)
![Language](https://img.shields.io/badge/UI%20Language-%E7%AE%80%E4%BD%93%E4%B8%AD%E6%96%87-blue)
![Safety](https://img.shields.io/badge/Safety%20Lock-Mode%20B%20%2410%20Cap%20%26%20Emergency%20Halt-red)
![UI Style](https://img.shields.io/badge/UI%20Style-AsmrProg--YT%20Dashboard%20Designs-cyan)
![Rust](https://img.shields.io/badge/Rust-1.85+-orange)
![Frontend](https://img.shields.io/badge/Frontend-React%2018%20%7C%20Vite%206%20%7C%20Tailwind-blue)
![Database](https://img.shields.io/badge/Database-SQLite%20WAL%20(19%20Tables)-purple)
![Self-Learning](https://img.shields.io/badge/AI%20Engine-Online%20SGD%20Incremental%20Learning-blueviolet)
![Tests](https://img.shields.io/badge/Tests-16%20Suites%20%7C%2093%20Passed%20(100%25)-success)
![License](https://img.shields.io/badge/License-MIT-lightgrey)

---

## 🌟 核心亮点与系统定位

**EventAlpha** 是一套专为 **Polymarket** 上的 **BTC / ETH / SOL 5 分钟 Up/Down 二元期权预测市场** 打造的工业级量化策略研发、自主在线学习自进化、模拟盘推演与 Polymarket CLOB 实盘撮合落地系统。

系统坚决摒弃“玩具 Demo”与过度简化的假设，全链路遵循高频量化与预测市场微观结构规范，以 **“真实可运行、数据零泄漏、严格时间箭头、持续自主学习越来越强、真实盘口撮合、严格风控熔断”** 为最高准则。

---

## 🚀 核心功能矩阵

### 1. 模拟盘 (Paper Trading) 与 实盘 (Live Trading) 双模无缝切换
- **模拟盘 (Paper)**：采用真实订单簿 Section 21 深度穿透撮合（Depth-Walking Fill），扣除真实滑点与手续费，零本金风险验证策略。
- **实盘 (Live)**：支持绑定真实的 Polymarket 账户凭据（API Key / Secret / Passphrase / Polygon 钱包地址），通过 Polymarket L2 HMAC-SHA256 签名协议直连真实订单簿撮合。
- **实盘安全防线（Hard Protection）**：
  - 未绑定/激活账户时强制阻断切换实盘；
  - 严格限制单笔最大下注与资金硬顶（Mode B 封顶 $10.00 USDC）；
  - 配备前台全局 **一键紧急熔断 (KILL SWITCH)**：毫秒级撤销全部在途委托并强制安全切回模拟盘。

### 2. 自主在线学习与持续自进化引擎 (Continuous Online Self-Learning)
- **让量化系统越跑越强**：系统配备专门的在线自学习演化中枢。
- **在线单轮增量 SGD 更新**：每当一个 5 分钟盘面结算（UP 或 DOWN）后，自学习引擎自动将该轮收集的 37 维微观特征、预测概率与实际胜负结果进行损失计算，毫秒级步进更新权重：$w \leftarrow w - \eta (p - y)x$。
- **动态 Platt 概率校准**：自适应修正 Logistic 回归输出概率偏差，使长期置信度真实对应大数定律胜率。
- **全历史断点续传**：学习所得的模型权重、偏差与校准参数实时保存在 SQLite 数据库中，跨服务重启自愈继承，经验永不丢失。
- **一键全量历史重训练**：支持基于历史所有结算盘面进行全样本 SGD 批量重训演化。

### 3. AsmrProg-YT 经典三栏式 Responsive Dashboard UI 控制台
深度复刻 **[AsmrProg-YT Dashboard-Designs](https://github.com/AsmrProg-YT/Dashboard-Designs)** 的代表作设计语言：
- **经典三栏式栅格布局 (3-Column Layout)**：
  - **左侧导航栏 (Left Sidebar)**：带有经典红黑双环 AP 标识 (Double-Ring AP Logo)，激活状态配备标志性的左侧圆角竖条指示器 (Active Pill Indicator)；
  - **中央主看板 (Main Analytics Area)**：顶部 Analytics 标题与日夜模式切换器，下接经典 SVG 环形进度指标卡片 (Circular Progress Rings)、活跃币种头像流 (Active Markets Row) 与精致 Recent Orders 订单流水表；
  - **右侧多功能区 (Right Profile & Reminders)**：顶部 Profile 身份徽章卡片与活跃 Polymarket 账户状态，中下部配置实时行情策略备忘提醒 (Reminders) 以及虚线「+ Add Account」快速开户入口。
- **日间白昼 (Light) 与深色暗夜 (Dark) 完美双模**：
  - **日间模式**：`#f6f6f9` 浅灰极简底色，搭配纯白 `#ffffff` 圆角卡片与弥散柔和投影 `0 1.5rem 2rem rgba(132, 139, 200, 0.18)`；
  - **深色模式**：`#181a1e` 暗夜底色与 `#202528` 高质感深灰卡片，通过顶部 Sun/Moon 按钮一键平滑切换，状态持久化至本地存储。
- **SVG 动态环形进度圆环 (Circular Progress Rings)**：在活跃本金卡、策略胜率卡、模型自学习卡中展示高精度 SVG 圆环与居中百分比，实时映射盘面状态。
- **内存级在线增量自学习 (In-Memory Online SGD)**：每次 5 分钟盘面结算后，SGD 权重梯度直接原子写入运行中的模型管理中枢 (`ModelManager.logistic.write()`)，免重启即刻赋能下一轮推演。

### 4. 专为国内大陆交易者打造的 100% 纯中文本地化交互
- **全要素中文界面**：摒弃生涩外文词汇，所有导航栏、仪表盘指标卡、活跃币种分析、机会评分雷达、实时交易流水、深度委托簿、资金风控中心、回测控制台均使用纯正简体中文呈现；
- **异常安全兜底（Null-Safe Resilience）**：全面加固盘口价格与指标数据管道，对所有价格、收益、深度数值增加非空与有效性校验，彻底消除前端 `toFixed` 渲染异常风险。

### 5. 🔐 公网服务器部署安全防护与账户登录认证体系
- **彻底杜绝公网裸奔风险**：云服务器（如部署在公网 IP `185.248.185.194`）暴露在互联网时，系统内置强安全门禁；
- **默认登录凭据**：
  - **默认账号**：`admin`
  - **默认初始密码**：`admin_polyquant`
- **Session Bearer Token 机制**：登录成功后签发高安全随机 Session Token，所有 API 均进行 Bearer 身份校验，未认证或超时自动阻断并跳转至现代化登录界面；
- **凭据自定义与环境变量覆盖**：
  - 支持在配置文件 `config/config.yaml` 的 `auth:` 模块中自定义修改用户名和密码；
  - 支持直接通过环境变量 `ADMIN_USER` 和 `ADMIN_PASSWORD` 实现零改密部署；
  - 默认 Session 保持时间 72 小时，支持前台一键退出登录（右上角与左侧导航底部均可一键注销）。

---

## 💻 控制台视图概览 (AsmrProg 3-Column Responsive Grid)

```
+-------------------------------------------------------------------------------------------------------------------------+
| [AP] EventAlpha       |  Analytics  [☀️/🌙]  [模拟盘|实盘] [KILL SWITCH]          | [Profile] Admin / Polymarket MM     |
+-----------------------+-------------------------------------------------------------+-----------------------------------+
|  [Left Sidebar]       |  [Stat Cards with SVG Circular Progress Rings]              |  [Right Profile & Reminders]      |
|  * Dashboard (Active) |  +----------------+  +----------------+  +----------------+ |  +-----------------------------+  |
|  * Real-Time Trading  |  | Active Bankroll|  | Online Learning|  | Strategy Win-R | |  | Active: Main MM (0x8f21)   |  |
|  * Self-Learning AI   |  |   [SVG Ring]   |  |   [SVG Ring]   |  |   [SVG Ring]   | |  | Balance: $48.50 USDC       |  |
|  * Live CLOB Orders   |  |      81%       |  |      48%       |  |      73%       | |  +-----------------------------+  |
|  * Account Auth       |  |  $8.50 / $10   |  |  SGD 1,280 Rds |  |    19W - 7L    | |                                   |
|  * OrderBook Ladder   |  +----------------+  +----------------+  +----------------+ |  [Reminders & Alerts]             |
|  * Feature Center     |                                                             |  * [BTC] High OBI Imbalance (+0.6)|  |
|  * Settings           |  [Active Markets Row: BTC | ETH | SOL]                      |  * [ETH] Volatility Spike Alert   |  |
|                       |                                                             |                                   |
|                       |  [Recent Orders Table with Colored Status]                  |  +-----------------------------+  |
|                       |  Asset | Time  | Side | Price | Shares | Mode | Status      |  | [+ Add Polymarket Account]  |  |
|                       |  BTC   | 03:25 | BUY  | 0.520 | 16.3   | LIVE | Active      |  +-----------------------------+  |
|                       |  ETH   | 03:20 | BUY  | 0.480 | 10.4   | LIVE | Declined    |                                   |
|                       |  SOL   | 03:15 | SELL | 0.510 | 20.0   | PAPR | Pending     |                                   |
+-----------------------+-------------------------------------------------------------+-----------------------------------+
```

---

## 🚀 Linux 与宝塔面板 (aaPanel) 一键极速安装

系统已原生支持所有主流 Linux 发行版（Ubuntu、Debian、CentOS、Rocky Linux、AlmaLinux）及宝塔面板，提供全自动一键安装脚本。

### 1. 原生 Linux / 宝塔终端一键执行（最推荐）

登录您的 Linux 云服务器或打开宝塔面板中的 **「终端」**，执行以下命令：

```bash
# 1. 克隆代码仓库
git clone https://github.com/Grandova/EventAlpha.git
cd EventAlpha

# 2. 执行一键安装脚本（自动安装所有依赖、编译前后端、注册系统服务）
sudo bash install.sh

# 3. 后续升级更新（若此前已安装过，任选其一即可）：
polyquant update          # 方式一：直接运行全局快捷更新命令
# 或：
cd ~/EventAlpha && git pull && sudo bash install.sh   # 方式二：手动拉取并重新部署
```

> 🔑 **访问控制台与初始登录凭据**：
> 安装完成后，直接在浏览器中打开：`http://您的服务器IP:8080`
> - **登录账号**：`admin`
> - **初始密码**：`admin_polyquant`
> 系统已全面配置安全认证拦截防线，有效保护暴露在公网上的量化策略与真实交易资金安全。可在 `/opt/polyquant/config/config.yaml` 中随时修改或通过环境变量 `ADMIN_PASSWORD` 覆盖。

**安装脚本将全自动完成以下工作：**
1. 自动检测 Linux 发行版与包管理器（`apt` / `dnf` / `yum` / `apk`）；
2. 自动安装系统核心编译库（GCC, Make, OpenSSL, SQLite3, PKG-Config）；
3. 自动安装或升级 **Node.js v20 LTS** 与 **Rust 稳定版工具链**；
4. 编译打包 React 18 + Tailwind 前端生产资源；
5. 以 `--release` 极速优化模式编译 Rust 量化引擎；
6. 自动部署至 `/opt/polyquant`，并注册为 `systemd` 系统级守护服务（自启动、崩溃秒级重启）；
7. 自动注入全局命令行管理指令 `polyquant`。

---

### 2. 全局命令行管理指令 (`polyquant`)

安装完成后，可以在服务器任意路径直接使用 `polyquant` 命令管理量化系统：

```bash
polyquant status   # 查看服务实时运行状态、内存占用、PID 与健康指标
polyquant log      # 实时追踪行情采集、特征计算、自学习更新与撮合日志 (按 Ctrl+C 退出)
polyquant restart  # 一键重启量化服务
polyquant stop     # 停止量化服务
polyquant config   # 快速编辑量化参数与风控阈值 (保存后自动生效)
polyquant update   # 一键从 GitHub 拉取最新代码并热重编译更新
```

---

### 3. 宝塔面板 (aaPanel) 专属配置指引

1. **端口放行**：
   - 进入宝塔面板 -> **「安全」** -> **「防火墙」** -> 放行端口 `8080`，备注 `EventAlpha`。
   - （如使用阿里云/腾讯云，在对应云控制台「安全组」放行入方向 `8080` 端口）。
   - 访问 `http://您的服务器IP:8080` 即可直接看到控制台！

2. **域名绑定与 Nginx 反向代理（推荐配置 SSL）**：
   - 宝塔 -> **「网站」** -> 添加站点（如 `quant.yourdomain.com`）。
   - 点击该站点设置 -> **「反向代理」** -> 添加反向代理：
     - **代理名称**：`eventalpha`
     - **目标 URL**：`http://127.0.0.1:8080`
     - **发送域名**：`$host`
   - 为确保 WebSocket 毫秒级推流正常，请点击该反代配置的「配置文件」，确认包含如下 WebSocket 支持（或直接使用预置的 [`deploy/baota/nginx_reverse_proxy.conf`](deploy/baota/nginx_reverse_proxy.conf)）：
     ```nginx
     proxy_http_version 1.1;
     proxy_set_header Upgrade $http_upgrade;
     proxy_set_header Connection "upgrade";
     ```

---

## 🔑 Polymarket 账户配置与实盘接入指南

系统支持在前端直接添加与管理 Polymarket 账户：

1. 打开前端控制台右上角 **「账户授权」** 或点击左侧导航栏 **「Accounts」**；
2. 点击 **「添加新账户」**，输入：
   - **账户别名**（如：主力实盘账户 01）
   - **Polygon 钱包地址**（0x 开头的私钥派生公钥地址）
   - **Polymarket CLOB API Key**
   - **API Secret (Base64)**
   - **API Passphrase**
3. 点击 **「确认安全添加」**：
   - 凭据仅保存于本地宿主机的 SQLite 数据库；
   - 前台展示自动进行密钥掩码处理（如 `a1b2...c3d4`）；
   - 点击 **「刷新」** 即可实时查询链上 Polygon USDC 余额；
4. 点击顶部模式切换器中的 **「实盘」**，阅读风险确认提示后即可开始全自动或半自动交易！
5. 如遇任何异常行情，可随时点击顶部红色的 **「紧急熔断 (KILL SWITCH)」** 一键回退至模拟盘。

---

## 🧠 自进化在线学习与模型演进

1. **单轮在线演进（Auto-Learning）**：
   - 默认开启；
   - 每次 5 分钟盘面结算时，系统自动捕捉微观订单簿、CVD 与多周期波动率等 37 维特征与胜负标签；
   - 计算交叉熵梯度损失并即时更新权重向量与 Platt 参数；
   - 在前端 **「Auto-Learning」** 面板可实时查看特征重要性柱状图梯级。
2. **全量历史批量重训练（Batch Retrain）**：
   - 点击面板上的 **「全量历史回放自学习」**；
   - 引擎将回放数据库中沉淀的成百上千轮历史 5 分钟盘面；
   - 采用带动量项的 SGD 进行多 Epoch 拟合，大幅提升样本外泛化能力与 Brier Score 分数。

---

## 📊 资金管理模式与四重风控熔断

### 1. 资金双池隔离机制
- **Active Bankroll（活跃本金池）**：实际参与下注的动态本金，默认初始 **10.00 USDC**，上限严格封顶为 **10.00 USDC**。
- **Locked Profit（锁定利润池）**：交易盈利剥离金库，**永远不用于再次下注**，确保“已落袋利润绝对安全”。
- **模式 B（Capital Recovery，默认推荐模式）**：
  - 若活跃本金发生回撤（例如亏损至 9.00 USDC），后续交易盈利优先补足活跃本金至 10.00 USDC 封顶；
  - 超过 10.00 USDC 的溢出盈利部分，**100% 自动划入 Locked Profit 锁定**。
- **模式 A（Profit Isolation，利润完全隔离模式）**：
  - 每一笔平仓盈利的净收益均直接 100% 剥离进入 Locked Profit，本金池不递增。

### 2. 四重硬性风控熔断器（Circuit Breakers）
| 风控项 | 默认阈值 | 触发机制与处理动作 |
| :--- | :--- | :--- |
| **本金最低下限** | `bankroll.minimum = 2.0 USDC` | 若活跃本金 $\le 2$ USDC，全系统自动停机，禁止一切新开仓 |
| **单日最大亏损** | `risk.daily_loss_limit = 2.0 USDC` | 当日累计净亏损达到 2 USDC，触发熔断，当日禁止开仓 |
| **最大回撤限制** | `risk.max_drawdown = 20%` | 从本金历史峰值回撤达到 20%，系统暂停开仓 |
| **连续亏损冷静** | `risk.max_consecutive_losses = 5 次` | 连续遭遇 5 次亏损，自动进入 **30 分钟策略冷静期** |

---

## 📡 REST API 与 WebSocket 实时接口

| 模块分类 | 方式 | 接口路径 | 功能简述 |
| :--- | :--- | :--- | :--- |
| **安全认证** | `POST` | `/api/v1/auth/login` | 管理员登录，获取高安全 Bearer Session Token |
| | `GET` | `/api/v1/auth/me` | 校验当前 Token 状态与当前登录用户身份 |
| | `POST` | `/api/v1/auth/logout` | 注销并安全销毁当前 Session |
| **交易模式** | `GET` | `/api/v1/trading/mode` | 查询当前交易模式（paper/live）与当前活跃账户 |
| | `POST` | `/api/v1/trading/mode` | 切换交易模式（paper/live），包含前置账户状态校验 |
| **账户管理** | `GET` | `/api/v1/accounts` | 获取所有绑定的 Polymarket 账户信息（API Key 掩码） |
| | `POST` | `/api/v1/accounts` | 绑定新 Polymarket 账户凭据并加密存储 |
| | `POST` | `/api/v1/accounts/{id}/activate` | 设为当前活跃实盘交易账户 |
| | `DELETE`| `/api/v1/accounts/{id}` | 删除指定账户 |
| | `GET` | `/api/v1/accounts/{id}/balance` | 实时刷新并返回账户 Polygon USDC 余额 |
| **实盘执行** | `GET` | `/api/v1/real/orders` | 获取 Polymarket CLOB 真实委托与成交审计明细 |
| | `POST` | `/api/v1/real/emergency_halt` | **紧急熔断接口**：立即撤销全部在途委托并强制切回模拟盘 |
| **在线学习** | `GET` | `/api/v1/learning/status` | 获取当前模型版本、权重分布、Platt 参数与 Brier 分数 |
| | `POST` | `/api/v1/learning/toggle` | 开启或暂停每轮结算后的在线 SGD 自主学习 |
| | `POST` | `/api/v1/learning/retrain` | 触发基于全量历史样本的离线/在线批次重新训练 |
| | `GET` | `/api/v1/learning/history` | 查询历史盘面结算后的逐轮梯度变动与 Loss 履历 |
| **行情微观** | `GET` | `/api/v1/collector/prices` | Binance / OKX / Bybit / Coinbase 实时行情 |
| | `GET` | `/api/v1/composite/price/{asset}` | 稳健综合价格指数、收益率与已实现波动率 |
| | `GET` | `/api/v1/polymarket/markets` | Polymarket 5分钟周期活跃市场列表 |
| | `GET` | `/api/v1/polymarket/book/{asset}` | 5分钟二元盘口完整深度、买卖价差与 OBI |
| **特征预测** | `GET` | `/api/v1/features/latest/{asset}`| 37 维实时量化特征快照 |
| | `GET` | `/api/v1/models/prediction/{asset}`| 逻辑回归与校准后胜率预测、特征贡献度 |
| **回测复盘** | `POST` | `/api/v1/backtest/run` | 触发事件驱动历史回测模拟 |
| | `POST` | `/api/v1/replay/start` | 启动历史行情逐帧回放引擎 |

---

## 🧪 自动化测试套件（100% 通过）

运行全套 16 组集成与端到端测试：
```powershell
# Windows
cargo test -- --nocapture
```
```bash
# Linux
cargo test -- --nocapture
```

测试覆盖清单：
- `real_trading_and_learning_integration`: Polymarket 账户 CRUD、在线增量 SGD 自学习自进化、实盘模式门禁守卫测试；
- `phase1_integration`: 内核级安全拒绝、SQLite 数据库迁移、健康检查；
- `phase2_collector_integration`: 跨所归一化、Fail-closed 新鲜度中断；
- `phase3_polymarket_integration`: 5M周期推进、订单簿深度与结算解析；
- `phase4_composite_integration`: 综合价格指数、异常报价剔除；
- `phase5_feature_integration`: 37维特征对齐、时钟漂移检测；
- `phase6_dataset_integration`: 数据集严格无未来泄露 Walk-Forward 切分；
- `phase7_models_integration`: 逻辑回归在线训练、Platt 校准器胜率压缩；
- `phase9_strategy_integration`: 7 重硬过滤器拦截、机会评分体系；
- `phase10_execution_integration`: Section 21 深度穿透、流动性不足拒单；
- `phase11_risk_integration`: 模式 B 本金回收封顶、模式 A 隔离、四重熔断；
- `phase12_closed_loop_integration`: 真实结算闭环驱动、资金账本审计；
- `phase13_backtest_integration`: 历史回测驱动、Sharpe/Sortino/Calmar 计算；
- `phase14_replay_integration`: 历史逐帧回放、单步前进、时间定位；
- `phase17_realtime_and_tuning_integration`: 动态调参热重载与合成数据集推演。

---

## ⚖️ 免责声明

1. 本开源系统适用于二元预测市场的策略研究、模拟推演与合规实盘交易。
2. 实盘交易模式涉及加密资产与链上交互风险，请务必充分理解 Mode B 资金硬顶与风控设置，严禁超出自身风险承受能力下注。
3. 开发者不对任何因网络延迟、市场剧烈波动、第三方交易所或 Polymarket 节点宕机导致的潜在亏损承担连带责任。
