use chrono::{DateTime, NaiveDateTime, Utc};
use clap::{Parser,Subcommand};
use solana_client::{rpc_client::RpcClient, rpc_response::{Response,RpcSupply, RpcVersionInfo}};
use solana_sdk:: {
    account::from_account, clock::Clock, commitment_config::CommitmentConfig, native_token::lamports_to_sol,native_token::sol_to_lamports, pubkey::Pubkey, signature::{keypair_from_seed, read_keypair_file, write_keypair_file , Keypair}, signer::{keypair, Signer}, sysvar ,system_instruction, transaction::Transaction,
};
use bip39::{Language, Mnemonic, MnemonicType, Seed};
use std::str::FromStr;

use std::{
    io::{self, Write},
};
use std::{thread, time};
#[derive(Parser)]
#[clap(author,version,about,long_about=None)]
struct Cli {
    #[command(subcommand)]
    command : Option<Commands>
}
#[derive(Subcommand)]
enum Commands {
    ClusterInfo,
    Supply,
    KeyGen {
        #[arg(short, long, help = "Output file path for keypair file.")]
        output: String,
        #[arg(
            short,
            long,
            default_value_t = 12,
            help = "How many words to generate for the mnemonic. Valid values are: 12, 15, 18, 21, and 24."
        )]
        mnemonic_word_count: u32,
        #[arg(short, long, help = "Passphrase to use for extra security.")]
        passphrase: Option<String>,
    },
    Balance {
        #[arg(group = "input")]
        address: Option<String>,
        #[arg(long, group = "input")]
        wallet_file: Option<String>,
    },
    Airdrop {
        #[arg(short, long)]
        address: String,
        #[arg(short, long)]
        sol: f64,
    },
    Transfer {
        #[arg(short, long)]
        from_wallet: String,
        #[arg(short, long)]
        to: String,
        #[arg(short, long)]
        sol: f64,
    },
}



const SERVER_URL : &str = "https://api.devnet.solana.com";



fn get_cluster_info(client:&RpcClient) {
        let version:RpcVersionInfo = client.get_version().unwrap();
        let result = client.get_account_with_commitment(&sysvar::clock::id(), CommitmentConfig::finalized()).unwrap();
        let (slot, timestamp) = match result.value {
            Some(clock_account) => {
                let clock: Clock = from_account(&clock_account).unwrap();
                (result.context.slot, clock.unix_timestamp)
            }
            None => {
                panic!("Unexpected None");
            }
        };
        let datetime = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap(),
            Utc,
        );
        println!("Cluster version: {}", version.solana_core);
        println!(
            "Block: {}, Time: {}",
            slot,
            datetime.format("%Y-%m-%d %H:%M:%S")
        );
}



fn get_supply(client : &RpcClient)  {
    let supply_response : Response<RpcSupply> = client.supply().unwrap();
    let supply = supply_response.value;
    println!("Total supply: {} SOL\nCirculating: {} SOL\nNon-Circulating: {} SOL" ,lamports_to_sol(supply.total),lamports_to_sol(supply.circulating),lamports_to_sol(supply.non_circulating) );
}


fn generate_keypair(output_path:&str , mnemonic_word_count:usize,passphrase:&Option<String>) {
        let mnemonic_type = MnemonicType::for_word_count(mnemonic_word_count).unwrap();
        let mnemonic = Mnemonic::new(mnemonic_type, Language::English);
        let seed  = match passphrase {
            Some(phrase)=> Seed::new(&mnemonic, phrase),
            None => Seed::new(&mnemonic , ""),
        };
        let keypair = keypair_from_seed(seed.as_bytes()).unwrap();
        write_keypair_file(&keypair, output_path).unwrap();

        println!("Mnemonic: {:?}", mnemonic);
        println!("Public key {:?}" , &keypair.pubkey());
}


fn get_balance(address : &str , client : &RpcClient) {
    let pubkey = Pubkey::from_str(address).unwrap();
    let balance = client.get_balance(&pubkey).unwrap();
    println!("Balance for {} : {}" , address , lamports_to_sol(balance));

}

fn airdrop_sol(address: &str, sol: f64, client: &RpcClient) {
    let lamports = sol_to_lamports(sol);
    let pubkey = Pubkey::from_str(address).unwrap();
    let signature = client.request_airdrop(&pubkey, lamports).unwrap();
    let wait_milis = time::Duration::from_millis(100);
    print!("Waiting to confirm");
    io::stdout().flush().unwrap();
    loop {
        if let Ok(confirmed) = client.confirm_transaction(&signature) {
            if confirmed {
                println!("\nAirdrop to {}: {}", address, confirmed);
                break;
            }
        }
        print!(".");
        io::stdout().flush().unwrap();
        thread::sleep(wait_milis);
    }
        
    
}


fn transfer_sol(client: &RpcClient, keypair: &Keypair, to_key: &str, sol_amount: f64) {
    let to_pubkey = Pubkey::from_str(to_key).unwrap();
    let lamports = sol_to_lamports(sol_amount);
    let transfer_instruction =
        system_instruction::transfer(&keypair.pubkey(), &to_pubkey, lamports);
    let latest_blockhash = client.get_latest_blockhash().unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&keypair.pubkey()),
        &[keypair],
        latest_blockhash,
    );
    let wait_milis = time::Duration::from_millis(100);
    print!("Waiting to confirm");
    io::stdout().flush().unwrap();
    match client.send_transaction(&transaction) {
        Ok(signature) => loop {
            if let Ok(confirmed) = client.confirm_transaction(&signature) {
                if confirmed {
                    println!("\nTransfer of sol was confirmed");
                    break;
                }
            }
            print!(".");
            io::stdout().flush().unwrap();
            thread::sleep(wait_milis);
        },
        Err(e) => {
            println!("Error transferring sol : {}", e);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let client = RpcClient::new(SERVER_URL);
    match &cli.command {
        Some(Commands::ClusterInfo)=> {
            println!("Get cluster info");
            get_cluster_info(&client);
        },
        Some(Commands::Supply)=> {
            println!("Get supply info");
            get_supply(&client);
        },
        Some(Commands::KeyGen { output, mnemonic_word_count, passphrase }) => {
            println!("Generating key pair");
            generate_keypair(output, *mnemonic_word_count as usize , passphrase);
        },
        Some(Commands::Balance{address, wallet_file})=>{
            if let Some(my_address) = address {
                println!("Get balance for address: {}", my_address);
                get_balance(my_address, &client);
            } else if let Some(wallet_path) = wallet_file {
                println!("Get balance for Wallet file: {}", wallet_path);
                let keypair = read_keypair_file(wallet_path).unwrap();
                get_balance(&keypair.pubkey().to_string(), &client);

            }
        },
        Some(Commands::Airdrop { address, sol }) =>{
            println!("Airdrop {} SOL to {}", sol, address);
            airdrop_sol(address, *sol, &client);
        },
        Some(Commands::Transfer {
            from_wallet,
            to,
            sol,
        }) => {
            let keypair = read_keypair_file(from_wallet).unwrap();
            println!("Transfer {} SOL from {} to {}", sol, &keypair.pubkey(), to);
            transfer_sol(&client, &keypair, to, *sol);
        }
        None => {}
    }
}
