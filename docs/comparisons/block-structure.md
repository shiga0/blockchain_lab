# Block Structure Comparison: Linear Chain vs DAG

## Overview

| Structure | Chains | Parents | Orphans | Throughput |
|-----------|--------|---------|---------|------------|
| Linear Chain | Bitcoin, Ethereum, Core | 1 | 破棄 | 低 |
| Linear + Commit | Cosmos | 1 | なし (BFT) | 中 |
| DAG | Kaspa, IOTA, Avalanche X-Chain | 複数 | 含む | 高 |
| Slot-Entry | Solana | 1 (prev slot) | スキップ可 | 超高 |
| Snowman Chain | Avalanche P/C-Chain | 1 | なし (Snowball) | 高 |
| Slot-based | Cardano | 1 | 許容 (longest) | 中 |
| Relay + Para | Polkadot | 1 (relay) + paras | GRANDPA収束 | 高 |
| Mysticeti DAG | Sui | 複数 (round) | 全含む | 超高 |

## Linear Chain (Bitcoin/Ethereum/Core)

### 構造

```
[Genesis] ← [Block 1] ← [Block 2] ← [Block 3] ← ... ← [Tip]
    ↑          ↑           ↑           ↑
 height=0   height=1    height=2    height=3
```

### ブロック構造

```rust
// core/src/primitives/block.rs

pub struct Block {
    pub timestamp: i64,
    pub prev_hash: String,      // 1つの親のみ
    pub hash: String,
    pub transactions: Vec<Transaction>,
    pub nonce: i64,
    pub height: usize,
    pub merkle_root: Vec<u8>,
}
```

### 特徴

- **シンプル**: 実装・理解が容易
- **一意な順序**: height で自然に決定
- **オーファン問題**: 同時生成ブロックは1つを除き破棄

### フォーク解決

```
         ┌─ [B2a] ← [B3a] ← [B4a]  ← Winner (longest chain)
[B1] ←───┤
         └─ [B2b] ← [B3b]          ← Orphaned
```

## DAG (Kaspa/IOTA)

### 構造

```
         ┌───────────────┐
         ↓               ↓
[B0] ← [B1] ← [B3] ← [B5] ← [B7]
         ↑     ↗ ↖     ↑
       [B2] ← [B4] ← [B6]
```

### ブロック構造

```rust
// implementations/kaspa/src/dag.rs

pub struct DagBlock {
    pub hash: Vec<u8>,
    pub parents: Vec<Vec<u8>>,   // 複数の親
    pub blue_score: u64,         // GHOSTDAG スコア
    pub selected_parent: Option<Vec<u8>>,
    pub is_blue: bool,
}
```

### 特徴

- **高スループット**: 並列ブロック生成
- **オーファンなし**: 全ブロックがDAGに含まれる
- **複雑な順序付け**: GHOSTDAG等のアルゴリズムが必要

### GHOSTDAG 順序付け

```
1. Selected Parent Chain を特定（Blue Score 最大の経路）
2. 各ブロックを Blue / Red に分類
3. Blue Set 内でトポロジカル順序
4. Red Set は Blue の後に配置
```

## Slot-Entry Structure (Solana)

### 構造

```
Epoch (~2 days)
  └── Slot 0 (400ms)
       ├── Entry 0 (tick)
       ├── Entry 1 (txs)
       ├── Entry 2 (tick)
       ├── ...
       └── Entry 63 (tick)
  └── Slot 1
  └── ...
  └── Slot 432,000
```

### エントリー構造

```rust
// implementations/solana/src/consensus.rs

pub struct Entry {
    pub num_hashes: u64,        // 前エントリーからのハッシュ回数
    pub hash: [u8; 32],         // PoH ハッシュ
    pub transactions: Vec<Tx>,  // このエントリーのトランザクション
}
```

### Slot と Entry

