# Data Model Comparison: UTXO vs Account

## Overview

| Model | Chains | State表現 | 並列性 |
|-------|--------|----------|--------|
| UTXO | Bitcoin, Kaspa, Core | 未使用出力の集合 | 高 |
| Account | Ethereum | アカウント残高マップ | 低 |
| Account + Owner | Solana | アカウント + 所有者プログラム | 高 |
| Account + ABCI | Cosmos | アカウント + モジュール分離 | 中 |
| Multi-VM Subnet | Avalanche | チェーン別 (UTXO/Account/Custom) | 高 |
| Extended UTXO | Cardano | UTXO + Datum + Multi-Asset | 高 |
| Relay + Parachain | Polkadot | 共有セキュリティ + 独立状態 | 高 |
| Object-centric | Sui | オブジェクト所有権 + PTB | 超高 |
| Account + Resource | Aptos | アカウント + Move リソース | 超高 |

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

## Multi-VM Subnet Model (Avalanche)

### 概念

Avalanche は Subnet ごとに異なる VM を実行可能。各チェーンが独自のデータモデルを持つ。

```
┌─────────────────────────────────────────────────────────────┐
│                      Primary Network                         │
├─────────────────────────────────────────────────────────────┤
│  P-Chain (Platform VM):                                     │
│    ・バリデーター管理                                       │
│    ・Subnet 作成                                            │
│    ・UTXO モデル                                            │
│                                                             │
│  X-Chain (Avalanche VM - AVM):                              │
│    ・デジタルアセット作成・転送                             │
│    ・DAG 構造 + UTXO モデル                                 │
│                                                             │
│  C-Chain (Coreth - EVM):                                    │
│    ・スマートコントラクト                                   │
│    ・Account モデル (Ethereum互換)                          │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                      Custom Subnets                          │
├─────────────────────────────────────────────────────────────┤
│  Subnet A (Gaming):         Subnet B (DeFi):                │
│    Custom VM                  Subnet EVM                    │
│    独自データモデル            Account モデル               │
└─────────────────────────────────────────────────────────────┘
```

### VM タイプ別データモデル

| VM | データモデル | 用途 |
|----|-------------|------|
| Platform VM | UTXO | ステーキング、Subnet管理 |
| AVM | DAG + UTXO | アセット転送、NFT |
| EVM (Coreth) | Account | スマートコントラクト |
| Subnet EVM | Account (カスタム) | DeFi、ゲーム |
| Custom VM | 任意 | 独自ロジック |

### バリデーターセット

```rust
// implementations/avalanche/src/validator.rs

pub struct Validator {
    pub node_id: NodeId,          // ノードID
    pub stake: Stake,             // ステーク量
    pub is_active: bool,          // アクティブ状態
    pub start_time: u64,          // 検証開始時刻
    pub end_time: u64,            // 検証終了時刻
}

pub struct ValidatorSet {
    validators: HashMap<NodeId, Validator>,
    total_stake: Stake,
}
```

### 特徴

**メリット:**
- VM 選択の柔軟性（用途に最適な VM）
- Subnet ごとの独立したルール・手数料
- Primary Network のセキュリティを継承

**デメリット:**
- 複数 VM の複雑性
- Subnet バリデーターの募集が必要
- クロスチェーン操作が必要

## Extended UTXO Model (Cardano)

### 概念

Cardano の eUTXO は Bitcoin の UTXO を拡張し、スマートコントラクトを可能にする。

```
┌─────────────────────────────────────────────────────────────┐
│ Bitcoin UTXO:          │ Cardano eUTXO:                    │
├─────────────────────────────────────────────────────────────┤
│ TxOut {                │ TxOut {                           │
│   value: Satoshi       │   value: Value (multi-asset)      │
│   script: P2PKH        │   address: Address                │
│ }                      │   datum: Option<Datum>            │
│                        │   reference_script: Option<Script>│
│                        │ }                                 │
└─────────────────────────────────────────────────────────────┘
```

