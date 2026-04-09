# 🔭 Vantage: Spec for Market Backtesting

## 👤 User Story
"As a Trader, I want to backtest against volatile markets, so that I can evaluate the robustness of my algorithms."

## ❓ The "So What?"
What business problem does this solve?
Testing algorithms against volatile data helps prevent financial losses during sudden market crashes.

## 📈 Metric Definition
Success = The engine handles injected extreme conditions and outputs the result without crashing.

## 🔍 Gap Analysis
- **Current State:** The backtesting engine often crashes when encountering NaN values.
- **The Gap:** We lack robust handling for extreme market anomalies (like NaN data points).

## ✅ Acceptance Criteria
- Must handle NaN data without panicking.
- Must output a CSV report.

## 🚫 Out of Scope
- Real-time execution (Phase 2).