```
┌────────────────────────────────────────────────────────┐
│                    Slot (400ms)                        │
├────────────────────────────────────────────────────────┤
│ Entry 0: num_hashes=12500, hash=..., txs=[]  (tick)   │
│ Entry 1: num_hashes=100, hash=..., txs=[tx1,tx2]      │
│ Entry 2: num_hashes=12400, hash=..., txs=[]  (tick)   │
│ ...                                                    │
│ Entry 63: num_hashes=12500, hash=..., txs=[] (tick)   │
└────────────────────────────────────────────────────────┘

- 1 tick = 12,500 hashes (~6.25ms)
- 1 slot = 64 ticks (400ms)
- リーダーは連続4スロットを担当
```

### Shred (ネットワーク転送単位)

```
Entry は Shred に分割してネットワーク転送:

┌─────────────────────────────────────────┐
│               Shred                      │
├─────────────────────────────────────────┤
│ signature: [u8; 64]                     │
│ slot: u64                               │
│ index: u32                              │
│ shred_type: Data / Code                 │
│ payload: ~1KB                           │
└─────────────────────────────────────────┘

- Data Shred: 実際のエントリーデータ
- Code Shred: Reed-Solomon 誤り訂正符号
```

### 特徴

- **ストリーミング**: ブロック完成を待たずにデータ送信
- **誤り訂正**: UDP パケットロスに対応
- **並列検証**: 受信しながら PoH 検証可能

## Linear + Commit Structure (Cosmos)

### 構造

Cosmos は線形チェーンだが、LastCommit でファイナリティ証明を含む。

```
[Block N-1] ← [Block N] ← [Block N+1]
                  │
                  ├── Header (metadata + hashes)
                  ├── Data (transactions)
                  ├── Evidence (Byzantine proof)
                  └── LastCommit (2/3+ signatures for N-1)
```

### ブロック構造

```rust
// implementations/cosmos/src/types.rs

pub struct Block {
    pub header: Header,          // ブロックメタデータ
    pub data: Data,              // トランザクション
    pub evidence: EvidenceData,  // 不正行為の証拠
    pub last_commit: Option<Commit>,  // 前ブロックの署名
}

pub struct Header {
    pub chain_id: String,
    pub height: i64,
    pub time: u64,
    pub last_block_id: BlockId,      // 前ブロックへの参照
    pub validators_hash: Hash,        // バリデーターセットのハッシュ
    pub app_hash: Hash,               // アプリケーション状態ルート
    pub proposer_address: Address,    // 提案者
    // ... その他ハッシュ
}

pub struct Commit {
    pub height: i64,
    pub round: i32,
    pub block_id: BlockId,
    pub signatures: Vec<CommitSig>,  // 2/3+の署名
}
```

### LastCommit の役割

```
Block N:
┌─────────────────────────────────────────┐
│ Header:                                 │
│   height: N                             │
│   last_block_id: hash(Block N-1)        │
│                                         │
│ LastCommit:                             │
│   ┌───────────────────────────────────┐ │
│   │ Commit for Block N-1              │ │
│   │ ・Validator 1: ✓ signed           │ │
│   │ ・Validator 2: ✓ signed           │ │
│   │ ・Validator 3: ✓ signed           │ │
│   │ ・Validator 4: ✗ absent           │ │
│   │ (2/3+ = Block N-1 is FINAL)       │ │
│   └───────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

### 特徴

- **即時ファイナリティ**: LastCommit で前ブロックの最終化を証明
- **フォークなし**: BFT コンセンサスによりフォーク不可能
- **Evidence**: 二重投票等の不正を記録・スラッシング
- **軽量クライアント対応**: Header と Commit だけで検証可能

## Snowman Chain (Avalanche P/C-Chain)

### 構造

Avalanche の P-Chain と C-Chain は Snowman コンセンサスを使用する線形チェーン。

```
[Block 0] ← [Block 1] ← [Block 2] ← [Block 3]
                              ↓
                    Snowball でファイナリティ
