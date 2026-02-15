# Cryptography Comparison

## Hash Functions

| Chain | Block Hash | Address Hash | Merkle |
|-------|-----------|--------------|--------|
| Bitcoin | Double SHA256 | RIPEMD160(SHA256) | Double SHA256 |
| Ethereum | Keccak-256 | Keccak-256 (last 20 bytes) | Keccak-256 |
| Kaspa | BLAKE2b | BLAKE2b | BLAKE2b |
| Solana | SHA256 (PoH) | Ed25519公開鍵 | SHA256 |
| Cosmos | SHA256 | Bech32 (secp256k1/Ed25519) | SHA256 |
| Avalanche | SHA256 | CB58/Bech32 (secp256k1) | SHA256 |
| Cardano | BLAKE2b-256 | Bech32 (Ed25519) | BLAKE2b-256 |
| Polkadot | BLAKE2-256 | SS58 (Sr25519/Ed25519) | Binary Merkle |
| Sui | BLAKE2b-256 | Bech32 (Ed25519/Secp256k1/Secp256r1) | BLAKE2b-256 |
| Core | SHA256 | RIPEMD160(SHA256) | SHA256 |

### SHA256 (Bitcoin/Core)

```
入力 → SHA256 → 256-bit ハッシュ

特徴:
- NIST標準
- 広く使用されている
- Bitcoin では Double SHA256 (SHA256を2回)
```

### Keccak-256 (Ethereum)

```
入力 → Keccak-256 → 256-bit ハッシュ

特徴:
- SHA-3 の原型（標準化前のバージョン）
- スポンジ構造
- 拡張性に優れる
```

### BLAKE2b (Kaspa)

```
入力 → BLAKE2b → 256-bit ハッシュ

特徴:
- SHA-3 競争のファイナリスト
- 高速（SHA256より速い）
- 並列化に適する
```

## Signature Schemes

| Chain | Curve | Algorithm |
|-------|-------|-----------|
| Bitcoin | secp256k1 | ECDSA / Schnorr (Taproot) |
| Ethereum | secp256k1 | ECDSA |
| Kaspa | secp256k1 | ECDSA / Schnorr |
| Solana | Ed25519 | EdDSA |
| Cosmos | secp256k1 / Ed25519 | ECDSA / EdDSA |
| Avalanche | secp256k1 | ECDSA |
| Cardano | Ed25519 | EdDSA + VRF |
| Polkadot | Sr25519 / Ed25519 | Schnorr + VRF |
| Sui | Ed25519 / Secp256k1 / Secp256r1 | EdDSA / ECDSA |
| Core | P-256 (NIST) | ECDSA |

### secp256k1 vs P-256

```
secp256k1 (Bitcoin/Ethereum):
- Koblitz曲線
- パラメータが「ランダムに見えない」（透明性）
- 計算効率が良い

P-256 (NIST/Core):
- NIST標準曲線
- より広く使用（TLS等）
- ハードウェアアクセラレーション対応
```

### Schnorr Signatures (BIP-340)

```
特徴:
- 署名集約（複数署名を1つに）
- バッチ検証が高速
- 線形性（マルチシグに有利）

Bitcoin Taproot で採用
```

### Ed25519 (Solana)

```
特徴:
- EdDSA (Edwards-curve Digital Signature Algorithm)
- Curve25519 の twisted Edwards 形式
- 高速（ECDSA より検証が速い）
- 決定的署名（同一入力で同一出力）
- 64バイト署名 (R: 32 + S: 32)

Solana の選択理由:
- バッチ検証が高速（並列トランザクション処理に有利）
- 実装がシンプル（サイドチャネル攻撃に強い）
```

## Address Formats

### Base58Check (Bitcoin/Core)

```
[version] + [payload] + [checksum]
    1B         20B         4B

version: 0x00 (mainnet), 0x6f (testnet)
payload: RIPEMD160(SHA256(pubkey))
checksum: first 4 bytes of double_sha256(version + payload)

例: 1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2
```

### Bech32/Bech32m (Bitcoin SegWit)

```
[HRP] + [separator] + [data]
 bc        1          witness_program

特徴:
- 小文字のみ（QRコード効率）
- エラー検出/訂正能力が高い
- SegWit アドレス用

例: bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4
```

### Hex + EIP-55 (Ethereum)

```
0x + 40 hex characters (20 bytes)

EIP-55 checksum:
- 大文字/小文字でチェックサム埋め込み
- keccak256(lowercase_address) のビットで決定

例: 0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed
```

### Base58 (Solana)

```
Ed25519 公開鍵をそのまま Base58 エンコード (32 bytes)

特徴:
- チェックサムなし（公開鍵自体が有効性を持つ）
- 44文字程度の文字列
- 0, O, I, l を除外（視認性向上）

例: 7EYnhQoR9YM3N7UoaKRoA44Uy8JeaZV3qyouov87awMs

Program Derived Address (PDA):
- seeds + program_id からオフカーブアドレスを導出
- プログラムのみが「署名」可能（エスクロー等に使用）
```

