# Verify2Trade

가상화폐(BTCUSDT) 백테스팅 및 자동매매 시스템 검증 프로젝트.

## 개요
이 프로젝트는 특정 매매 기법(1시간봉 25 이평선 터치 전략 등)이 유효한지 벡테스트를 통해 검증하고, 검증된 전략을 실전 자동매매에 적용할 수 있도록 지원하는 시스템입니다.
Hive Partitioned Parquet 데이터 구조와 DuckDB/Polars를 활용하여 대용량 틱/분봉 데이터를 효율적으로 처리하며, Svelte 기반의 웹 인터페이스를 통해 백테스트 실행 및 결과를 시각화합니다.

## 기술 스택

### Backend
- **Language**: Rust
- **Web Framework**: Axum
- **Data Processing**: Polars, DuckDB
- **Database**: SQLite (Metadata), DuckDB (Parquet query)
- **Charting**: Plotters
- **AI Integration**: Ollama (Local), OpenAI Compatible API (Remote)

### Frontend
- **Framework**: Svelte 5 (Vite)
- **Language**: TypeScript
- **Styling**: TailwindCSS
- **Communication**: REST API, SSE (Server-Sent Events)

### Infrastructure
- **Container**: Podman Compose
- **Data Storage**: Local Filesystem (Parquet Hive Partitioning)

## 프로젝트 구조
```
Verify2Trade/
├── backend/          # Rust Backend Application
├── frontend/         # Svelte Frontend Application
├── cryptodata/       # Market Data (Parquet)
├── config.yaml       # System Configuration
├── .env             # Secrets (API Keys, etc.)
└── docker-compose.yml # Podman/Docker Compose definition
```

## 시작하기

### 필수 요구사항
- Rust (latest)
- Node.js (LTS)
- Podman or Docker
- Ollama (for VLM features)

### 설치 및 실행

1. **환경 변수 설정**
   `.env.example`을 `.env`로 복사하고 필요한 키를 입력합니다.
   ```bash
   cp .env.example .env
   ```

2. **백엔드 실행**
   ```bash
   cd backend
   cargo run
   ```

3. **프론트엔드 실행**
   ```bash
   cd frontend
   npm install
   npm run dev
   ```

## API 명세
`API.md` 파일을 참조하세요.

## 라이선스
MIT
