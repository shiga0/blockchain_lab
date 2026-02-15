//! Blockchain Base - CLI Entry Point
//!
//! This is the main entry point for the blockchain CLI application.

use blockchain_lab_core::{
    api::{Cli, Command},
    crypto::{validate_address, address_to_pub_key_hash, hash160, pub_key_hash_to_address},
    execution::{Transaction, UTXOSet},
    network::{Server, send_tx, CENTRAL_NODE},
    primitives::Blockchain,
    storage::Storage,
    wallet::Wallets,
    config::{Config, GLOBAL_CONFIG},
};
use clap::Parser;
use data_encoding::HEXLOWER;
use log::LevelFilter;

fn main() {
    // Initialize logging
    env_logger::builder().filter_level(LevelFilter::Info).init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute command
    match cli.command {
        Command::CreateBlockchain { address } => {
            if !validate_address(&address) {
                eprintln!("ERROR: Invalid address");
                std::process::exit(1);
            }

            let storage = Storage::open_default();
            let blockchain = Blockchain::create(&address, storage.get_db());
            let utxo_set = UTXOSet::new(storage.get_db());
            let utxo_map = blockchain.find_utxo();
            utxo_set.reindex(&utxo_map);

            println!("Blockchain created successfully!");
            println!("Genesis block mined. Reward sent to: {}", address);
        }

        Command::CreateWallet => {
            let mut wallets = Wallets::new();
            let address = wallets.create_wallet();
            println!("Your new address: {}", address);
        }

        Command::GetBalance { address } => {
            if !validate_address(&address) {
                eprintln!("ERROR: Invalid address");
                std::process::exit(1);
            }

            let pub_key_hash = address_to_pub_key_hash(&address).unwrap();
            let storage = Storage::open_default();
            let utxo_set = UTXOSet::new(storage.get_db());
            let utxos = utxo_set.find_utxo(&pub_key_hash);

            let balance: i32 = utxos.iter().map(|o| o.get_value()).sum();
            println!("Balance of {}: {}", address, balance);
        }

        Command::ListAddresses => {
            let wallets = Wallets::new();
            let addresses = wallets.get_addresses();

            if addresses.is_empty() {
                println!("No wallets found. Create one with 'createwallet'");
            } else {
                println!("Addresses:");
                for addr in addresses {
                    println!("  {}", addr);
                }
            }
        }

        Command::Send { from, to, amount, mine } => {
            if !validate_address(&from) {
                eprintln!("ERROR: Invalid sender address");
                std::process::exit(1);
            }
            if !validate_address(&to) {
                eprintln!("ERROR: Invalid recipient address");
                std::process::exit(1);
            }

            let storage = Storage::open_default();
            let blockchain = match Blockchain::open(storage.get_db()) {
                Ok(bc) => bc,
                Err(e) => {
                    eprintln!("ERROR: {}", e);
                    std::process::exit(1);
                }
            };

            let wallets = Wallets::new();
            let wallet = match wallets.get_wallet(&from) {
                Some(w) => w,
                None => {
                    eprintln!("ERROR: Wallet not found for address: {}", from);
                    std::process::exit(1);
                }
            };

            let utxo_set = UTXOSet::new(storage.get_db());
            let pub_key_hash = hash160(wallet.get_public_key());

            // Find spendable outputs
            let (accumulated, valid_outputs) = utxo_set.find_spendable_outputs(&pub_key_hash, amount);

            if accumulated < amount {
                eprintln!(
                    "ERROR: Insufficient funds. Available: {}, Required: {}",
                    accumulated, amount
                );
                std::process::exit(1);
            }

            // Build transaction inputs
            use blockchain_lab_core::execution::TXInput;
            use blockchain_lab_core::execution::TXOutput;

            let mut inputs = Vec::new();
            for (txid_hex, outs) in valid_outputs {
                let txid = HEXLOWER.decode(txid_hex.as_bytes()).unwrap();
                for out_idx in outs {
                    let mut input = TXInput::new(&txid, out_idx);
                    input.set_pub_key(wallet.get_public_key().to_vec());
                    inputs.push(input);
                }
            }

            // Build transaction outputs
            let mut outputs = vec![TXOutput::new(amount, &to)];
            if accumulated > amount {
                outputs.push(TXOutput::new(accumulated - amount, &from));
            }

            // Create and sign transaction
            let mut tx = Transaction::new(inputs, outputs);
            tx.sign(wallet.get_pkcs8(), |txid| blockchain.find_transaction(txid));

            if mine == 1 {
                // Mine immediately
                let coinbase_tx = Transaction::new_coinbase_tx(&from);
                let block = blockchain.mine_block(&[tx, coinbase_tx]);
                let utxo_map = blockchain.find_utxo();
                utxo_set.reindex(&utxo_map);
                println!("Block mined: {}", block.get_hash());
            } else {
                // Send to network
                let config = Config::new();
                send_tx(CENTRAL_NODE, &tx, &config);
            }

            println!("Success! Sent {} from {} to {}", amount, from, to);
        }

        Command::PrintChain => {
            let storage = Storage::open_default();
            let blockchain = match Blockchain::open(storage.get_db()) {
                Ok(bc) => bc,
                Err(e) => {
                    eprintln!("ERROR: {}", e);
                    std::process::exit(1);
                }
            };

            let mut iterator = blockchain.iterator();
            while let Some(block) = iterator.next() {
                println!("=====================================");
                println!("Height: {}", block.get_height());
                println!("Timestamp: {}", block.get_timestamp());
                println!("Prev Hash: {}", block.get_prev_hash());
                println!("Hash: {}", block.get_hash());
                println!("Nonce: {}", block.get_nonce());
                println!("Merkle Root: {}", HEXLOWER.encode(block.get_merkle_root()));
                println!("\nTransactions:");

                for tx in block.get_transactions() {
                    let txid = HEXLOWER.encode(tx.get_id());
                    println!("  TX: {}", txid);

                    if tx.is_coinbase() {
                        println!("    [Coinbase]");
                    } else {
                        for input in tx.get_vin() {
                            let in_txid = HEXLOWER.encode(input.get_txid());
                            println!("    Input: {}:{}", in_txid, input.get_vout());
                        }
                    }

                    for (i, output) in tx.get_vout().iter().enumerate() {
                        let addr = pub_key_hash_to_address(output.get_pub_key_hash());
                        println!("    Output[{}]: {} -> {}", i, output.get_value(), addr);
                    }
                }
                println!();
            }
        }

        Command::ReindexUtxo => {
            let storage = Storage::open_default();
            let blockchain = match Blockchain::open(storage.get_db()) {
                Ok(bc) => bc,
                Err(e) => {
                    eprintln!("ERROR: {}", e);
                    std::process::exit(1);
                }
            };

            let utxo_set = UTXOSet::new(storage.get_db());
            let utxo_map = blockchain.find_utxo();
            utxo_set.reindex(&utxo_map);

            let count = utxo_set.count_transactions();
            println!("Done! {} transactions in UTXO set.", count);
        }

        Command::StartNode { miner } => {
            if let Some(ref addr) = miner {
                if !validate_address(addr) {
                    eprintln!("ERROR: Invalid mining address");
                    std::process::exit(1);
                }
                println!("Mining enabled. Rewards to: {}", addr);
                GLOBAL_CONFIG.set_mining_addr(addr.clone());
            }

            let storage = Storage::open_default();
            let blockchain = match Blockchain::open(storage.get_db()) {
                Ok(bc) => bc,
                Err(e) => {
                    eprintln!("ERROR: {}", e);
                    std::process::exit(1);
                }
            };

            let config = Config::new();
            let addr = config.get_node_addr();
            println!("Starting node on {}", addr);

            let server = Server::new(blockchain, config);
            server.run(&addr);
        }
    }
}
