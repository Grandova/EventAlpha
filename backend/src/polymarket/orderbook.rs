use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

use crate::types::{Asset, MarketSide, OrderBookLevel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenOrderBook {
    pub token_id: String,
    pub asset: Asset,
    pub side: MarketSide,
    pub bids: Vec<OrderBookLevel>, // sorted descending by price
    pub asks: Vec<OrderBookLevel>, // sorted ascending by price
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub mid: Option<f64>,
    pub spread: Option<f64>,
    pub total_bid_depth_usdc: f64,
    pub total_ask_depth_usdc: f64,
    pub obi_top5: f64,
    pub obi_top10: f64,
    pub obi_top20: f64,
    pub last_update_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketBookSummary {
    pub market_id: String,
    pub asset: Asset,
    pub up_book: TokenOrderBook,
    pub down_book: TokenOrderBook,
    pub implied_prob_up: f64,
    pub implied_prob_down: f64,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedFill {
    pub target_side: MarketSide,
    pub requested_stake_usdc: f64,
    pub quote_price: f64, // best ask at decision time
    pub avg_fill_price: f64,
    pub total_shares: f64,
    pub slippage: f64,
    pub levels_consumed: usize,
}

#[derive(Clone)]
pub struct PolymarketBookEngine {
    // (Asset, MarketSide) -> TokenOrderBook
    books: Arc<DashMap<(Asset, MarketSide), TokenOrderBook>>,
    // History of OBI samples for sliding window feature computation
    obi_history: Arc<DashMap<Asset, VecDeque<(i64, f64)>>>,
}

impl PolymarketBookEngine {
    pub fn new() -> Self {
        Self {
            books: Arc::new(DashMap::new()),
            obi_history: Arc::new(DashMap::new()),
        }
    }

    /// Update orderbook from L2 bids and asks
    pub fn update_book(
        &self,
        token_id: &str,
        asset: Asset,
        side: MarketSide,
        mut bids: Vec<OrderBookLevel>,
        mut asks: Vec<OrderBookLevel>,
        now_ms: i64,
    ) {
        // Sort bids descending by price
        bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
        // Sort asks ascending by price
        asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));

        let best_bid = bids.first().map(|l| l.price);
        let best_ask = asks.first().map(|l| l.price);

        let mid = match (best_bid, best_ask) {
            (Some(b), Some(a)) => Some((b + a) / 2.0),
            (Some(b), None) => Some(b),
            (None, Some(a)) => Some(a),
            (None, None) => None,
        };

        let spread = match (best_bid, best_ask) {
            (Some(b), Some(a)) => Some((a - b).max(0.0)),
            _ => None,
        };

        let bid_depth_usdc: f64 = bids.iter().map(|l| l.price * l.size).sum();
        let ask_depth_usdc: f64 = asks.iter().map(|l| l.price * l.size).sum();

        let obi_top5 = Self::calc_obi(&bids, &asks, 5);
        let obi_top10 = Self::calc_obi(&bids, &asks, 10);
        let obi_top20 = Self::calc_obi(&bids, &asks, 20);

        if side == MarketSide::Up {
            let mut hist = self.obi_history.entry(asset).or_insert_with(VecDeque::new);
            hist.push_back((now_ms, obi_top5));
            // Keep at most 120 seconds of OBI samples
            while let Some(&(ts, _)) = hist.front() {
                if now_ms - ts > 120_000 {
                    hist.pop_front();
                } else {
                    break;
                }
            }
        }

        let book = TokenOrderBook {
            token_id: token_id.to_string(),
            asset,
            side,
            bids,
            asks,
            best_bid,
            best_ask,
            mid,
            spread,
            total_bid_depth_usdc: bid_depth_usdc,
            total_ask_depth_usdc: ask_depth_usdc,
            obi_top5,
            obi_top10,
            obi_top20,
            last_update_ms: now_ms,
        };

        self.books.insert((asset, side), book);
    }

    /// Calculate Order Book Imbalance (OBI) for top N levels
    pub fn calc_obi(bids: &[OrderBookLevel], asks: &[OrderBookLevel], levels: usize) -> f64 {
        let bid_vol: f64 = bids.iter().take(levels).map(|l| l.size).sum();
        let ask_vol: f64 = asks.iter().take(levels).map(|l| l.size).sum();

        let total = bid_vol + ask_vol;
        if total > 0.0 {
            (bid_vol - ask_vol) / total
        } else {
            0.0
        }
    }

    /// Get sliding window OBI for specified seconds (e.g. 1s, 3s, 5s, 10s, 30s)
    pub fn get_window_obi(&self, asset: Asset, window_seconds: i64) -> f64 {
        let Some(hist) = self.obi_history.get(&asset) else {
            return 0.0;
        };

        let now_ms = chrono::Utc::now().timestamp_millis();
        let cutoff = now_ms - (window_seconds * 1000);

        let relevant: Vec<f64> = hist
            .iter()
            .filter(|(ts, _)| *ts >= cutoff)
            .map(|(_, obi)| *obi)
            .collect();

        if relevant.is_empty() {
            0.0
        } else {
            relevant.iter().sum::<f64>() / relevant.len() as f64
        }
    }

    /// Retrieve combined UP and DOWN orderbooks for an asset
    pub fn get_market_summary(&self, market_id: &str, asset: Asset) -> Option<MarketBookSummary> {
        let up_book = self.books.get(&(asset, MarketSide::Up))?.clone();
        let down_book = self.books.get(&(asset, MarketSide::Down))?.clone();

        let implied_up = up_book.mid.or(up_book.best_ask).unwrap_or(0.5).clamp(0.01, 0.99);
        let implied_down = down_book.mid.or(down_book.best_ask).unwrap_or(1.0 - implied_up).clamp(0.01, 0.99);

        Some(MarketBookSummary {
            market_id: market_id.to_string(),
            asset,
            up_book,
            down_book,
            implied_prob_up: implied_up,
            implied_prob_down: implied_down,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        })
    }

    /// Section 21: Walk the orderbook depth to simulate realistic trade execution fill
    pub fn simulate_buy_fill(
        &self,
        asset: Asset,
        side: MarketSide,
        stake_usdc: f64,
    ) -> Result<SimulatedFill, String> {
        let book = self
            .books
            .get(&(asset, side))
            .ok_or_else(|| format!("No orderbook found for {:?} {:?}", asset, side))?;

        if book.asks.is_empty() {
            return Err("Orderbook asks are empty; cannot simulate fill".to_string());
        }

        let best_ask = book.best_ask.unwrap_or(book.asks[0].price);
        let mut remaining_stake = stake_usdc;
        let mut total_shares = 0.0;
        let mut levels_consumed = 0;

        for ask in &book.asks {
            if ask.price <= 0.0 {
                continue;
            }
            levels_consumed += 1;
            let level_capacity_usdc = ask.price * ask.size;

            if remaining_stake <= level_capacity_usdc {
                // Partial fill of this ask level
                let shares_bought = remaining_stake / ask.price;
                total_shares += shares_bought;
                remaining_stake = 0.0;
                break;
            } else {
                // Completely absorb this ask level and proceed to higher price level
                total_shares += ask.size;
                remaining_stake -= level_capacity_usdc;
            }
        }

        if remaining_stake > 0.001 {
            return Err(format!(
                "Insufficient orderbook liquidity: {:.2} USDC unfulfilled out of {:.2} USDC requested",
                remaining_stake, stake_usdc
            ));
        }

        let avg_fill_price = stake_usdc / total_shares;
        let slippage = (avg_fill_price - best_ask).max(0.0);

        Ok(SimulatedFill {
            target_side: side,
            requested_stake_usdc: stake_usdc,
            quote_price: best_ask,
            avg_fill_price,
            total_shares,
            slippage,
            levels_consumed,
        })
    }

    /// Seed realistic orderbook for asset when bootstrapping or fallback
    pub fn seed_fallback_ladder(&self, asset: Asset, mid_up: f64, now_ms: i64) {
        let p_up = mid_up.clamp(0.05, 0.95);
        let p_down = 1.0 - p_up;

        let up_bids = vec![
            OrderBookLevel { price: (p_up - 0.01).max(0.01), size: 100.0 },
            OrderBookLevel { price: (p_up - 0.02).max(0.01), size: 250.0 },
            OrderBookLevel { price: (p_up - 0.03).max(0.01), size: 500.0 },
        ];
        let up_asks = vec![
            OrderBookLevel { price: (p_up + 0.01).min(0.99), size: 100.0 },
            OrderBookLevel { price: (p_up + 0.02).min(0.99), size: 250.0 },
            OrderBookLevel { price: (p_up + 0.03).min(0.99), size: 500.0 },
        ];

        let down_bids = vec![
            OrderBookLevel { price: (p_down - 0.01).max(0.01), size: 100.0 },
            OrderBookLevel { price: (p_down - 0.02).max(0.01), size: 250.0 },
            OrderBookLevel { price: (p_down - 0.03).max(0.01), size: 500.0 },
        ];
        let down_asks = vec![
            OrderBookLevel { price: (p_down + 0.01).min(0.99), size: 100.0 },
            OrderBookLevel { price: (p_down + 0.02).min(0.99), size: 250.0 },
            OrderBookLevel { price: (p_down + 0.03).min(0.99), size: 500.0 },
        ];

        self.update_book(&format!("{}_UP", asset), asset, MarketSide::Up, up_bids, up_asks, now_ms);
        self.update_book(&format!("{}_DOWN", asset), asset, MarketSide::Down, down_bids, down_asks, now_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_depth_and_obi() {
        let engine = PolymarketBookEngine::new();
        let bids = vec![
            OrderBookLevel { price: 0.60, size: 200.0 },
            OrderBookLevel { price: 0.59, size: 300.0 },
        ];
        let asks = vec![
            OrderBookLevel { price: 0.61, size: 100.0 },
            OrderBookLevel { price: 0.62, size: 200.0 },
        ];

        engine.update_book("BTC_UP", Asset::BTC, MarketSide::Up, bids, asks, 1700000000);

        let book = engine.books.get(&(Asset::BTC, MarketSide::Up)).unwrap();
        assert_eq!(book.best_bid, Some(0.60));
        assert_eq!(book.best_ask, Some(0.61));
        assert_eq!(book.mid, Some(0.605));
        assert!((book.spread.unwrap() - 0.01).abs() < 1e-6);

        // Total bid volume = 500, ask volume = 300 => OBI = (500 - 300) / 800 = 0.25
        assert!((book.obi_top5 - 0.25).abs() < 1e-4);
    }

    #[test]
    fn test_section21_orderbook_fill_simulation() {
        let engine = PolymarketBookEngine::new();
        // Ask ladder:
        // 0.61 -> size: 0.8196... (0.50 USDC capacity)
        // 0.62 -> size: 3.2258... (2.00 USDC capacity)
        let asks = vec![
            OrderBookLevel { price: 0.61, size: 0.50 / 0.61 },
            OrderBookLevel { price: 0.62, size: 2.00 / 0.62 },
        ];
        let bids = vec![OrderBookLevel { price: 0.60, size: 10.0 }];

        engine.update_book("BTC_UP", Asset::BTC, MarketSide::Up, bids, asks, 1700000000);

        // Buy 1.0 USDC:
        // 0.5 USDC @ 0.61 -> 0.81967 shares
        // 0.5 USDC @ 0.62 -> 0.80645 shares
        // Total shares = 1.62612 shares
        // Average price = 1.0 / 1.62612 = 0.614959 USDC
        let fill = engine.simulate_buy_fill(Asset::BTC, MarketSide::Up, 1.0).expect("Should fill");
        assert_eq!(fill.levels_consumed, 2);
        assert_eq!(fill.quote_price, 0.61);
        assert!((fill.avg_fill_price - 0.615).abs() < 0.001);
        assert!(fill.slippage > 0.004); // slippage = 0.6149 - 0.61 = ~0.0049
    }
}