```

### Snowball によるブロック合意

```
┌─────────────────────────────────────────────────────────────┐
│                    Block Decision                           │
├─────────────────────────────────────────────────────────────┤
│  Choice A: Block hash = 0xabc...                           │
│  Choice B: Block hash = 0xdef...                           │
│                                                             │
│  Snowball Instance:                                         │
│    preference: A                                            │
│    preferenceStrength: {A: 15, B: 3}                       │
│    confidence: 12                                           │
│    finalized: false                                         │
│                                                             │
│  Round N: Sample k=20 validators                            │
│           16 prefer A (≥ α=15) → confidence++               │
│           ...                                               │
│  Round N+8: confidence reaches β=20 → FINALIZED            │
└─────────────────────────────────────────────────────────────┘
```

### 実装

```rust
// implementations/avalanche/src/snowball.rs

pub struct Snowball {
    snowflake: Snowflake,                    // 信頼度追跡
    preference_strength: HashMap<ChoiceId, usize>,  // 累積投票
    strongest: Option<ChoiceId>,             // 最強選択肢
}

pub struct BinarySnowball {
    preference: bool,          // 現在の選好
    strength_a: usize,         // 選択肢Aの累積
    strength_b: usize,         // 選択肢Bの累積
    confidence: usize,         // 信頼度カウンター
    finalized: bool,           // 最終化フラグ
}
```

## DAG (Avalanche X-Chain)

X-Chain は DAG 構造を使用（Kaspa と同様のアプローチ）。

```
複数の親を参照可能 + UTXO モデル

         ┌───────────────┐
         ↓               ↓
[Vtx0] ← [Vtx1] ← [Vtx3] ← [Vtx5]
         ↑     ↗ ↖     ↑
       [Vtx2] ← [Vtx4]
```

### 特徴

- **リーダーレス**: 誰でも vertex (ブロック) を提案可能
- **高スループット**: 並列提案・処理
- **Snowball で順序付け**: コンフリクトを投票で解決

## Relay Block + Parachain Inclusion (Polkadot)

### 構造

Polkadot のリレーブロックはパラチェーン候補を含む特殊な構造。

```
┌─────────────────────────────────────────────────────────────────┐
│                    Relay Block Header                            │
├─────────────────────────────────────────────────────────────────┤
│ parent_hash: Hash                                               │
│ number: BlockNumber                                             │
│ state_root: Hash          // リレーチェーン状態                 │
│ extrinsics_root: Hash     // トランザクション                   │
│ digest: [                                                       │
│   PreRuntime(BABE, slot_claim),   // BABE スロット情報          │
│   Seal(BABE, signature),          // ブロック署名               │
│   Consensus(GRANDPA, authority),  // GRANDPA 権限セット変更     │
│ ]                                                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Parachains Inherent Data                        │
├─────────────────────────────────────────────────────────────────┤
│ bitfields: [                                                    │
│   SignedAvailabilityBitfield(validator=0, bits=0b11110111),    │
│   SignedAvailabilityBitfield(validator=1, bits=0b11111111),    │
│   ...                                                           │
│ ]                                                               │
│                                                                 │
│ backed_candidates: [                                            │
│   BackedCandidate(para_id=1000, receipt=..., votes=[sig1,sig2])│
│   BackedCandidate(para_id=1001, receipt=..., votes=[sig3,sig4])│
│ ]                                                               │
│                                                                 │
│ disputes: [                                                     │
│   DisputeStatement(candidate=..., valid=true, validator=5)     │
│   DisputeStatement(candidate=..., valid=false, validator=7)    │
│ ]                                                               │
└─────────────────────────────────────────────────────────────────┘
```

### Availability Core

```
各パラチェーンはコアに割り当てられ、ローテーション:

