# blockchain_lab

A Rust-based blockchain architecture playground  
to compare and experiment with different blockchain designs.

ブロックチェーンアーキテクチャを比較・学習するための Rust 実装集。

---

## 🎯 Purpose

This repository provides:

- A shared core implementation
- Multiple chain-specific adaptations
- Architectural comparison across major blockchains

The goal is not to fully reimplement each chain,
but to understand their **design trade-offs** from first principles.

---

## 🧱 Architecture

```
blockchain_lab/
├── core/                    # 共通ライブラリ（ベース実装）
├── implementations/
│   ├── bitcoin/
│   ├── kaspa/
│   ├── ethereum/
│   ├── solana/
│   ├── cosmos/
│   ├── avalanche/
│   ├── cardano/
│   ├── polkadot/
│   ├── sui/
│   ├── aptos/
│   └── monero/
├── docs/
│   └── comparisons/
└── experiments/
```

Each implementation extends or modifies the shared core
to reflect the design philosophy of the target chain.

---

## 📊 Comparison Overview

| Aspect | Bitcoin | Kaspa | Ethereum | Solana | Cosmos | Avalanche | Cardano | Polkadot | Sui | Aptos | Monero |
|--------|---------|-------|----------|--------|--------|-----------|---------|----------|-----|-------|--------|
| Model | UTXO | UTXO | Account | Account | ABCI | Multi-VM | eUTXO | Relay+Para | Object | Resource | Privacy UTXO |
| Consensus | PoW | PoW+GHOSTDAG | PoS | PoH+TowerBFT | Tendermint | Snowball | Ouroboros | BABE+GRANDPA | Mysticeti | AptosBFT | RandomX |
| Structure | Linear | DAG | Linear | Slot | Linear | DAG | Slot | Relay | DAG | DAG | Linear |
| Finality | Probabilistic | Probabilistic | Economic | Economic | Instant | Probabilistic | Probabilistic | Deterministic | Deterministic | Deterministic | Probabilistic |

For detailed comparisons, see `docs/comparisons/`.

---

## 🚀 Quick Start

```bash
cargo build --workspace
cargo test -p blockchain-lab-core
```

---

## 🧠 How to Study

1. Start with `core/src/`
2. Compare design differences in `docs/comparisons/`
3. Inspect each chain implementation
4. Run experiments in `experiments/`

---

## 🔬 Philosophy

Understand systems by reconstructing them.

Compare trade-offs.
Expose assumptions.
Reduce abstraction.

---

## 📚 Documentation

- Consensus comparison
- Data model comparison
- Block structure comparison
- Cryptography comparison

See `docs/comparisons/`.

---

## 📜 License

Educational use only.
