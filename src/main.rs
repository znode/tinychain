use data_encoding::HEXLOWER;
use log::{info, LevelFilter};
use structopt::StructOpt;
use tinychain::{
    send_tx, validate_address, Blockchain, Server, Transaction, UTXOSet, Wallets, CENTERAL_NODE,
    GLOBAL_CONFIG,
};

#[derive(Debug, StructOpt)]
#[structopt(name = "tinychain")]
struct Opt {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(name = "createblockchain", about = "Create a new blockchain")]
    Createblockchain {
        #[structopt(name = "address", help = "The address to send genesis block reward to")]
        address: String,
    },
    #[structopt(name = "createwallet", about = "Create a new wallet")]
    Createwallet,
    #[structopt(
        name = "getbalance",
        about = "Get the wallet balance of the target address"
    )]
    GetBalance {
        #[structopt(name = "address", help = "The wallet address")]
        address: String,
    },
    #[structopt(name = "listaddresses", about = "Print local wallet addres")]
    ListAddresses,
    #[structopt(name = "send", about = "Add new block to chain")]
    Send {
        #[structopt(name = "from", help = "Source wallet address")]
        from: String,
        #[structopt(name = "to", help = "Destination wallet address")]
        to: String,
        #[structopt(name = "amount", help = "Amount to send")]
        amount: i32,
        #[structopt(short, name = "mine", help = "Mine immediately on the same node")]
        mine: bool,
    },
    #[structopt(name = "printchain", about = "Print blockchain all block")]
    Printchain,
    #[structopt(name = "reindexutxo", about = "rebuild UTXO index set")]
    Reindexutxo,
    #[structopt(name = "startnode", about = "Start a node")]
    StartNode {
        #[structopt(short, name = "addr", help = "Node address")]
        addr: Option<String>,
        #[structopt(
            short,
            name = "miner",
            help = "Enable mining mode and send reward to ADDRESS"
        )]
        miner: Option<String>,
    },
}

fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init();
    let opt = Opt::from_args();
    match opt.command {
        Command::Createblockchain { address } => {
            let blockchain = Blockchain::new(address.as_str());
            let utxo_set = UTXOSet::new(blockchain);
            utxo_set.reindex();
            info!("Done!");
        }
        Command::Createwallet => {
            let mut wallet = Wallets::new();
            let address = wallet.create_wallet();
            info!("Your new address: {}", address)
        }
        Command::GetBalance { address } => {
            let address_valid = validate_address(address.as_str());
            if !address_valid {
                panic!("ERROR: Address is not valid")
            }
            let blockchain = Blockchain::load();
            let utxo_set = UTXOSet::new(blockchain);
            let utxos = utxo_set.find_utxo(&address);
            let mut balance = 0;
            for utxo in utxos {
                balance += utxo.get_value();
            }
            info!("Balance of {}: {}", address, balance);
        }
        Command::ListAddresses => {
            let wallets = Wallets::new();
            for address in wallets.get_addresses() {
                info!("{}", address)
            }
        }
        Command::Send {
            from,
            to,
            amount,
            mine,
        } => {
            if !validate_address(from.as_str()) {
                panic!("ERROR: Sender address is not valid")
            }
            if !validate_address(to.as_str()) {
                panic!("ERROR: Recipient address is not valid")
            }
            let blockchain = Blockchain::load();
            let utxo_set = UTXOSet::new(blockchain.clone());

            let transaction =
                Transaction::new_utxo_transaction(from.as_str(), to.as_str(), amount, &utxo_set);

            if mine {
                let coinbase_tx = Transaction::new_coinbase_tx(from.as_str());

                let block = blockchain.mine_block(&vec![transaction, coinbase_tx]);

                utxo_set.update(&block);
            } else {
                send_tx(CENTERAL_NODE, &transaction);
            }
            info!("Success!")
        }
        Command::Printchain => {
            let blockchain = Blockchain::load();
            for block in &blockchain {
                info!("Pre block hash: {}", block.get_pre_block_hash());
                info!("Cur block hash: {}", block.get_hash());
                info!("Cur block Timestamp: {}", block.get_timestamp());
                for tx in block.get_transactions() {
                    let cur_txid_hex = HEXLOWER.encode(tx.get_id());
                    info!("- Transaction txid_hex: {}", cur_txid_hex);

                    if !tx.is_coinbase() {
                        for input in tx.get_vin() {
                            let txid_hex = HEXLOWER.encode(input.get_txid());
                            let address = input.get_address();
                            info!(
                                "-- Input txid = {}, vout = {}, from = {}",
                                txid_hex,
                                input.get_vout(),
                                address,
                            )
                        }
                    }
                    for output in tx.get_vout() {
                        let address = output.get_address();
                        info!("-- Output value = {}, to = {}", output.get_value(), address,)
                    }
                }
            }
        }
        Command::Reindexutxo => {
            let blockchain = Blockchain::load();
            let utxo_set = UTXOSet::new(blockchain);
            utxo_set.reindex();
            let count = utxo_set.count_transactions();
            info!("Done! There are {} transactions in the UTXO set.", count);
        }
        Command::StartNode { addr, miner } => {
            if let Some(addr) = miner {
                if !validate_address(addr.as_str()) {
                    panic!("Wrong miner address!")
                }
                info!("Mining is on. Address to receive rewards: {}", addr);
                GLOBAL_CONFIG.set_mining_addr(addr);
            }
            let blockchain = Blockchain::load();

            if let Some(addr) = addr {
                GLOBAL_CONFIG.set_node_addr(addr);
            }
            let sockert_addr = GLOBAL_CONFIG.get_node_addr();
            Server::new(blockchain).run(sockert_addr.as_str());
        }
    }
}
