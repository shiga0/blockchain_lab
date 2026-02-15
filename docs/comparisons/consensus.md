# Consensus Mechanisms Comparison

## Overview

| Blockchain | Consensus | Finality | Block Time | Energy |
|------------|-----------|----------|------------|--------|
| Bitcoin | PoW (SHA256) | Probabilistic (~6 blocks) | 10 min | High |
| Ethereum | PoS (Casper FFG) | Economic (~13 min) | 12 sec | Low |
| Kaspa | PoW + GHOSTDAG | Probabilistic (fast) | 1 sec | Medium |
| Solana | PoH + Tower BFT | Economic (~2.5 sec) | 400 ms | Low |
| Cosmos | Tendermint BFT | Instant (2/3+ precommit) | 1-7 sec | Low |
| Avalanche | Snowball | Probabilistic (~1-2 sec) | ~1-2 sec | Low |
| Cardano | Ouroboros Praos | Probabilistic (~k slots) | 1 sec | Low |
| Core (base) | PoW (SHA256) | Probabilistic | Configurable | - |

## Proof of Work (PoW)

### Bitcoin / Core

```
目標: hash < target
   ↓
nonce を変えながら SHA256 を繰り返す
   ↓
条件を満たすハッシュが見つかればブロック完成
```

**特徴:**
- 計算リソースを消費して信頼を構築
- 51%攻撃に耐性（計算量で保護）
- エネルギー消費が大きい

### 難易度調整

| Chain | 調整間隔 | アルゴリズム |
|-------|---------|-------------|
| Bitcoin | 2016ブロック (~2週間) | actual_time / expected_time |
| Core | なし（固定） | - |

## Proof of Stake (PoS)

### Ethereum (Casper FFG)

```
バリデーター（32 ETH ステーク）
   ↓
ランダムにブロック提案者を選出 (RANDAO)
   ↓
他のバリデーターが投票 (attestation)
   ↓
2/3 以上の投票でチェックポイント正当化
   ↓
連続2回正当化で最終化 (finality)
```

**特徴:**
- ステークを担保に不正を防止（スラッシング）
- エネルギー効率が良い
- 経済的ファイナリティ

## GHOSTDAG (Kaspa)

```
複数の親ブロックを許可（DAG構造）
   ↓
Blue Set / Red Set に分類
   ↓
Blue Score で順序付け
   ↓
高スループット（1秒ブロック）でも安全
```

**特徴:**
- 並列ブロック生成が可能
- オーファンブロックなし
- 高TPS（トランザクション/秒）

## Proof of History + Tower BFT (Solana)

### Proof of History (PoH)

```
SHA-256 ハッシュチェーンによる時間証明 (VDF)
   ↓
hash₀ → SHA256(hash₀) → hash₁ → SHA256(hash₁) → ...
   ↓
イベント（トランザクション）をチェーンに混入
   ↓
hash_n → SHA256(hash_n || event) → hash_{n+1}
```

**PoH の役割:**
- 時間の経過を暗号学的に証明
- コンセンサス前の事前順序付け
- リーダーは PoH を生成し、バリデーターは検証のみ

### Tower BFT

```
バリデーターが投票 (slot に対して)
   ↓
投票はスタック構造 (最大32投票)
   ↓
ロックアウトが指数的に増加 (2^depth slots)
   ↓
フォーク切り替えコストが指数的に増大
   ↓
2/3+ stake で実質的ファイナリティ
```

**投票ロックアウト:**
```
Depth 0: lockout = 2 slots   (0.8秒)
Depth 1: lockout = 4 slots   (1.6秒)
Depth 2: lockout = 8 slots   (3.2秒)
...
Depth 31: lockout = 2^32 slots (~54年)
```

**特徴:**
- PoH が共通時計として機能（メッセージ交換削減）
- 経済的ファイナリティ（ロールバックコスト増大）
- 高スループット（並列トランザクション実行 + 400ms スロット）

## Tendermint BFT (Cosmos)

### ラウンドベースBFTコンセンサス