### Bech32 (Cosmos)

```
[HRP] + [separator] + [data]
cosmos    1          pubkey_hash

特徴:
- 人間可読プレフィックス (HRP) でチェーン区別
- secp256k1 または Ed25519 公開鍵をサポート
- 複数鍵タイプ対応（バリデーター等）

例: cosmos1hsk6jryyqjfhp5dhc55tc9jtckygx0eph6dd02

バリデーター用:
- cosmosvaloper... (オペレーター)
- cosmosvalcons... (コンセンサス)
```

### CB58 / Bech32 (Avalanche)

```
チェーン別アドレスフォーマット:

X-Chain (アセット転送):
  X-avax1... (Bech32)
  例: X-avax1k8yzttlmxfg0nkkhngrzpk35kp9ux3hvk7efg

P-Chain (プラットフォーム):
  P-avax1... (Bech32)
  例: P-avax1k8yzttlmxfg0nkkhngrzpk35kp9ux3hvk7efg

C-Chain (EVM):
  0x... (Ethereum互換)
  例: 0x8db97C7cEcE249c2b98bDC0226Cc4C2A57BF52FC

NodeID (バリデーター):
  NodeID-... (CB58)
  例: NodeID-7Xhw2mDxuDS44j42TCB6U5579esbSt3Lg

特徴:
- チェーンプレフィックスで用途を区別
- C-Chain は Ethereum と互換
- CB58 = Base58 with checksum
```

### Bech32 (Cardano)

```
Shelley アドレス構造:
  addr1... (mainnet) / addr_test1... (testnet)

┌─────────────────────────────────────────────────────────────┐
│ Address = Header + PaymentCredential + StakeCredential      │
├─────────────────────────────────────────────────────────────┤
│ Header (1 byte):                                            │
│   - ネットワーク (mainnet/testnet)                          │
│   - アドレスタイプ (base/pointer/enterprise/reward)         │
│                                                             │
│ PaymentCredential (28 bytes):                               │
│   - PubKeyHash または ScriptHash                            │
│                                                             │
│ StakeCredential (28 bytes, optional):                       │
│   - ステーキング用クレデンシャル                           │
└─────────────────────────────────────────────────────────────┘

アドレスタイプ:
- Base: Payment + Staking (一般的)
- Enterprise: Payment のみ (ステーキングなし)
- Pointer: Staking を chain pointer で参照
- Reward: ステーキング報酬受取用

例: addr1qx2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer...

Script Address:
- addr1w... (スクリプトでロックされた資金)
- Plutus validator の hash が PaymentCredential
```

### SS58 (Polkadot/Substrate)

```
SS58 アドレスフォーマット:
  [prefix] + [pubkey] + [checksum]

┌─────────────────────────────────────────────────────────────┐
│ Prefix (1-2 bytes):                                         │
│   - 0: Polkadot                                             │
│   - 2: Kusama                                               │
│   - 42: Generic Substrate                                   │
│   - チェーン固有の番号（SS58 registry で管理）             │
│                                                             │
│ Public Key (32 bytes):                                      │
│   - Sr25519 または Ed25519 公開鍵                          │
│                                                             │
│ Checksum (2 bytes):                                         │
│   - Blake2b-512("SS58PRE" || prefix || pubkey)[0:2]        │
└─────────────────────────────────────────────────────────────┘

Base58 エンコードで文字列化:
  Polkadot:  1... (例: 12rqdJSjFFYqQ5TJKqQQU...)
  Kusama:    C... (例: CxDDSH8gS7jecsxaRL8Txf...)
  Generic:   5... (例: 5GrwvaEF5zXb26Fz9rcQpD...)

特徴:
- チェーン別プレフィックスで誤送金を防止
- 同一秘密鍵で異なるチェーンアドレスを導出可能
- チェックサムでタイプミス検出
```

### Sr25519 (Polkadot)

```
Ristretto255 上の Schnorr 署名

特徴:
- BABE VRF に必要（Ed25519 は VRF 非対応）
- 署名集約可能（将来のマルチシグ最適化）
- Ed25519 と同等の速度
- Curve25519 ベース（広く研究されている）

用途:
- Session Keys: バリデーターのコンセンサス鍵
- BABE: VRF ベースのスロット割り当て
- アカウント署名: ユーザートランザクション

Ed25519 との比較:
┌───────────────┬───────────────┬───────────────┐
│               │ Ed25519       │ Sr25519       │
├───────────────┼───────────────┼───────────────┤
│ VRF           │ ✗             │ ✓             │
│ 署名集約      │ ✗             │ ✓             │
│ 決定的署名    │ ✓             │ ✗ (より安全)  │
│ 標準化        │ RFC 8032      │ Substrate独自 │
└───────────────┴───────────────┴───────────────┘
```

