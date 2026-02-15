# Cryptography Comparison

## Hash Functions

| Chain | Block Hash | Address Hash | Merkle |
|-------|-----------|--------------|--------|
| Bitcoin | Double SHA256 | RIPEMD160(SHA256) | Double SHA256 |
| Ethereum | Keccak-256 | Keccak-256 (last 20 bytes) | Keccak-256 |
| Kaspa | BLAKE2b | BLAKE2b | BLAKE2b |
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