```
┌──────────────────────────────────────────────────────────────┐
│                      Round N                                 │
├────────────┬────────────┬─────────────┬────────────────────┤
│  Propose   │  Prevote   │  Precommit  │      Commit        │
│            │            │             │                    │
│ Proposer   │ Validators │ Validators  │ Block is FINAL     │
│ sends      │ vote on    │ vote if     │ (no forks!)        │
│ block      │ proposal   │ 2/3+ prevote│                    │
├────────────┼────────────┼─────────────┼────────────────────┤
│ Timeout?   │ Need 2/3+  │ Need 2/3+   │ Instant finality   │
│ → Round+1  │ prevotes   │ precommits  │                    │
└────────────┴────────────┴─────────────┴────────────────────┘
```

### フロー詳細

```
1. Propose:    提案者がブロックを作成・ブロードキャスト
       ↓
2. Prevote:    各バリデーターが有効なブロックに prevote
       ↓       (2/3+ prevotes で POL - Proof of Lock 形成)
3. Precommit:  POL があれば precommit
       ↓       (2/3+ precommits でコミット確定)
4. Commit:     ブロック最終化（フォーク不可能）
```

### 即時ファイナリティ

```
PoW/PoS (Bitcoin/Ethereum):    Tendermint BFT (Cosmos):
  Block N                         Block N
    ↓ (probabilistic)               ↓ (2/3+ precommits)
  Block N+1                       ✓ FINAL (no reversal possible)
    ↓ (still not final)
  Block N+2
    ↓
  ...
  Block N+6
    ↓ (~final)
```

**特徴:**
- **即時ファイナリティ**: 2/3+ precommit でブロック確定、フォーク不可
- **BFT耐性**: 1/3 未満の Byzantine ノードに耐性
- **ABCI**: コンセンサスとアプリケーションを分離（モジュラー設計）
- **IBC対応**: チェーン間通信プロトコルのベース

### タイムアウト設定

| パラメータ | デフォルト | 増分/ラウンド |
|-----------|-----------|--------------|
| Propose | 3000ms | +500ms |
| Prevote | 1000ms | +500ms |
| Precommit | 1000ms | +500ms |
| Commit | 1000ms | - |

## Snowball Consensus (Avalanche)

### プロトコル階層

```
┌─────────────────────────────────────────────────────────────┐
│                        Snowball                             │
│  累積投票数を追跡 (preferenceStrength)                      │
│  長期的な選好を記憶して Byzantine 耐性を向上                │
├─────────────────────────────────────────────────────────────┤
│                       Snowflake                             │
│  信頼度カウンター (confidence) + 閾値 β                     │
│  連続成功ポーリングで最終化                                 │
├─────────────────────────────────────────────────────────────┤
│                         Slush                               │
│  シンプルな多数決追従                                       │
│  最後の成功ポーリング結果に従う                             │
└─────────────────────────────────────────────────────────────┘
```

### 繰り返しランダムサンプリング

```
従来の BFT (Tendermint):       Avalanche Snowball:
  全バリデーターが投票            k 個のランダムサンプル
         ↓                              ↓
  O(n²) メッセージ複雑性          O(k) メッセージ/ラウンド
         ↓                              ↓
  決定論的ファイナリティ          確率的ファイナリティ
         ↓                              ↓
  遅い (2/3+ 応答待ち)            高速 (小サンプル)
```

### Snowball フロー

```
ノード A がブロック B を最終化したい:

┌────────────────────────────────────────────────────────────┐
│ Round 1: k=20 のランダムバリデーターをサンプル             │
│          質問: 「ブロック B を選好しますか?」               │
│          応答: 16 が B を選好 (≥ α=15)                     │
│          → 選好を B に更新, confidence++                   │
├────────────────────────────────────────────────────────────┤
│ Round 2: 別の k=20 バリデーターをサンプル                  │
│          応答: 17 が B を選好 (≥ α=15)                     │
│          → confidence++ (今 2)                             │
├────────────────────────────────────────────────────────────┤
│ ...繰り返し...                                             │
├────────────────────────────────────────────────────────────┤
│ Round 20: confidence が β=20 に到達                        │
│           → ブロック B は最終化                            │
└────────────────────────────────────────────────────────────┘
```

### 主要パラメータ

