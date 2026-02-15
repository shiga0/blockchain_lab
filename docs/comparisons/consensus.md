# Consensus Mechanisms Comparison

## Overview

| Blockchain | Consensus | Finality | Block Time | Energy |
|------------|-----------|----------|------------|--------|
| Bitcoin | PoW (SHA256) | Probabilistic (~6 blocks) | 10 min | High |
| Ethereum | PoS (Casper FFG) | Economic (~13 min) | 12 sec | Low |
| Kaspa | PoW + GHOSTDAG | Probabilistic (fast) | 1 sec | Medium |
| Solana | PoH + Tower BFT | Economic (~2.5 sec) | 400 ms | Low |
| Core (base) | PoW (SHA256) | Probabilistic | Configurable | - |

## Proof of Work (PoW)

### Bitcoin / Core

```
目標: hash < target
   ↓
nonce を変えながら SHA256 を繰り返す
   ↓
条件を満たすハッシュが見つかればブロック完成
```

**特徴:**
- 計算リソースを消費して信頼を構築
- 51%攻撃に耐性（計算量で保護）
- エネルギー消費が大きい

### 難易度調整

| Chain | 調整間隔 | アルゴリズム |
|-------|---------|-------------|
| Bitcoin | 2016ブロック (~2週間) | actual_time / expected_time |
| Core | なし（固定） | - |

## Proof of Stake (PoS)

### Ethereum (Casper FFG)

```
バリデーター（32 ETH ステーク）
   ↓
ランダムにブロック提案者を選出 (RANDAO)
   ↓
他のバリデーターが投票 (attestation)
   ↓
2/3 以上の投票でチェックポイント正当化
   ↓
連続2回正当化で最終化 (finality)
```

**特徴:**
- ステークを担保に不正を防止（スラッシング）
- エネルギー効率が良い
- 経済的ファイナリティ

## GHOSTDAG (Kaspa)

```
複数の親ブロックを許可（DAG構造）
   ↓
Blue Set / Red Set に分類
   ↓
Blue Score で順序付け
   ↓
高スループット（1秒ブロック）でも安全
```

**特徴:**
- 並列ブロック生成が可能
- オーファンブロックなし
- 高TPS（トランザクション/秒）

## Proof of History + Tower BFT (Solana)

### Proof of History (PoH)

```
SHA-256 ハッシュチェーンによる時間証明 (VDF)
   ↓
hash₀ → SHA256(hash₀) → hash₁ → SHA256(hash₁) → ...
   ↓
イベント（トランザクション）をチェーンに混入
   ↓
hash_n → SHA256(hash_n || event) → hash_{n+1}
```

**PoH の役割:**
- 時間の経過を暗号学的に証明
- コンセンサス前の事前順序付け
- リーダーは PoH を生成し、バリデーターは検証のみ

### Tower BFT

```
バリデーターが投票 (slot に対して)
   ↓
投票はスタック構造 (最大32投票)
   ↓
ロックアウトが指数的に増加 (2^depth slots)
   ↓
フォーク切り替えコストが指数的に増大
   ↓
2/3+ stake で実質的ファイナリティ
```

**投票ロックアウト:**
```
Depth 0: lockout = 2 slots   (0.8秒)
Depth 1: lockout = 4 slots   (1.6秒)
Depth 2: lockout = 8 slots   (3.2秒)
...
Depth 31: lockout = 2^32 slots (~54年)
```

**特徴:**
- PoH が共通時計として機能（メッセージ交換削減）
- 経済的ファイナリティ（ロールバックコスト増大）
- 高スループット（並列トランザクション実行 + 400ms スロット）

## 実装ファイル

| Chain | File |
|-------|------|
| Core | `core/src/consensus/pow.rs` |
| Bitcoin | `implementations/bitcoin/src/consensus.rs` |
| Kaspa | `implementations/kaspa/src/consensus.rs` |
| Ethereum | `implementations/ethereum/src/consensus.rs` |
| Solana | `implementations/solana/src/consensus.rs` |
