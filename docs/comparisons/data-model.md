# Data Model Comparison: UTXO vs Account

## Overview

| Model | Chains | State表現 | 並列性 |
|-------|--------|----------|--------|
| UTXO | Bitcoin, Kaspa, Core | 未使用出力の集合 | 高 |
| Account | Ethereum, Solana | アカウント残高マップ | 低 |

## UTXO Model (Unspent Transaction Output)

### 概念

```
┌─────────────────────────────────────────────────────┐
│                    Transaction                       │
├─────────────────────┬───────────────────────────────┤
│       Inputs        │          Outputs              │
├─────────────────────┼───────────────────────────────┤
│ ┌─────────────────┐ │ ┌───────────────────────────┐ │
│ │ Prev TX: abc... │ │ │ Value: 5 BTC             │ │
│ │ Output Index: 0 │ │ │ Lock: PubKeyHash(Bob)    │ │
│ │ Signature: ...  │ │ └───────────────────────────┘ │
│ └─────────────────┘ │ ┌───────────────────────────┐ │
│ ┌─────────────────┐ │ │ Value: 45 BTC (お釣り)    │ │
│ │ Prev TX: def... │ │ │ Lock: PubKeyHash(Alice)  │ │
│ │ Output Index: 1 │ │ └───────────────────────────┘ │
│ └─────────────────┘ │                               │
└─────────────────────┴───────────────────────────────┘
```

### 特徴

**メリット:**
- 並列検証が容易（TXは独立）
- プライバシー（アドレス使い捨て可能）
- 監査しやすい（トレース可能）

**デメリット:**
- スマートコントラクトが複雑
- お釣り管理が必要
- 状態が暗黙的

### 実装 (Core)

```rust
// core/src/execution/transaction.rs

pub struct TXInput {
    pub txid: Vec<u8>,      // 参照するTXのID
    pub vout: usize,        // 出力インデックス
    pub signature: Vec<u8>, // 署名
    pub pub_key: Vec<u8>,   // 公開鍵
}

pub struct TXOutput {
    pub value: i32,             // 金額
    pub pub_key_hash: Vec<u8>,  // ロック先アドレスハッシュ
}
```

## Account Model

### 概念

```
┌─────────────────────────────────────┐
│         World State                  │
├─────────────────────────────────────┤
│  Address A:                          │
│    Balance: 100 ETH                  │
│    Nonce: 5                          │
│    Code: (empty = EOA)               │
│                                      │
│  Address B (Contract):               │
│    Balance: 50 ETH                   │
│    Nonce: 0                          │
│    Code: 0x6080604052...             │
│    Storage:                          │
│      slot[0] = 0x123...              │
│      slot[1] = 0x456...              │
└─────────────────────────────────────┘
```

### 特徴

**メリット:**
- 直感的（銀行口座のよう）
- スマートコントラクトが自然
- 状態が明示的

**デメリット:**
- 並列実行が難しい（状態競合）
- Nonce管理が必要
- リプレイ攻撃対策が必要

### 実装 (Ethereum)

```rust
// implementations/ethereum/src/state.rs

pub struct Account {
    pub nonce: u64,           // トランザクション数
    pub balance: u128,        // 残高 (wei)
    pub code_hash: Vec<u8>,   // コントラクトコード
    pub storage_root: Vec<u8>, // ストレージツリー
}
```

## 比較表

| 観点 | UTXO | Account |
|------|------|---------|
| 残高確認 | 全UTXOをスキャン | アカウント参照 |
| 送金 | 入力選択 + 出力作成 | 残高更新 |
| 並列処理 | ◎ (TX独立) | △ (状態共有) |
| スマコン | △ (複雑) | ◎ (自然) |
| プライバシー | ◎ (アドレス変更) | △ (固定アドレス) |
| 軽量クライアント | △ (UTXO証明) | ◎ (状態証明) |

## 実装ファイル

| Model | File |
|-------|------|
| UTXO (Core) | `core/src/execution/utxo.rs` |
| UTXO (Core) | `core/src/execution/transaction.rs` |
| Account (Ethereum) | `implementations/ethereum/src/state.rs` |
