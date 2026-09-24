# Polymarket 5-Minute Crypto Up/Down Quant System
### 高精度量化预测、概率校准与真实盘口模拟交易系统

![Mode](https://img.shields.io/badge/Mode-Paper_Trading_Only-brightgreen)
![Rust](https://img.shields.io/badge/Rust-1.98+-orange)
![License](https://img.shields.io/badge/License-Proprietary-blue)

---

## 一、系统核心定位与设计目标

本项目用于深入研究与量化验证 **Polymarket** 上的 **BTC / ETH / SOL 5 分钟 Up/Down 二元预测市场**。
系统遵循工业级量化工程规范开发，坚持以 **“真实可运行、数据零泄漏、真实盘口撮合、长期可验证”** 为最高原则。

### 核心安全准则（Zero Real Money Risk）
- **硬性禁止真实交易**：系统底层设计 `SafetyGuard` 强制断言，`real_trading_enabled` 必须恒为 `false`。
- **Fail-Closed 保护**：行情断连、数据 stale、时钟漂移、盘口异常时自动停止下达模拟订单。
- **双层资金管理模式**：Active Bankroll（受控风险本金）与 Locked Profit（锁定利润完全隔离），杜绝盲目复利风险。

---

## 二、系统架构总览

```
Poly量化/
  ├── Cargo.toml                       # 根目录 Cargo 工作空间配置
  ├── config/
  │    └── config.yaml                 # 核心系统配置（资金、策略、风控、撮合、新鲜度）
  ├── .env.example                     # 环境变量范本
  ├── backend/                         # Rust 高性能实时核心服务
  │    ├── Cargo.toml
  │    ├── migrations/
  │    │    └── 0001_initial_schema.sql# 数据库 Schema 迁移脚本
  │    ├── src/
  │    │    ├── lib.rs
  │    │    ├── main.rs                # 服务主入口与优雅停机
  │    │    ├── safety/                # 底层内核安全守卫 (SafetyGuard)
  │    │    ├── config/                # 严格配置加载器与校验器
  │    │    ├── types/                 # 统一领域模型 (MarketTick, Prediction, Bankroll 等)
  │    │    ├── db/                    # 数据库连接池 (SQLite WAL) 与仓储
  │    │    └── api/                   # Axum HTTP REST & 状态监控端点
  │    └── tests/
  │         └── phase1_integration.rs  # Phase 1 端到端集成测试
  ├── frontend/                        # 现代化 Web Dashboard（计划中）
  ├── ml/                              # 机器学习训练、校准与特征导出（计划中）
  └── data/                            # 本地 SQLite 时序数据存储
```

---

## 三、资金管理与风控模型

### 1. Active Bankroll 与 Locked Profit
- **Active Bankroll**：真正参与模拟下注的动态本金（默认初始 10 USDC，上限 Cap 10 USDC）。
- **Locked Profit**：交易盈利剥离池，**永远不用于再次下注**。
- **模式 A (Profit Isolation)**：单笔盈利后，本金归还 Active Bankroll，所有净利润进入 Locked Profit。
- **模式 B (Capital Recovery，默认推荐)**：若本金发生回撤（如 10U → 9U），后续盈利优先补回本金至 Cap（10U），溢出部分全额锁定入 Locked Profit。

### 2. 多重风控熔断机制
| 参数项 | 默认阈值 | 作用描述 |
| :--- | :--- | :--- |
| `bankroll.minimum` | `2.0 USDC` | 若本金 $\le 2$ USDC，自动停止所有新交易 |
| `risk.daily_loss_limit` | `2.0 USDC` | 当日累计亏损达到限制，当日自动停机 |
| `risk.max_consecutive_losses` | `5 次` | 连续亏损 5 笔进入 30 分钟策略冷静期 |
| `risk.max_drawdown` | `20%` | 峰值回撤达到 20% 自动暂停模拟下单 |

---

## 四、数据库模型 (Section 28 标准)

系统底层由 15 张高度规范化的数据表组成：
1. `markets`: 5 分钟周期市场元数据（开始/结束时间、开盘价、结算价、状态）
2. `market_ticks`: Polymarket 盘口深度与流动性快照
3. `exchange_ticks`: Binance, OKX, Bybit, Coinbase 跨交易所毫秒级行情
4. `orderbook_snapshots`: 订单簿深度与 Top5/10/20 OBI 特征
5. `trades`: 主流交易所与 Polymarket 逐笔成交及主动买卖方向
6. `features`: 实时计算的价差、波动率、微观结构特征
7. `predictions`: 模型输出概率、公允价值、净 Edge、决策日志
8. `paper_orders`: 模拟委托订单与真实深度撮合记录
9. `paper_positions`: 模拟持仓追踪
10. `paper_results`: 结算盈亏、资金变动归因分析
11. `bankroll_history`: 资金账户毫秒级完整审计流水
12. `strategy_versions`: 策略版本参数与哈希版本号
13. `model_versions`: 机器学习模型版本与训练度量
14. `backtest_runs`: 历史回测运行记录
15. `system_events`: 系统启动、熔断告警、Fail-closed 事件追踪

---

## 五、快速开始

### 1. 编译系统
```bash
cargo build
```

### 2. 运行单元与集成测试
```bash
cargo test
cargo test --test phase1_integration
```

### 3. 启动实时核心服务
```bash
cargo run
```

### 4. 验证服务状态
```bash
# 健康状态与安全守卫确认
curl http://127.0.0.1:8080/api/v1/health

# 安全锁状态
curl http://127.0.0.1:8080/api/v1/safety

# 当前资金状态
curl http://127.0.0.1:8080/api/v1/paper/bankroll

# 系统审计日志
curl http://127.0.0.1:8080/api/v1/events
```

---

## 六、开发阶段推进计划

- [x] **Phase 1: 项目骨架、数据库 Schema、配置系统、底层安全守卫、基础 API**
- [ ] **Phase 2: Binance / OKX / Bybit / Coinbase 实时 WebSocket 行情采集**
- [ ] **Phase 3: Polymarket 市场发现、CLOB 订单簿与结算结果采集**
- [ ] **Phase 4: 统一行情引擎与跨交易所复合价格计算**
- [ ] **Phase 5: Feature Engine（OBI、CVD、波动率、微观结构特征）**
- [ ] **Phase 6: 数据归档与时序历史数据库**
- [ ] **Phase 7: LightGBM / XGBoost 基线预测模型**
- [ ] **Phase 8: 概率校准 (Isotonic Regression / Platt Scaling)**
- [ ] **Phase 9: Strategy Engine（EV 计算、多重过滤、动态打分）**
- [ ] **Phase 10: 深度遍历与滑点延迟 Paper Execution Engine**
- [ ] **Phase 11: 资金与风控引擎实时状态机**
- [ ] **Phase 12: 实时端到端 Paper Trading 闭环运行**
- [ ] **Phase 13: 历史回测引擎 (Backtest Engine)**
- [ ] **Phase 14: Tick 级高倍速行情回放引擎 (Replay Engine)**
- [ ] **Phase 15: 现代化 React + Vite 量化交易 Dashboard**
- [ ] **Phase 16: 长期运行稳定性与无偏样本验证**
- [ ] **Phase 17: 性能优化 (延迟降低与内存分析)**
- [ ] **Phase 18: 生产部署、自动化监控与告警**
