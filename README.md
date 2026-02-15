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
│   ├── solana/              # Solana風実装（PoH + Tower BFT）
│   ├── cosmos/              # Cosmos風実装（Tendermint BFT + ABCI）
│   ├── avalanche/           # Avalanche風実装（Snowball + Subnet）
│   ├── cardano/             # Cardano風実装（Ouroboros + eUTXO + Plutus）
│   ├── polkadot/            # Polkadot風実装（BABE + GRANDPA + Parachains）
│   ├── sui/                 # Sui風実装（Mysticeti + Object Model + PTB）
│   ├── aptos/               # Aptos風実装（AptosBFT + Block-STM + Move）
│   └── monero/              # Monero風実装（CryptoNote + RingCT + Bulletproofs）
├── docs/
│   └── comparisons/         # 比較ドキュメント
└── experiments/             # 実験・ベンチマーク
```

## 比較表

| 観点 | Core | Bitcoin | Kaspa | Ethereum | Solana | Cosmos | Avalanche | Cardano | Polkadot | Sui | Aptos | Monero |
|------|------|---------|-------|----------|--------|--------|-----------|---------|----------|-----|-------|--------|
| データモデル | UTXO | UTXO | UTXO | Account | Account+Owner | Account+ABCI | Multi-VM | eUTXO | Relay+Para | Object | Resource | Privacy UTXO |
| コンセンサス | PoW | PoW | PoW+GHOSTDAG | PoS | PoH+TowerBFT | Tendermint | Snowball | Ouroboros | BABE+GRANDPA | Mysticeti | AptosBFT | RandomX PoW |
| ブロック構造 | 線形 | 線形 | DAG | 線形 | Slot-Entry | 線形+Commit | Snowman/DAG | スロット | Relay+Para | DAG | DAG | 線形 |
| ハッシュ | SHA256 | Double SHA256 | BLAKE2b | Keccak-256 | SHA256 | SHA256 | SHA256 | BLAKE2b | BLAKE2-256 | BLAKE2b | SHA256/SHA3 | Keccak-256 |
| 署名曲線 | P-256 | secp256k1 | secp256k1 | secp256k1 | Ed25519 | secp256k1/Ed25519 | secp256k1 | Ed25519 | Sr25519 | Multi | Multi | Ed25519+Ring |
| ブロック時間 | 可変 | 10分 | 1秒 | 12秒 | 400ms | 1-7秒 | 1-2秒 | 1秒 | 6秒 | ~500ms | ~1秒 | 2分 |
| ファイナリティ | 確率的 | 確率的 | 確率的 | 経済的 | 経済的 | 即時 | 確率的 | 確率的 | 決定論的 | 決定論的 | 決定論的 | 確率的 |

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
