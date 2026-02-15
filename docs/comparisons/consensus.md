# Consensus Mechanisms Comparison

## Overview

| Blockchain | Consensus | Finality | Block Time | Energy |
|------------|-----------|----------|------------|--------|
| Bitcoin | PoW (SHA256) | Probabilistic (~6 blocks) | 10 min | High |
| Ethereum | PoS (Casper FFG) | Economic (~13 min) | 12 sec | Low |
| Kaspa | PoW + GHOSTDAG | Probabilistic (fast) | 1 sec | Medium |
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

## 実装ファイル

| Chain | File |
|-------|------|
| Core | `core/src/consensus/pow.rs` |
| Bitcoin | `implementations/bitcoin/src/consensus.rs` |
| Kaspa | `implementations/kaspa/src/consensus.rs` |
| Ethereum | `implementations/ethereum/src/consensus.rs` |