### Sui Address (Multi-Scheme)

```
Sui は複数の署名スキームをサポート:

┌─────────────────────────────────────────────────────────────┐
│ Signature Schemes (flag byte):                               │
├─────────────────────────────────────────────────────────────┤
│ 0x00: Ed25519 (推奨)                                        │
│ 0x01: Secp256k1 (Ethereum互換)                              │
│ 0x02: Secp256r1/P-256 (Apple/WebAuthn互換)                  │
│ 0x03: MultiSig                                               │
│ 0x05: zkLogin                                                │
└─────────────────────────────────────────────────────────────┘

アドレス生成:
  address = BLAKE2b-256(flag_byte || public_key)[0:32]

フォーマット:
  0x + 64 hex characters (32 bytes)
  例: 0x5aef8ee6...c9d7a312

特徴:
- 複数署名方式を1アカウントで切り替え可能
- zkLogin でソーシャルログイン対応
- MultiSig で N-of-M 署名
- Object アドレスも同じフォーマット
```

### Object ID と Transaction Digest (Sui)

```
Object ID (32 bytes):
  新規オブジェクト: transaction_digest || creation_index
  パッケージ: deployer_address || creation_index

Transaction Digest (32 bytes):
  BLAKE2b-256(transaction_data)

BlockRef の Digest:
  BLAKE2b-256(epoch || round || author || timestamp || ancestors || ...)

特徴:
- 全てのIDが32バイト統一
- Object ID はコンテンツアドレッシング的
- トランザクションは再実行可能（決定的）
```

### VRF (Verifiable Random Function)

```
Polkadot の BABE で使用:

VRF_sign(secret_key, input) → (output, proof)
VRF_verify(public_key, input, output, proof) → bool

特徴:
- output は入力に対して決定的だが予測不可能
- proof により正しく計算されたことを検証可能
- BABE スロット割り当てで使用

┌─────────────────────────────────────────────────────────────┐
│ BABE VRF 使用例:                                            │
├─────────────────────────────────────────────────────────────┤
│ input = epoch_randomness || slot_number                     │
│                                                             │
│ (output, proof) = VRF_sign(validator_key, input)            │
│                                                             │
│ if output < threshold(stake):                               │
│     → このスロットでブロック生成権を獲得                   │
│     → ブロックに output と proof を含める                  │
│                                                             │
│ 他のバリデーターは proof で検証可能                        │
└─────────────────────────────────────────────────────────────┘
```

## Merkle Trees

### Binary Merkle Tree (Bitcoin/Core)

```
         Root
        /    \
      H01    H23
     /  \   /  \
    H0  H1 H2  H3
    |   |   |   |
   TX0 TX1 TX2 TX3

Node = SHA256(left + right)
奇数の場合: 最後を複製
```

### Merkle Patricia Trie (Ethereum)

```
特徴:
- キーバリューストア
- 効率的な状態更新
- 軽量クライアント証明

ノード種類:
- Extension: 共通プレフィックス圧縮
- Branch: 16分岐 + value
- Leaf: キー末尾 + value
```

## 実装ファイル

| Component | File |
|-----------|------|
| Hash (Core) | `core/src/crypto/hash.rs` |
| Signature (Core) | `core/src/crypto/signature.rs` |
| Address (Core) | `core/src/crypto/address.rs` |
| Merkle (Core) | `core/src/crypto/merkle.rs` |
| secp256k1 (Bitcoin) | `implementations/bitcoin/src/crypto.rs` |
| PoH/SHA256 (Solana) | `implementations/solana/src/consensus.rs` |
| Ed25519 (Solana) | `implementations/solana/src/account.rs` |
| PDA (Solana) | `implementations/solana/src/program.rs` |
| Block Hash (Cosmos) | `implementations/cosmos/src/types.rs` |
| Vote/Commit (Cosmos) | `implementations/cosmos/src/consensus.rs` |
| Validator Hash (Avalanche) | `implementations/avalanche/src/validator.rs` |
| Snowball Choice (Avalanche) | `implementations/avalanche/src/snowball.rs` |
| Ouroboros VRF (Cardano) | `implementations/cardano/src/ouroboros.rs` |
| Plutus Data Hash (Cardano) | `implementations/cardano/src/eutxo.rs` |
| BABE VRF (Polkadot) | `implementations/polkadot/src/babe.rs` |
| GRANDPA Signatures (Polkadot) | `implementations/polkadot/src/grandpa.rs` |
| Parachain Hashes (Polkadot) | `implementations/polkadot/src/parachain.rs` |
| Object ID/Digest (Sui) | `implementations/sui/src/object.rs` |
| Block Digest (Sui) | `implementations/sui/src/mysticeti.rs` |
| Transaction Digest (Sui) | `implementations/sui/src/ptb.rs` |
