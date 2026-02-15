# Block Structure Comparison: Linear Chain vs DAG

## Overview

| Structure | Chains | Parents | Orphans | Throughput |
|-----------|--------|---------|---------|------------|
| Linear Chain | Bitcoin, Ethereum, Core | 1 | 破棄 | 低 |
| DAG | Kaspa, IOTA | 複数 | 含む | 高 |
| Slot-Entry | Solana | 1 (prev slot) | スキップ可 | 超高 |

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

## 比較表

| 観点 | Linear Chain | DAG | Slot-Entry |
|------|-------------|-----|------------|
| ブロック生成 | 順次（待ち時間あり） | 並列（即時） | ストリーミング |
| オーファン | 発生（無駄） | なし（全活用） | スキップ可 |
| 順序付け | 自明（height） | 要アルゴリズム | PoH時間順 |
| 実装難易度 | 低 | 高 | 中 |
| ブロック時間 | 長め (Bitcoin: 10分) | 短い (Kaspa: 1秒) | 超短 (Solana: 400ms) |
| 確認時間 | 長い | 短い | 超短い |

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
