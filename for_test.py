import polars as pl
from datetime import datetime
import os


def transform_to_hive_partition(file_path, base_output_path):
    # 1. 파일 이름에서 심볼 추출 (예: BTCUSDT.parquet -> BTCUSDT)
    symbol = os.path.basename(file_path).split(".")[0]

    # 2. 데이터 스캔 (LazyFrame 사용으로 메모리 효율 극대화)
    lf = pl.scan_parquet(file_path)

    # 3. 데이터 전처리 및 정렬
    lf = (
        lf.with_columns(
            [
                # time이 숫자형(Int64)이면 datetime으로 변환, 이미 datetime이면 유지
                pl.col("time").cast(pl.Datetime).alias("timestamp")
                if lf.schema["time"] == pl.Int64
                else pl.col("time").alias("timestamp")
            ]
        )
        .with_columns(
            [
                pl.col("timestamp").dt.to_string("%Y").alias("year"),
                pl.col("timestamp").dt.to_string("%m").alias("month"),
            ]
        )
        .sort("timestamp")
    )

    # 4. 심볼별 루트 디렉토리 생성
    symbol_path = os.path.join(base_output_path, f"symbol={symbol}")

    # 5. 메모리에 데이터를 올린 후 파티션별로 저장
    # (참고: Polars는 현재 sink_parquet에서 직접적인 hive partitioning 쓰기를
    #  지원하는 과정에 있어, partition_by를 활용한 반복문이 가장 안정적입니다.)
    df = lf.collect()

    for (year, month), partition_df in df.partition_by(
        ["year", "month"], as_dict=True
    ).items():
        # 디렉토리 경로 생성: /cryptodata/symbol=BTCUSDT/year=2024/month=01/
        output_dir = os.path.join(symbol_path, f"year={year}", f"month={month}")
        os.makedirs(output_dir, exist_ok=True)

        # 파일 저장 (01.parquet)
        # 파티션 정보인 year, month 컬럼은 중복이므로 제거하고 저장할 수 있습니다.
        partition_df.drop(["year", "month"]).write_parquet(
            os.path.join(output_dir, "01.parquet"), compression="snappy"
        )
        print(f"Saved: {output_dir}/01.parquet")


# 사용 예시
input_parquet = "BTCUSDT.parquet"
output_root = "cryptodata"
transform_to_hive_partition(input_parquet, output_root)
