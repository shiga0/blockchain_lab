# blockchain_lab

ブロックチェーンアーキテクチャを比較・学習するためのRust実装集。

## 概要

異なるブロックチェーンの仕組みを理解するため、共通のコア実装をベースに各チェーン固有の特徴を実装・比較できる環境。

## 構成

```
blockchain_lab/
├── core/                    # 共通ライブラリ（ベース実装）
├── implementations/
│   ├── bitcoin/             # Bitcoin風実装
│   ├── kaspa/               # Kaspa風実装（GHOSTDAG）
│   ├── ethereum/            # Ethereum風実装（Account Model）
│   └── solana/              # Solana風実装（PoH + Tower BFT）
├── docs/
│   └── comparisons/         # 比較ドキュメント
└── experiments/             # 実験・ベンチマーク
```

## 比較表

| 観点 | Core | Bitcoin | Kaspa | Ethereum | Solana |
|------|------|---------|-------|----------|--------|
| データモデル | UTXO | UTXO | UTXO | Account | Account+Owner |
| コンセンサス | PoW | PoW | PoW + GHOSTDAG | PoS | PoH + Tower BFT |
| ブロック構造 | 線形 | 線形 | DAG | 線形 | Slot-Entry |
| ハッシュ | SHA256 | Double SHA256 | BLAKE2b | Keccak-256 | SHA256 |
| 署名曲線 | P-256 | secp256k1 | secp256k1 | secp256k1 | Ed25519 |
| ブロック時間 | 可変 | 10分 | 1秒 | 12秒 | 400ms |

## クイックスタート

```bash
# ビルド
cargo build --release

# Core のテスト
cargo test -p blockchain-lab-core

# 全体ビルド
cargo build --workspace
```

## 学習の進め方

1. **Core を理解**: `core/src/` のコードを読み、基本を把握
2. **比較ドキュメント**: `docs/comparisons/` で違いを確認
3. **実装を追加**: 各 `implementations/*/src/` にTODOを実装
4. **実験**: `experiments/` で動作確認・ベンチマーク

## ドキュメント

- [コンセンサス比較](docs/comparisons/consensus.md)
- [データモデル比較](docs/comparisons/data-model.md)
- [ブロック構造比較](docs/comparisons/block-structure.md)
- [暗号方式比較](docs/comparisons/cryptography.md)

## ライセンス

Educational purposes.
