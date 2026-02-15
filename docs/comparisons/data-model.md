# Data Model Comparison: UTXO vs Account

## Overview

| Model | Chains | State表現 | 並列性 |
|-------|--------|----------|--------|
| UTXO | Bitcoin, Kaspa, Core | 未使用出力の集合 | 高 |
| Account | Ethereum | アカウント残高マップ | 低 |
| Account + Owner | Solana | アカウント + 所有者プログラム | 高 |
| Account + ABCI | Cosmos | アカウント + モジュール分離 | 中 |

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

## Account + Owner Model (Solana)

### 概念

```
┌─────────────────────────────────────────────────────┐
│              Solana Account                         │
├─────────────────────────────────────────────────────┤
│  lamports: u64         (残高: 1 SOL = 1e9 lamports)│
│  data: Vec<u8>         (プログラムが使う任意データ) │
│  owner: Pubkey         (このアカウントを所有する    │
│                         プログラムのアドレス)       │
│  executable: bool      (プログラムかどうか)        │
│  rent_epoch: u64       (レント徴収エポック)        │
└─────────────────────────────────────────────────────┘
```

### 所有権モデル

```
System Program (11111...1)
    │
    ├── owns → User Wallet A (lamports のみ変更可)
    └── owns → User Wallet B

Token Program (TokenkegQfeZyi...)
    │
    ├── owns → Token Mint (トークン定義)
    └── owns → Token Account (トークン残高)
```

**ルール:**
- owner プログラムのみがアカウントの data を変更可能
- 誰でも lamports を追加できる（引き出しは owner のみ）
- executable アカウントは BPF ローダーが所有

### 特徴

**メリット:**
- 高並列性（アカウントアクセスを事前宣言）
- プログラム間の明確な境界（所有権）
- 状態とロジックの分離

**デメリット:**
- アカウント管理が複雑
- レント（最低残高）が必要
- PDAs (Program Derived Addresses) の理解が必要

### 実装 (Solana)

```rust
// implementations/solana/src/account.rs

pub struct Account {
    pub lamports: u64,      // 残高
    pub data: Vec<u8>,      // 状態データ
    pub owner: Pubkey,      // 所有プログラム
    pub executable: bool,   // プログラムフラグ
    pub rent_epoch: u64,    // レントエポック
}
```

## Account + ABCI Model (Cosmos)

### 概念

Cosmos SDK はアカウントモデルを使用しつつ、ABCI でコンセンサスとアプリケーションを分離。

```
┌─────────────────────────────────────────────────────┐
│              CometBFT (Consensus)                   │
│  ・ブロック生成・検証                               │
│  ・P2Pネットワーク                                 │
└────────────────────┬────────────────────────────────┘
                     │ ABCI Interface
                     ▼
┌─────────────────────────────────────────────────────┐
│              Application (Cosmos SDK)               │
├─────────────────────────────────────────────────────┤
│  x/bank:    残高管理                               │
│  x/staking: バリデーター・ステーキング             │
│  x/gov:     ガバナンス投票                         │
│  x/ibc:     チェーン間通信                         │
└─────────────────────────────────────────────────────┘
```

### App Hash (状態ルート)

```
FinalizeBlock (TX実行)
    ↓
状態変更をストアに適用
    ↓
Commit → App Hash を計算
    ↓
App Hash が次ブロックのヘッダに含まれる
```

### 特徴

**メリット:**
- コンセンサスとアプリケーションの分離（モジュラー）
- 任意の言語でアプリケーション実装可能
- モジュールシステムで機能追加が容易

**デメリット:**
- ABCI 通信のオーバーヘッド
- アプリケーション側の状態管理が必要

### 実装 (Cosmos)

```rust
// implementations/cosmos/src/abci.rs

pub trait Application {
    fn init_chain(&mut self, req: RequestInitChain) -> ResponseInitChain;
    fn check_tx(&self, req: RequestCheckTx) -> ResponseCheckTx;
    fn finalize_block(&mut self, req: RequestFinalizeBlock) -> ResponseFinalizeBlock;
    fn commit(&mut self, req: RequestCommit) -> ResponseCommit;
}
```

## 比較表

| 観点 | UTXO | Account (ETH) | Account+Owner (SOL) | Account+ABCI (Cosmos) |
|------|------|---------------|---------------------|----------------------|
| 残高確認 | 全UTXOをスキャン | アカウント参照 | アカウント参照 | アカウント参照 |
| 送金 | 入力選択 + 出力作成 | 残高更新 | lamports 更新 | x/bank モジュール |
| 並列処理 | ◎ (TX独立) | △ (状態共有) | ◎ (事前宣言) | △ (順次) |
| スマコン | △ (複雑) | ◎ (EVM) | ◎ (プログラム) | ◎ (モジュール) |
| プライバシー | ◎ (アドレス変更) | △ (固定) | △ (固定) | △ (固定) |
| 状態とロジック | 結合 | 結合 | 分離 | ABCI分離 |

## 実装ファイル

| Model | File |
|-------|------|
| UTXO (Core) | `core/src/execution/utxo.rs` |
| UTXO (Core) | `core/src/execution/transaction.rs` |
| Account (Ethereum) | `implementations/ethereum/src/state.rs` |
| Account+Owner (Solana) | `implementations/solana/src/account.rs` |
| Runtime (Solana) | `implementations/solana/src/runtime.rs` |
| ABCI (Cosmos) | `implementations/cosmos/src/abci.rs` |
| Types (Cosmos) | `implementations/cosmos/src/types.rs` |