### eUTXO の拡張点

| 拡張 | 説明 |
|------|------|
| Datum | UTxO に任意データを添付（状態保存） |
| Redeemer | スクリプト実行時の入力データ |
| ScriptContext | トランザクション全体を参照可能 |
| Multi-Asset | ネイティブトークン（スマコン不要） |
| Reference Scripts | スクリプト参照による再利用 |

### Plutus バリデーター

```
validator(datum, redeemer, script_context) → Bool

┌─────────────────────────────────────────────────────────────┐
│ ScriptContext:                                              │
├─────────────────────────────────────────────────────────────┤
│ TxInfo:                                                     │
│   inputs: [(TxIn, TxOut)]      // 全入力                   │
│   outputs: [TxOut]             // 全出力                   │
│   mint: Value                  // 発行/焼却                │
│   valid_range: TimeRange       // 有効時間範囲             │
│   signatories: [PubKeyHash]    // 署名者                   │
│                                                             │
│ Purpose:                                                    │
│   Spending(TxIn)     // UTXO消費の検証                     │
│   Minting(PolicyId)  // トークン発行の検証                 │
└─────────────────────────────────────────────────────────────┘
```

### 実装 (Cardano)

```rust
// implementations/cardano/src/eutxo.rs

pub struct TxOut {
    pub address: Address,
    pub value: Value,              // Multi-asset
    pub datum: Option<Datum>,      // 状態データ
    pub reference_script: Option<ScriptHash>,
}

pub struct Value {
    pub coin: Lovelace,            // ADA
    pub multi_asset: HashMap<PolicyId, HashMap<AssetName, i64>>,
}

pub enum Datum {
    Hash(DatumHash),               // ハッシュのみ
    Inline(PlutusData),            // インラインデータ
}
```

### 特徴

**メリット:**
- UTXO の並列性を維持しつつスマートコントラクト可能
- ネイティブマルチアセット（スマコン不要で高速）
- 決定論的手数料（実行前に計算可能）
- スクリプトは純粋関数（状態変更なし）

**デメリット:**
- 状態マシンの設計が直感的でない
- 複数 UTxO 間の協調が複雑
- Datum サイズに制限

## Relay Chain + Parachain Model (Polkadot)

### 概念

Polkadot はリレーチェーンとパラチェーンの2層アーキテクチャ。パラチェーンは独自の状態を持ちながら、リレーチェーンからセキュリティを継承。

```
┌─────────────────────────────────────────────────────────────────┐
│                      RELAY CHAIN                                │
│  ・共有セキュリティ（最大1000バリデーター）                     │
│  ・パラチェーンブロックの検証・包含                            │
│  ・クロスチェーンメッセージ (XCM) の調整                       │
│  ・DOT ステーキング・ガバナンス                                │
└─────────────────────────────────────────────────────────────────┘
         │              │              │
   ┌─────┴─────┐  ┌─────┴─────┐  ┌─────┴─────┐
   │ Parachain │  │ Parachain │  │ Parachain │
   │    1000   │  │    1001   │  │    1002   │
   │  (Acala)  │  │(Moonbeam) │  │ (Astar)   │
   │           │  │           │  │           │
   │ Collators │  │ Collators │  │ Collators │
   │ 独自状態  │  │ EVM互換   │  │ WASM+EVM  │
   └───────────┘  └───────────┘  └───────────┘
```

### パラチェーン候補のライフサイクル

```
1. Collator がパラチェーンブロックを生成
   ┌────────────────────────────────────────┐
   │ Candidate {                            │
   │   para_id,                             │
   │   relay_parent,                        │
   │   pov_hash,        // Proof of Validity│
   │   head_data,       // 新しい状態ルート │
   │   commitments,     // メッセージ等     │
   │ }                                      │
   └────────────────────────────────────────┘

2. Backing Group (バリデーターグループ) が PoV を検証
   ┌────────────────────────────────────────┐
   │ BackedCandidate {                      │
   │   candidate,                           │
   │   validity_votes: [sig1, sig2],        │
   │   validator_indices: 0b110,            │
   │ }                                      │
   └────────────────────────────────────────┘

3. Availability (イレイジャー符号化 PoV を分散)
   ┌────────────────────────────────────────┐
   │ 2/3+ バリデーターがチャンクを保持      │
   │ AvailabilityBitfield: 0b11110111       │
   └────────────────────────────────────────┘

4. Inclusion (リレーブロックに包含)
```

