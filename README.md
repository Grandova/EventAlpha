# Polymarket 5-Minute Crypto Up/Down Quant System
### 高精度量化预测、概率校准、真实深度模拟撮合与逐帧回放系统

![Mode](https://img.shields.io/badge/Trading%20Mode-Strictly%20Paper%20Only-brightgreen)
![Safety](https://img.shields.io/badge/Safety%20Lock-Kernel%20Panic%20Enforced-red)
![Rust](https://img.shields.io/badge/Rust-1.85+-orange)
![Frontend](https://img.shields.io/badge/Frontend-React%2018%20%7C%20Vite%206%20%7C%20Tailwind-blue)
![Database](https://img.shields.io/badge/Database-SQLite%20WAL%20(15%20Tables)-purple)
![Tests](https://img.shields.io/badge/Tests-15%20Suites%20%7C%2089%20Passed%20(100%25)-success)

---

## 一、系统定位与设计原则

本项目为研究与量化验证 **Polymarket** 上的 **BTC / ETH / SOL 5 分钟 Up/Down 二元期权预测市场** 的工业级量化模拟交易与回测系统。
系统坚决摒弃“玩具 Demo”逻辑，全链路遵循高频量化与预测市场微观结构规范，以 **“真实可运行、数据零泄漏、严格时间箭头、真实盘口撮合、长期可持续验证”** 为核心准则。

### 核心安全准则（Zero Real Money Risk Invariant）
1. **系统内核级禁止实盘**：系统底层 `SafetyGuard` 强制断言，`real_trading_enabled` 必须恒为 `false`；任何试图在配置中启用实盘的操作均会在服务启动最前置阶段直接触发 `panic!` 终止进程。
2. **私钥隔离**：系统中不存在任何私钥存储或真实链上交易签名模块，彻底杜绝任何资金损失风险。
3. **Fail-Closed 保护**：行情断连、数据滞后（>2000ms）、时钟漂移（>1000ms）、盘口深度不足（<$300）时自动阻断模拟下单。

---

## 二、系统架构总览

```mermaid
flowchart TD
    subgraph Data Layer [数据采集与新鲜度追踪]
        B[Binance L2 & Ticks] --> C[Collector Engine]
        O[OKX L2 & Ticks] --> C
        BY[Bybit L2 & Ticks] --> C
        CB[Coinbase L2 & Ticks] --> C
        P_CLOB[Polymarket CLOB & Discovery] --> PM[Polymarket Engine]
        C --> FRESH[Freshness Tracker <2000ms]
    end

    subgraph Core Quant Engine [核心量化与特征计算]
        C --> COMP[Composite Price Engine]
        COMP --> FEAT[37-Dim Real-Time Feature Matrix]
        PM --> FEAT
        FEAT --> ML[ML Logistic & Platt Calibrator & Ensemble]
    end

    subgraph Strategy & Risk [策略门控与风控]
        ML --> STRAT[Strategy Engine: 7 Hard Filter Gates]
        FEAT --> STRAT
        STRAT --> SCORE[Opportunity Scoring: 0-100 Rubric]
        SCORE --> RISK[Risk Manager: Mode B $10 Cap & Mode A]
    end

    subgraph Execution & Simulation [真实深度撮合与回测]
        RISK --> EXEC[Paper Execution: Section 21 Depth-Walking Fill]
        PM --> EXEC
        EXEC --> DB[(SQLite WAL Database: 15 Tables)]
        DB --> SETTLE[Closed-Loop Resolution Listener]
        SETTLE --> RISK
    end

    subgraph Diagnostic & UI [回放与前端控制台]
        DB --> BT[High-Performance Backtest Engine]
        DB --> REPLAY[Historical Replay Engine: 1x-50x / Step]
        DB --> API[Axum REST & WebSocket Server :8080]
        API --> UI[React 18 + Vite + Tailwind Web Console]
    end
```

---

## 三、全系统十六阶段完整功能全景

| 阶段 | 模块名称 | 核心功能与交付成果 |
| :--- | :--- | :--- |
| **Phase 1** | 项目骨架与内核守卫 | 严格目录架构、配置校验、`SafetyGuard` 熔断内核、15 张 SQLite 迁移表 |
| **Phase 2** | 多源行情采集与新鲜度 | Binance/OKX/Bybit/Coinbase 采集器、Fail-closed（<2000ms）心跳监控 |
| **Phase 3** | Polymarket 发现与盘口 | 5分钟周期计算、CLOB 订单簿维护、Top 5/10/20 OBI、结算监听器 |
| **Phase 4** | 稳健综合价格计算 | 动态加权中间价、多周期收益率、已实现波动率 $\sigma$、开盘价偏离度 |
| **Phase 5** | 37维实时特征引擎 | 收益率、波动率、CVD 累积成交差、订单簿微观结构、时间衰减因子 |
| **Phase 6** | 训练数据集生成与导出 | 严格无未来函数数据集构建、Walk-Forward 切分（70/15/15）、CSV/JSONL 导出 |
| **Phase 7** | 机器学习与概率校准 | 逻辑回归 + 在线 SGD、Platt 概率校准器、双模型集成与 Brier Score |
| **Phase 8** | 数据集导出与验证端点 | 样本统计汇总、正负样本平衡度监控、`/api/v1/dataset/summary` |
| **Phase 9** | 策略引擎与机会评分 | 7 重严格硬过滤器门控（Fail-closed）、[0, 100] 分值机会评价体系 |
| **Phase 10** | 真实模拟撮合执行 | Section 21 真实订单簿深度穿透撮合、滑点模拟、1.2% 手续费扣除 |
| **Phase 11** | 资金与风控引擎 | 模式 B（$10 本金回收封顶）、模式 A（利润完全隔离）、四重熔断风控 |
| **Phase 12** | 闭环市场结算与账本 | 真实市场结算事件监听、二元期权 $1.00/$0.00 收益计算、账本自动复原 |
| **Phase 13** | 高性能历史回测引擎 | 事件驱动历史仿真、Sharpe / Sortino / Calmar 风险比率、净值曲线追踪 |
| **Phase 14** | 历史逐帧回放引擎 | 历史 Tick 与特征逐帧步进（Step）、时间定位（Seek）、1x~50x 变速回放 |
| **Phase 15** | 现代化前端控制台 | React 18 + Vite 6 + Tailwind 黑色高对比度量化控制台、真实深度梯形图 |
| **Phase 16** | 一键打包与自动化脚本 | `start_all.ps1`、`run_all_tests.ps1`，零依赖启动全套系统 |

---

## 四、资金管理与风控模型详解

### 1. 双层资金池架构
- **Active Bankroll（活跃本金）**：实际参与模拟下注的动态本金，默认初始 **10.00 USDC**，上限严格封顶为 **10.00 USDC**。
- **Locked Profit（锁定利润池）**：交易盈利剥离金库，**永远不用于再次下注**，确保“已落袋利润绝对安全”。
- **模式 B（Capital Recovery，默认推荐）**：
  - 若活跃本金发生回撤（例如亏损至 9.00 USDC），后续交易盈利优先补足活跃本金至 10.00 USDC 封顶；
  - 超过 10.00 USDC 的溢出盈利部分，**100% 自动划入 Locked Profit 锁定**。
- **模式 A（Profit Isolation）**：
  - 无论活跃本金当前是否处于回撤状态，每一笔平仓盈利的净收益均直接 100% 锁定入 Locked Profit。

### 2. 四重硬性风控熔断器（Circuit Breakers）
| 风控项 | 阈值参数 | 触发机制与处理动作 |
| :--- | :--- | :--- |
| **本金最低下限** | `bankroll.minimum = 2.0 USDC` | 若活跃本金 $\le 2$ USDC，全系统自动停机，禁止一切新开仓 |
| **单日最大亏损** | `risk.daily_loss_limit = 2.0 USDC` | 当日累计净亏损达到 2 USDC，触发熔断，当日禁止开仓 |
| **最大回撤限制** | `risk.max_drawdown = 20%` | 从本金历史峰值回撤达到 20%，系统暂停开仓 |
| **连续亏损冷静** | `risk.max_consecutive_losses = 5 次` | 连续遭遇 5 次亏损，自动进入 **30 分钟策略冷静期** |

---

## 五、37 维量化特征矩阵速查

系统为每一个资产（BTC, ETH, SOL）每 100ms 计算 37 维高精特征：
1. **多周期对数收益率 (6 维)**：`return_1s`, `return_3s`, `return_5s`, `return_10s`, `return_30s`, `return_60s`
2. **多周期已实现波动率 (4 维)**：`realized_vol_5s`, `realized_vol_10s`, `realized_vol_30s`, `realized_vol_60s`
3. **价格动力学速度与加速度 (3 维)**：`velocity_5s`, `velocity_15s`, `acceleration_5s_15s`
4. **当前开盘偏离特征 (3 维)**：`distance_from_open`, `distance_percent`, `distance_to_vol_ratio`
5. **周期时间状态 (3 维)**：`remaining_seconds`, `elapsed_seconds`, `time_decay_factor` ($\sqrt{T_{rem}/300}$)
6. **跨交易所价差离散度 (3 维)**：`spread_binance_okx`, `spread_binance_bybit`, `spread_binance_coinbase`
7. **资金流与累积成交差 CVD (4 维)**：`cvd_5s`, `cvd_15s`, `cvd_30s`, `cvd_60s`
8. **主动买卖订单失衡度 (4 维)**：`trade_imbalance_5s`, `trade_imbalance_15s`, `trade_imbalance_30s`, `trade_imbalance_60s`
9. **Polymarket 盘口微观结构 (7 维)**：`poly_obi_top5`, `poly_obi_top10`, `poly_obi_top20`, `poly_spread`, `poly_total_liquidity`, `poly_implied_prob`, `composite_price`

---

## 六、策略引擎与 7 重硬过滤器

系统在发出任何交易信号前，必须 **100% 串行通过 7 重硬性门控（Fail-Closed）**：
1. **Gate 1 - 新鲜度门控**：所有上游交易所数据延迟必须 $< 2000\text{ms}$，时钟漂移 $< 1000\text{ms}$。
2. **Gate 2 - 周期时间窗门控**：剩余时间必须在 $15\text{s} \le T_{\text{rem}} \le 285\text{s}$ 之间，过滤开盘第一秒与收盘抢跑异常。
3. **Gate 3 - 预测概率置信门控**：模型校准后胜率 $P \ge 70\%$。
4. **Gate 4 - 进场价格上限门控**：买入 Ask 价格不得超过 $0.85$（避免胜率过高但赔率过差的负期望交易）。
5. **Gate 5 - 盘口价差门控**：Polymarket 买卖价差必须 $\le 0.04$ USDC。
6. **Gate 6 - 盘口流动性门控**：订单簿对应方向挂单深度必须 $\ge 300.0$ USDC。
7. **Gate 7 - 资金风控熔断门控**：系统风控状态必须为 NORMAL，无连续亏损冷静期且活跃资金 $\ge \text{Stake}$。

通过硬性门控后，触发 **[0..100] 机会评分卡**：
- **胜率分 (30分)**：校准后概率越高得分越高；
- **净 Edge 分 (25分)**：预期毛收益扣除手续费 (1.2%) 与滑点 (0.5%) 后的净优势；
- **时间窗口分 (20分)**：剩余 60s ~ 180s 处于黄金博弈窗获得满分；
- **微观结构 OBI 分 (15分)**：订单簿深度失衡方向与预测方向一致获得加分；
- **动量与波动率分 (10分)**：短周期加速度与顺势波动率加分；
- **总分阈值**：低于 60 分自动判为 `SKIP`，60~70 为 Low，70~80 为 Medium，80~90 为 High，90+ 为 Very High。

---

## 七、真实订单簿深度穿透撮合（Section 21）

本系统坚决不采用“假设以挂单价全额成交”的虚假假设，而是严格根据实际订单簿深度进行逐层穿透：
1. **深度穿透（Depth-Walking Fill）**：遍历实际 Ask 订单簿档位，计算实际成交均价 $P_{\text{fill}}$；
2. **滑点惩罚**：$\text{Slippage} = P_{\text{fill}} - P_{\text{quote}}$；
3. **交易手续费扣除**：$\text{Fee} = \text{Stake} \times 1.2\%$；
4. **实际股份数计算**：$\text{Shares} = \frac{\text{Stake} - \text{Fee}}{P_{\text{fill}}}$。

---

## 八、全套 REST API 接口清单

| 类别 | 请求方式 | 接口端点 | 描述 |
| :--- | :--- | :--- | :--- |
| **系统与健康** | `GET` | `/api/v1/health` | 系统健康、安全守卫状态、交易所新鲜度 |
| | `GET` | `/api/v1/safety` | 严格安全锁签名与实盘禁用状态校验 |
| | `GET` | `/api/v1/config` | 运行时脱敏只读系统配置 |
| | `GET` | `/api/v1/events` | 审计安全事件与风控日志流 |
| **行情与微观** | `GET` | `/api/v1/collector/prices` | Binance / OKX / Bybit / Coinbase 实时行情 |
| | `GET` | `/api/v1/composite/price/{asset}` | 稳健综合价格指数、收益率与已实现波动率 |
| | `GET` | `/api/v1/polymarket/markets` | Polymarket 5分钟周期活跃市场列表 |
| | `GET` | `/api/v1/polymarket/book/{asset}` | 5分钟二元盘口完整深度、买卖价差与 OBI |
| **特征与模型** | `GET` | `/api/v1/features/latest/{asset}`| 37 维实时高精量化特征快照 |
| | `GET` | `/api/v1/models/prediction/{asset}`| 逻辑回归与校准后胜率预测、特征贡献度 |
| | `GET` | `/api/v1/dataset/summary` | 本地 SQLite 数据集样本统计与正负平衡度 |
| **策略与模拟** | `GET` | `/api/v1/strategy/signals/latest` | 最新交易决策信号（`BUY_UP` / `BUY_DOWN` / `SKIP`） |
| | `GET` | `/api/v1/paper/bankroll` | 活跃本金、锁定利润与熔断状态 |
| | `GET` | `/api/v1/risk/status` | 每日亏损限额、峰值回撤与连亏计数 |
| | `GET` | `/api/v1/paper/positions/active` | 当前活跃模拟持仓与未实现浮盈 |
| | `GET` | `/api/v1/paper/results` | 模拟订单历史交割结算盈亏明细 |
| | `GET` | `/api/v1/paper/statistics` | 胜率、盈亏比、总收益等综合统计 |
| **回测引擎** | `POST` | `/api/v1/backtest/run` | 触发事件驱动历史回测模拟 |
| | `GET` | `/api/v1/backtest/latest` | 获取最新回测报告与净值曲线 |
| | `GET` | `/api/v1/backtest/history` | 历史回测运行记录列表 |
| **逐帧回放** | `POST` | `/api/v1/replay/start` | 启动历史行情逐帧回放引擎 |
| | `POST` | `/api/v1/replay/pause` | 暂停回放 |
| | `POST` | `/api/v1/replay/resume` | 继续回放 |
| | `POST` | `/api/v1/replay/step` | 单步逐帧步进（Step 1 Frame） |
| | `POST` | `/api/v1/replay/seek` | 进度条时间/帧索引精确定位 |
| | `POST` | `/api/v1/replay/speed` | 设置回放倍速（1x, 5x, 10x, 20x, 50x） |
| | `GET` | `/api/v1/replay/status` | 当前回放状态与当前帧诊断指标 |

---

## 九、Linux 服务器与宝塔面板 (aaPanel) 一键部署

系统原生完美支持任何 Linux 云服务器（Ubuntu / Debian / CentOS / Rocky / AlmaLinux）以及宝塔面板 (aaPanel)。

### 1. 原生 Linux / 宝塔终端一键安装 (最推荐)
在云服务器或宝塔「终端」中执行：
```bash
sudo bash install.sh
```
一键全自动安装系统编译库、Node.js v20 LTS 与 Rust 稳定版工具链，编译前后端生产包，并注册为 `systemd` 系统守护服务。

### 2. 全局命令行快捷管理
安装后系统自动注入全局管理命令：
```bash
polyquant status   # 查看服务运行状态、内存、PID、监听端口与心跳
polyquant log      # 实时查看行情采集、策略评估与模拟撮合日志
polyquant restart  # 一键重启量化服务
polyquant stop     # 停止量化服务
polyquant config   # 编辑调优量化参数
polyquant update   # 一键 git pull 拉取最新代码并热重构更新
```

### 3. 宝塔面板专属部署
详见完整图文实操文档：[Linux 与宝塔面板一键部署手册](docs/LINUX_AND_BAOTA_DEPLOYMENT.md)
- **宝塔防火墙**：安全 -> 防火墙放行 `8080` 端口；
- **域名绑定与 Nginx 反向代理**：网站设置 -> 反向代理，目标填 `http://127.0.0.1:8080`，复制预置的 [deploy/baota/nginx_reverse_proxy.conf](deploy/baota/nginx_reverse_proxy.conf) 配置文件即可开启 WebSocket 极速推流；
- **Supervisor 进程守护**：预置 [deploy/baota/supervisor_polyquant.ini](deploy/baota/supervisor_polyquant.ini)；
- **Docker 容器化一键启动**：`docker compose up -d`。

---

## 十、Windows 本地快速启动（开发者模式）

### 一键启动
在 Windows 下通过 PowerShell 执行：
```powershell
.\scripts\start_all.ps1
```
自动构建前端静态资源至 `frontend/dist`，启动后端服务，并在浏览器自动弹出 `http://127.0.0.1:8080` 控制台。

### 分步手动启动
#### 1. 启动后台服务
```bash
cargo run --manifest-path backend/Cargo.toml
```

#### 2. 启动前端 Vite 开发服务（可选）
```bash
cd frontend
npm install
npm run dev
```

---

## 十、自动化测试验证套件

系统拥有全套 14 组端到端集成测试，测试覆盖率达到 100%：
```powershell
.\scripts\run_all_tests.ps1
```
或直接运行 Cargo 测试：
```bash
cargo test -- --nocapture
```

测试覆盖清单：
- `phase1_integration`: 内核级安全拒绝、SQLite 15 表迁移、健康检查；
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
- `phase14_replay_integration`: 历史逐帧回放、单步前进、时间定位。

---

## 十一、免责声明与安全声明

1. 本系统仅供量化策略研究、模拟推演与学术回测验证使用。
2. 系统的 `SafetyGuard` 强制锁定实盘禁止状态，系统内不存在任何真实资金接口或私钥。
3. 严禁修改安全守卫代码用于任何形式的未授权真实交易，开发者不对任何个人衍生行为承担法律与财务责任。