| パラメータ | 値 | 説明 |
|-----------|-----|------|
| k (サンプルサイズ) | 20 | 各ラウンドでクエリするバリデーター数 |
| α (クォーラム) | 15 | 選好更新に必要な最小応答数 |
| β (閾値) | 20 | 最終化に必要な連続成功ポーリング数 |

### Subnet アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│                   Primary Network                        │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                 │
│  │ P-Chain │  │ X-Chain │  │ C-Chain │                 │
│  │Platform │  │Exchange │  │  EVM    │                 │
│  └─────────┘  └─────────┘  └─────────┘                 │
└─────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│                   Custom Subnets                         │
│  独自バリデーター + 独自 VM + 独自ルール                │
└─────────────────────────────────────────────────────────┘
```

**特徴:**
- **リーダーレス**: 提案者選出不要、各ノードが独立してサンプリング
- **サブリニア通信**: O(k) メッセージ複雑性（k は定数）
- **高速ファイナリティ**: ~1-2 秒で確率的ファイナリティ
- **Subnet**: 独立したバリデーターセットとカスタム VM

## Ouroboros Praos (Cardano)

### VRFベースのリーダー選出

```
Ouroboros Praos:
  全てのステークプールが各スロットでリーダー抽選に参加

┌─────────────────────────────────────────────────────────────┐
│ スロット N:                                                 │
│                                                             │
│ Pool A (30% stake):                                        │
│   VRF(key, slot_nonce || N) = 0x3f...                      │
│   threshold = 1 - (1-f)^0.30 = 0.015                       │
│   0x3f... < 0.015? → No, not leader                        │
│                                                             │
│ Pool B (10% stake):                                        │
│   VRF(key, slot_nonce || N) = 0x01...                      │
│   threshold = 1 - (1-f)^0.10 = 0.005                       │
│   0x01... < 0.005? → Yes! Leader for slot N                │
└─────────────────────────────────────────────────────────────┘

f (active_slot_coeff) = 0.05 → ~5% のスロットにブロック
```

### スロットとエポック

```
┌─────────────────────────────────────────────────────────────┐
│                        Epoch (~5日)                         │
│                   432,000 スロット                          │
├─────────────────────────────────────────────────────────────┤
│ Slot 0  │ Slot 1  │ Slot 2  │ ... │ Slot 431,999           │
│ [Block] │ [empty] │ [empty] │     │ [Block]                │
│         │         │         │     │                        │
│ ← 1 秒/スロット →                                          │
└─────────────────────────────────────────────────────────────┘

エポック境界でのイベント:
- スナップショット: 次々エポックのステーク分布を記録
- Nonce 更新: VRF用のランダム性ソースを更新
- 報酬分配: プール報酬をステーカーに分配
```

### セキュリティパラメータ

| パラメータ | 値 | 説明 |
|-----------|-----|------|
| k | 2160 | 最大ロールバック深さ (セキュリティパラメータ) |
| f | 0.05 | アクティブスロット係数 (ブロック生成率) |
| slot | 1秒 | スロット長 |
| epoch | ~5日 | エポック長 (432,000 slots) |

### チェーン選択ルール

```
複数の有効なチェーンがある場合:
  1. ブロック数が多いチェーンを優先 (longest chain)
  2. 同じ長さなら、より低いスロットを優先
  3. k ブロック以上のロールバックは禁止
```

**特徴:**
- **証明可能な安全性**: 形式的セキュリティ証明あり
- **VRFによる秘密選出**: リーダーはブロック公開まで匿名
- **ステークベース**: 計算リソースではなくステーク量で選出確率決定
- **エポック制**: 定期的なステーク更新とパラメータ調整

## 実装ファイル

| Chain | File |
|-------|------|
| Core | `core/src/consensus/pow.rs` |
| Bitcoin | `implementations/bitcoin/src/consensus.rs` |
| Kaspa | `implementations/kaspa/src/consensus.rs` |
| Ethereum | `implementations/ethereum/src/consensus.rs` |
| Solana | `implementations/solana/src/consensus.rs` |
| Cosmos | `implementations/cosmos/src/consensus.rs` |
| Avalanche | `implementations/avalanche/src/snowball.rs` |
| Cardano | `implementations/cardano/src/ouroboros.rs` |