### 共有セキュリティ vs ブリッジ

```
従来のブリッジ:           Polkadot 共有セキュリティ:
  Chain A ←→ Chain B        Relay Chain
       ↕                          ↕
     Bridge                  Para A ─ XCM ─ Para B
       ↕                          ↕
  独立した              同じバリデーターが
  セキュリティ            全パラを検証
```

### XCM (Cross-Consensus Messaging)

```
Location (アドレス指定):
  ../Parachain(1000)/Account(0x123...)

Instructions (命令):
  WithdrawAsset(DOT, 10)      // 資産を引き出し
  BuyExecution(weight)         // 実行手数料を支払い
  DepositAsset(DOT, dest)      // 宛先に預け入れ
  Transact(call_data)          // リモート呼び出し
```

### 実装 (Polkadot)

```rust
// implementations/polkadot/src/parachain.rs

pub struct CandidateDescriptor {
    pub para_id: ParaId,
    pub relay_parent: Hash,
    pub pov_hash: Hash,
    pub para_head: Hash,
    pub validation_code_hash: Hash,
}

pub struct BackedCandidate {
    pub candidate: CommittedCandidateReceipt,
    pub validity_votes: Vec<ValidityAttestation>,
    pub validator_indices: Vec<bool>,
}

pub struct AvailabilityBitfield(pub Vec<bool>);
```

### 特徴

**メリット:**
- 共有セキュリティ（全パラチェーンがリレーの信頼を継承）
- ネイティブクロスチェーン（ブリッジ不要の XCM）
- カスタムランタイム（Substrate で独自ロジック）
- スロットオークション（限られたパラチェーン枠を公平に配分）

**デメリット:**
- スロット獲得コスト（DOT ボンド必要）
- パラチェーン数に上限あり（〜100）
- 学習曲線（Substrate/WASM ランタイム）

## Object-centric Model (Sui)

### 概念

Sui はアカウントではなくオブジェクトを中心としたモデル。オブジェクトの所有権が実行パスを決定。

```
┌─────────────────────────────────────────────────────────────────┐
│                    Object Ownership Types                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  AddressOwner(address)     Shared { version }                   │
│  ┌───────────────────┐     ┌───────────────────┐               │
│  │ owner: 0xAlice    │     │ initial_ver: 5    │               │
│  │ version: 42       │     │ version: 100      │               │
│  └───────────────────┘     └───────────────────┘               │
│           │                         │                           │
│           ▼                         ▼                           │
│    ┌─────────────┐          ┌─────────────────┐                │
│    │  Fastpath   │          │   Consensus     │                │
│    │ (即時実行)   │          │  (順序付け)      │                │
│    └─────────────┘          └─────────────────┘                │
│                                                                 │
│  ObjectOwner(object_id)    Immutable                           │
│  ┌───────────────────┐     ┌───────────────────┐               │
│  │ parent: 0x123     │     │ frozen forever   │               │
│  │ (子オブジェクト)  │     │ anyone can read  │               │
│  └───────────────────┘     └───────────────────┘               │
└─────────────────────────────────────────────────────────────────┘
```

### Programmable Transaction Blocks (PTB)

```
PTB は複数の操作を1トランザクションで合成:

ProgrammableTransaction {
  inputs: [
    Pure(100),           // 金額
    Object(coin_ref),    // コイン
  ],
  commands: [
    SplitCoins(Input(1), [Input(0)]),   // コイン分割
    TransferObjects([Result(0)], dest), // 転送
  ],
}

Command 間で結果を受け渡し:
  Command 0 の出力 → Result(0) → Command 1 の入力
```

