# 🔭 Vantage: Spec for Market Backtesting Engine

## 👤 User Story
"As a Quantitative Trader running algorithms on Duke, I want a Market Backtesting Engine, so that I can validate my trading strategies against historical tick data before risking real capital."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke is a general-purpose VM, but we want to position it for high-performance financial workloads. Quant traders need to replay years of historical market data (ticks, bars) through their Java-based trading strategies to measure profitability and risk. Without a dedicated backtesting engine that handles time-series data efficiently, traders will use Python (pandas/Backtrader) or other platforms. Building a high-performance backtesting engine proves Duke's capability to handle large-scale, stateful, and time-sensitive financial simulations, attracting the lucrative fintech/quant market.

## 📈 Metric Definition
Success = A user can load a 1GB CSV of historical tick data into the engine and backtest a simple moving average crossover strategy in under 5 seconds, outputting a final P&L (Profit and Loss) report.

## 🔍 Gap Analysis
- **Current State:** Duke can execute basic Java code and we have some file I/O planned, but there is no domain-specific engine for managing financial time-series data or simulating market execution (fills, slippage, commission).
- **Market/Standard Lib:** Python dominates this space with libraries like Backtrader and Zipline. Java has some libraries (like JForex or custom in-house engines), but they are often heavy.
- **The Gap:** We need an embedded engine that can read market data (CSV/binary), simulate a broker (managing order state, executing trades against historical prices), and track portfolio performance, exposed via a clean Java API.

## ✅ Acceptance Criteria
- Must support loading historical tick data (Timestamp, Price, Volume) from CSV files.
- Must provide a Java API to define a trading strategy (e.g., `onTick(Tick tick)` callback).
- Must simulate basic market execution (Market and Limit orders).
- Must track portfolio state (Cash balance, open positions, unrealized P&L).
- Must generate a final performance report (Total Return, Max Drawdown).
- Must handle NaN or missing data gracefully without panicking.

## 🚫 Out of Scope
- Real-time live market execution (Phase 2).
- Advanced order types (Stop-loss, Trailing-stop, Iceberg).
- Options/Derivatives pricing models.
