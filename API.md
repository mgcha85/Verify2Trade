# API Documentation

## Base URL
`http://localhost:3000`

## Endpoints

### 1. Health Check
- **GET** `/health`
- **Description**: 서버 상태 확인
- **Response**: `200 OK`

### 2. Backtest

#### Start Backtest
- **POST** `/api/backtest/run`
- **Description**: 새로운 백테스트 실행
- **Request Body**:
  ```json
  {
    "strategy": "ma_touch_reject",
    "symbol": "BTCUSDT",
    "start_date": "2023-01-01",
    "end_date": "2023-12-31",
    "params": {
      "ma_period": 25,
      "timeframe": "1h"
    }
  }
  ```
- **Response**:
  ```json
  {
    "backtest_id": "bt_123456789",
    "status": "started"
  }
  ```

#### Backtest Progress (SSE)
- **GET** `/api/backtest/progress/:backtest_id`
- **Description**: 백테스트 진행 상황 실시간 수신 (Server-Sent Events)
- **Response Event**:
  ```json
  {
    "progress": 45,
    "current_date": "2023-05-12",
    "status": "running"
  }
  ```

#### Get Backtest Result
- **GET** `/api/backtest/result/:backtest_id`
- **Description**: 완료된 백테스트 결과 조회
- **Response**:
  ```json
  {
    "id": "bt_123456789",
    "symbol": "BTCUSDT",
    "trades": [
      {
        "type": "short",
        "entry_price": 45000,
        "exit_price": 44000,
        "profit_percent": 2.2,
        "profit_abs": 220,
        "entry_time": "2023-05-12T10:00:00",
        "exit_time": "2023-05-12T14:00:00"
      }
    ],
    "statistics": {
      "total_return": 15.5,
      "win_rate": 65.2,
      "max_drawdown": -5.4
    }
  }
  ```

### 3. Market Data

#### Get Symbols
- **GET** `/api/data/symbols`
- **Description**: 사용 가능한 심볼 목록 조회
- **Response**: `["BTCUSDT", "ETHUSDT"]`

## 예제 (curl)

```bash
# 백테스트 실행
curl -X POST http://localhost:3000/api/backtest/run \
  -H "Content-Type: application/json" \
  -d '{
    "strategy": "ma_touch_reject",
    "symbol": "BTCUSDT",
    "start_date": "2023-01-01",
    "end_date": "2023-12-31"
  }'
```