### 実装 (Sui)

```rust
// implementations/sui/src/object.rs

pub enum Owner {
    AddressOwner(SuiAddress),      // 単一所有者 (Fastpath)
    ObjectOwner(ObjectId),          // 親オブジェクト所有
    Shared { initial_shared_version }, // 共有 (Consensus必要)
    Immutable,                      // 不変 (誰でも参照可)
}

pub struct Object {
    pub data: Data,                 // Move object or Package
    pub owner: Owner,
    pub previous_transaction: TransactionDigest,
    pub storage_rebate: u64,
}

// implementations/sui/src/ptb.rs

pub enum Command {
    MoveCall { target, type_args, args },
    TransferObjects { objects, recipient },
    SplitCoins { coin, amounts },
    MergeCoins { target, sources },
    Publish { modules, dependencies },
}
```

### 特徴

**メリット:**
- 超高並列性（オブジェクト独立性による）
- Fastpath（Owned オブジェクトはコンセンサス不要）
- PTB（複数操作を原子的に合成）
- 決定論的ガス（実行前に計算可能）

**デメリット:**
- Move言語の学習コスト
- オブジェクトモデルの理解が必要
- Shared オブジェクトのボトルネック

## Account + Resource Model (Aptos)

### 概念

```
Aptos は Move 言語のリソースモデルを採用:

┌─────────────────────────────────────────────────────────────────────────┐
│  Account (0x1234...abcd)                                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Resources:                                                             │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ 0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>                │   │
│  │   └── coin: Coin { value: 1000000 }                             │   │
│  │   └── frozen: false                                             │   │
│  │   └── deposit_events: EventHandle { ... }                       │   │
│  │   └── withdraw_events: EventHandle { ... }                      │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ 0x1::account::Account                                           │   │
│  │   └── authentication_key: 0x1234...                             │   │
│  │   └── sequence_number: 42                                       │   │
│  │   └── coin_register_events: EventHandle { ... }                 │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  Modules (published code):                                              │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ my_module (bytecode)                                            │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘

リソース: (address, type) でグローバルストレージにアクセス
  - get_resource<T>(address) -> T
  - move_to(signer, resource)
  - move_from<T>(address) -> T
```

### Sui との違い

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Aptos vs Sui                                          │
├───────────────────────────────────┬─────────────────────────────────────┤
│            Aptos                  │              Sui                    │
├───────────────────────────────────┼─────────────────────────────────────┤
│ Account-centric                   │ Object-centric                      │
│ Resources under accounts          │ Objects are first-class             │
│ Global storage: (addr, type)      │ Object ID: unique identifier        │
│ Sequence number for replay        │ Object version for replay           │
│ Block-STM parallel execution      │ Fastpath + Mysticeti                │
│ All txs go through consensus      │ Owned objects skip consensus        │
│ Move (original)                   │ Move (Sui variant)                  │
└───────────────────────────────────┴─────────────────────────────────────┘
```

### Block-STM 並列実行

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Block-STM Execution Model                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Transaction Block: [tx_0, tx_1, tx_2, tx_3, ...]                      │
│                                                                         │
│  1. Optimistic Execution (全 TX を並列実行)                             │
│     ┌────┐  ┌────┐  ┌────┐  ┌────┐                                     │
│     │tx_0│  │tx_1│  │tx_2│  │tx_3│  ... (並列)                         │
│     └──┬─┘  └──┬─┘  └──┬─┘  └──┬─┘                                     │
│        │       │       │       │                                        │
│        ▼       ▼       ▼       ▼                                        │
│     MVHashMap に read/write を記録                                      │
│                                                                         │
│  2. Validation (read set の整合性チェック)                              │
│     tx_1 が tx_0 の書き込みを読んだ？                                  │
│     → 読んだ値の version が変わっていないか確認                        │
│                                                                         │
│  3. Conflict → Re-execution                                             │
│     incarnation++ → 再実行 → 再検証                                     │
│                                                                         │
│  4. All Validated → Commit                                              │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘

MVHashMap (Multi-Version HashMap):
  key = (address, path)
  value = [(txn_idx, incarnation, value), ...]

  読み取り: 自身より前の最新値を返す
  ESTIMATE: 依存関係マーカー（再実行中の値）
```