┌───────┬───────┬───────┬───────┐
│Core 0 │Core 1 │Core 2 │Core 3 │
├───────┼───────┼───────┼───────┤
│Para   │Para   │Para   │Para   │
│1000   │1001   │1002   │1003   │
│       │       │       │       │
│Free   │Occupied│Free  │Occupied│
│       │pending │       │avail  │
└───────┴───────┴───────┴───────┘

Occupied → Availability 待ち (PoV チャンク分散中)
Available → 2/3+ チャンク保持確認済み → 包含可能
Free → 新しい候補を受け入れ可能
```

### 実装

```rust
// implementations/polkadot/src/parachain.rs

pub struct ParachainsInherentData {
    /// 署名付き Availability ビットフィールド
    pub bitfields: Vec<SignedAvailabilityBitfield>,
    /// バッキング済み候補
    pub backed_candidates: Vec<BackedCandidate>,
    /// 紛争ステートメント
    pub disputes: Vec<DisputeStatement>,
    /// 親ヘッダハッシュ
    pub parent_header: Hash,
}

pub enum CoreState {
    Free,
    Occupied(OccupiedCore),
}

pub struct OccupiedCore {
    pub para_id: ParaId,
    pub group_responsible: GroupIndex,
    pub candidate_hash: Hash,
    pub availability: AvailabilityBitfield,
    pub time_out_at: BlockNumber,
}
```

### GRANDPA ファイナリティ証明

```
┌─────────────────────────────────────────────────────────────────┐
│                  GRANDPA Justification                           │
├─────────────────────────────────────────────────────────────────┤
│ round: 42                                                       │
│ commit:                                                         │
│   target_hash: 0xabc...                                         │
│   target_number: 12345                                          │
│   precommits: [                                                 │
│     (validator=0, signature=...),                               │
│     (validator=1, signature=...),                               │
│     ... (2/3+ of validators)                                    │
│   ]                                                             │
│ votes_ancestries: [blocks between last finalized and target]   │
└─────────────────────────────────────────────────────────────────┘
```

### 特徴

- **2層構造**: リレーブロック + パラチェーンブロック
- **Availability**: イレイジャー符号化で PoV を分散保存
- **バッチファイナリティ**: GRANDPA が複数ブロックを一度に最終化
- **フォーク解決**: BABE でフォーク許容、GRANDPA で収束

## Mysticeti DAG (Sui)

### 構造

Sui の Mysticeti は各ラウンドで複数のバリデーターがブロックを提案する DAG 構造。

```
┌─────────────────────────────────────────────────────────────────┐
│                    Mysticeti DAG Structure                       │
│                                                                 │
│  Round 3:    [B3_0]───────[B3_1]───────[B3_2]───────[B3_3]     │
│                │  ╲         │  ╲         │  ╲         │        │
│  Round 2:    [B2_0]───────[B2_1]───────[B2_2]───────[B2_3]     │
│                │  ╲         │  ╲         │  ╲         │        │
│  Round 1:    [B1_0]───────[B1_1]───────[B1_2]───────[B1_3]     │
│                │           │           │           │            │
│  Genesis:    [G_0]       [G_1]       [G_2]       [G_3]         │
│                                                                 │
│  各バリデーターがラウンドごとに1ブロック提案                   │
│  ブロックは前ラウンドの複数ブロックを祖先として参照            │
└─────────────────────────────────────────────────────────────────┘
```

### ブロック構造

```rust
// implementations/sui/src/mysticeti.rs

pub struct Block {
    pub epoch: u64,
    pub round: Round,
    pub author: AuthorityIndex,
    pub timestamp_ms: TimestampMs,
    pub ancestors: Vec<BlockRef>,       // 複数の親
    pub transactions: Vec<ConsensusTransaction>,
    pub commit_votes: Vec<CommitVote>,  // 前リーダーへの投票
    pub signature: [u8; 64],
}

pub struct BlockRef {
    pub round: Round,
    pub author: AuthorityIndex,
    pub digest: BlockDigest,
}

