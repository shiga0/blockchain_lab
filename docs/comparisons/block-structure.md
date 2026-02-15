# Block Structure Comparison: Linear Chain vs DAG

## Overview

| Structure | Chains | Parents | Orphans | Throughput |
|-----------|--------|---------|---------|------------|
| Linear Chain | Bitcoin, Ethereum, Core | 1 | 破棄 | 低 |
| DAG | Kaspa, IOTA | 複数 | 含む | 高 |

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

## 比較表

| 観点 | Linear Chain | DAG |
|------|-------------|-----|
| ブロック生成 | 順次（待ち時間あり） | 並列（即時） |
| オーファン | 発生（無駄） | なし（全活用） |
| 順序付け | 自明（height） | 要アルゴリズム |
| 実装難易度 | 低 | 高 |
| ブロック時間 | 長め (Bitcoin: 10分) | 短い (Kaspa: 1秒) |
| 確認時間 | 長い | 短い |

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