### トランザクション構造

```rust
// RawTransaction
pub struct RawTransaction {
    sender: Address,
    sequence_number: u64,  // replay protection
    payload: TransactionPayload,
    max_gas_amount: u64,
    gas_unit_price: u64,
    expiration_timestamp_secs: u64,
    chain_id: u8,
}

// TransactionPayload
enum TransactionPayload {
    EntryFunction {
        module: ModuleId,    // e.g., 0x1::coin
        function: String,    // e.g., "transfer"
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,  // BCS-encoded
    },
    ModuleBundle(Vec<Vec<u8>>),
    Multisig { ... },
}
```

### 特徴

**メリット:**
- 超高スループット (Block-STM による楽観的並列実行)
- Move 言語のリソース安全性
- 低レイテンシ (~1秒ファイナリティ)
- 複数署名スキーム対応 (Ed25519, Secp256k1, MultiSig)

**デメリット:**
- Move 言語の学習コスト
- 全トランザクションがコンセンサス経由 (Sui の Fastpath なし)
- 競合時の再実行オーバーヘッド

## 比較表

| 観点 | UTXO | Account (ETH) | Account+Owner (SOL) | Account+ABCI (Cosmos) | Multi-VM (AVAX) | eUTXO (ADA) | Parachain (DOT) | Object (SUI) | Resource (APT) |
|------|------|---------------|---------------------|----------------------|-----------------|-------------|-----------------|--------------|----------------|
| 残高確認 | 全UTXOをスキャン | アカウント参照 | アカウント参照 | アカウント参照 | チェーン依存 | UTXOスキャン | パラ状態参照 | オブジェクト参照 | リソース参照 |
| 送金 | 入力選択 + 出力作成 | 残高更新 | lamports 更新 | x/bank モジュール | VM依存 | 入力選択+出力 | XCM/直接 | TransferObjects | coin::transfer |
| 並列処理 | ◎ (TX独立) | △ (状態共有) | ◎ (事前宣言) | △ (順次) | ◎ (Subnet独立) | ◎ (TX独立) | ◎ (パラ独立) | ◎◎ (所有権分離) | ◎◎ (Block-STM) |
| スマコン | △ (複雑) | ◎ (EVM) | ◎ (プログラム) | ◎ (モジュール) | ◎ (EVM/Custom) | ◎ (Plutus) | ◎ (WASM) | ◎ (Move) | ◎ (Move) |
| プライバシー | ◎ (アドレス変更) | △ (固定) | △ (固定) | △ (固定) | チェーン依存 | ◎ (アドレス変更) | パラ依存 | △ (固定) | △ (固定) |
| 状態とロジック | 結合 | 結合 | 分離 | ABCI分離 | VM分離 | Datum分離 | ランタイム | PTB合成 | Resource分離 |

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
| Validator (Avalanche) | `implementations/avalanche/src/validator.rs` |
| Subnet (Avalanche) | `implementations/avalanche/src/subnet.rs` |
| eUTXO (Cardano) | `implementations/cardano/src/eutxo.rs` |
| Plutus (Cardano) | `implementations/cardano/src/plutus.rs` |
| Parachain (Polkadot) | `implementations/polkadot/src/parachain.rs` |
| XCM (Polkadot) | `implementations/polkadot/src/xcm.rs` |
| Object (Sui) | `implementations/sui/src/object.rs` |
| PTB (Sui) | `implementations/sui/src/ptb.rs` |
| Account (Aptos) | `implementations/aptos/src/account.rs` |
| Block-STM (Aptos) | `implementations/aptos/src/block_stm.rs` |