pub struct Commit {
    pub index: CommitIndex,
    pub leader: BlockRef,
    pub blocks: Vec<BlockRef>,  // コミットされた subdag
}
```

### Wave ベースのコミット

```
3ラウンドを1 Wave として処理:

Wave 0 (rounds 0-2):
  Round 0: リーダー (validator 0) がブロック提案
  Round 1: 他バリデーターがリーダーブロックを参照
  Round 2: 2f+1 参照確認 → リーダーコミット

Wave 1 (rounds 3-5):
  Round 3: リーダー (validator 1) がブロック提案
  ...

コミット時、リーダーとその祖先の subdag 全体が順序付けされる
```

### 特徴

- **高スループット**: 全バリデーターが各ラウンドでブロック提案
- **低レイテンシ**: ~500ms ラウンド、~480ms ファイナリティ
- **全ブロック活用**: DAG に含まれる全ブロックがコミット対象
- **Wave ベース**: 3ラウンドごとにリーダー決定・コミット

## 比較表

| 観点 | Linear Chain | Linear+Commit | DAG | Slot-Entry | Snowman | Slot-based | Relay+Para | Mysticeti |
|------|-------------|---------------|-----|------------|---------|------------|------------|-----------|
| ブロック生成 | 順次（待ち時間あり） | 順次 (BFT) | 並列（即時） | ストリーミング | 並列提案可 | VRF抽選 | BABE VRF | 全員並列 |
| オーファン | 発生（無駄） | なし | なし（全活用） | スキップ可 | なし | 競合選択 | GRANDPA収束 | なし（全含） |
| 順序付け | 自明（height） | height + round | 要アルゴリズム | PoH時間順 | Snowball投票 | スロット順 | スロット+ラウンド | Wave+subdag |
| 実装難易度 | 低 | 中 | 高 | 中 | 中 | 中 | 高 | 高 |
| ブロック時間 | 長め (10分) | 中 (1-7秒) | 短い (1秒) | 超短 (400ms) | 高速 (1-2秒) | 1秒/スロット | 6秒/スロット | ~500ms |
| ファイナリティ | 確率的 | 即時 | 確率的 | 経済的 | 確率的 | 確率的 | 決定論的 (GRANDPA) | 決定論的 |

## Tips vs Single Tip

### Linear Chain

```
Tip は常に1つ（最新ブロック）

[B0] ← [B1] ← [B2] ← [B3]  ← Tip (single)
```

### DAG

```
Tips は複数存在可能（子を持たないブロック）

[B0] ← [B1] ← [B3]  ← Tip 1
         ↖
       [B2] ← [B4]  ← Tip 2
```

新ブロックは複数の Tips を親として参照可能。

## 実装ファイル

| Structure | File |
|-----------|------|
| Linear (Core) | `core/src/primitives/blockchain.rs` |
| Linear (Core) | `core/src/primitives/block.rs` |
| DAG (Kaspa) | `implementations/kaspa/src/dag.rs` |
| Slot-Entry (Solana) | `implementations/solana/src/consensus.rs` |
| Linear+Commit (Cosmos) | `implementations/cosmos/src/types.rs` |
| Snowball (Avalanche) | `implementations/avalanche/src/snowball.rs` |
| Subnet (Avalanche) | `implementations/avalanche/src/subnet.rs` |
| Ouroboros (Cardano) | `implementations/cardano/src/ouroboros.rs` |
| BABE (Polkadot) | `implementations/polkadot/src/babe.rs` |
| GRANDPA (Polkadot) | `implementations/polkadot/src/grandpa.rs` |
| Parachain (Polkadot) | `implementations/polkadot/src/parachain.rs` |
| Mysticeti DAG (Sui) | `implementations/sui/src/mysticeti.rs` |
| Object Model (Sui) | `implementations/sui/src/object.rs` |
| PTB (Sui) | `implementations/sui/src/ptb.rs` |
