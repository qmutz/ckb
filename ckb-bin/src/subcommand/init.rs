use std::fs;
use std::path::PathBuf;

use crate::helper::prompt;
use ckb_app_config::{ExitCode, InitArgs};
use ckb_chain_spec::ChainSpec;
use ckb_db::{db::RocksDB, DBConfig};
use ckb_jsonrpc_types::ScriptHashType;
use ckb_resource::{
    Resource, TemplateContext, AVAILABLE_SPECS, CKB_CONFIG_FILE_NAME, DEFAULT_SPEC,
    MINER_CONFIG_FILE_NAME, SPEC_DEV_FILE_NAME,
};
use ckb_types::{prelude::*, H256};

const DEFAULT_LOCK_SCRIPT_HASH_TYPE: &str = "type";
const SECP256K1_BLAKE160_SIGHASH_ALL_ARG_LEN: usize = 20 * 2 + 2; // 42 = 20 x 2 + prefix 0x

fn check_db_compatibility(path: PathBuf) {
    if path.exists() {
        let config = DBConfig {
            path: path.clone(),
            ..Default::default()
        };
        if let Some(err) = RocksDB::open_with_error(&config, 1).err() {
            if err
                .to_string()
                .contains("the database version is not matched")
            {
                let input =
                    prompt(format!("Database is not incompatible, remove {:?}? ", path).as_str());

                if ["y", "Y"].contains(&input.trim()) {
                    if let Some(e) = fs::remove_dir_all(path).err() {
                        eprintln!("{}", e);
                    }
                }
            }
        }
    }
}

pub fn init(args: InitArgs) -> Result<(), ExitCode> {
    let mut args = args;

    if args.list_chains {
        for spec in AVAILABLE_SPECS {
            println!("{}", spec);
        }
        return Ok(());
    }

    let exported = Resource::exported_in(&args.root_dir);
    if !args.force && exported {
        eprintln!("Config files already exists, use --force to overwrite.");

        if args.interactive {
            let input = prompt("Overwrite config file now? ");

            if !["y", "Y"].contains(&input.trim()) {
                return Err(ExitCode::Failure);
            }
        } else {
            return Err(ExitCode::Failure);
        }
    }

    if args.interactive {
        let data_dir = args.root_dir.join("data");
        let db_path = data_dir.join("db");
        let indexer_db_path = data_dir.join("indexer_db");

        check_db_compatibility(db_path);
        check_db_compatibility(indexer_db_path);

        let in_block_assembler_code_hash = prompt("code hash: ");
        let in_args = prompt("args: ");
        let in_hash_type = prompt("hash_type: ");

        args.block_assembler_code_hash = Some(in_block_assembler_code_hash.trim().to_string());

        args.block_assembler_args = in_args
            .trim()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        match serde_plain::from_str::<ScriptHashType>(in_hash_type.trim()).ok() {
            Some(hash_type) => args.block_assembler_hash_type = hash_type,
            None => eprintln!("Invalid block assembler hash type"),
        }
    }

    // Try to find the default secp256k1 from bundled chain spec.
    let default_code_hash_option =
        ChainSpec::load_from(&Resource::bundled(format!("specs/{}.toml", args.chain)))
            .ok()
            .map(|spec| {
                let hash: H256 = spec
                    .build_consensus()
                    .expect("Build consensus failed")
                    .get_secp_type_script_hash()
                    .unpack();
                format!("{:#x}", hash)
            });

    let block_assembler_code_hash = args.block_assembler_code_hash.as_ref().or_else(|| {
        if !args.block_assembler_args.is_empty() {
            default_code_hash_option.as_ref()
        } else {
            None
        }
    });

    let block_assembler = match block_assembler_code_hash {
        Some(hash) => {
            if let Some(default_code_hash) = &default_code_hash_option {
                if ScriptHashType::Type != args.block_assembler_hash_type {
                    eprintln!(
                        "WARN: the default lock should use hash type `{}`, you are using `{}`.\n\
                         It will require `ckb run --ba-advanced` to enable this block assembler",
                        DEFAULT_LOCK_SCRIPT_HASH_TYPE, args.block_assembler_hash_type
                    );
                } else if *default_code_hash != *hash {
                    eprintln!(
                        "WARN: the default secp256k1 code hash is `{}`, you are using `{}`.\n\
                         It will require `ckb run --ba-advanced` to enable this block assembler",
                        default_code_hash, hash
                    );
                } else if args.block_assembler_args.len() != 1
                    || args.block_assembler_args[0].len() != SECP256K1_BLAKE160_SIGHASH_ALL_ARG_LEN
                {
                    eprintln!(
                        "WARN: the block assembler arg is not a valid secp256k1 pubkey hash.\n\
                         It will require `ckb run --ba-advanced` to enable this block assembler"
                    );
                }
            }
            format!(
                "[block_assembler]\n\
                 code_hash = \"{}\"\n\
                 args = [ \"{}\" ]\n\
                 hash_type = \"{}\"",
                hash,
                args.block_assembler_args.join("\", \""),
                serde_plain::to_string(&args.block_assembler_hash_type).unwrap(),
            )
        }
        None => {
            eprintln!("WARN: mining feature is disabled because of lacking the block assembler config options");
            format!(
                "# secp256k1_blake160_sighash_all example:\n\
                 # [block_assembler]\n\
                 # code_hash = \"{}\"\n\
                 # args = [ \"ckb cli blake160 <compressed-pubkey>\" ]\n\
                 # hash_type = \"{}\"",
                default_code_hash_option.unwrap_or_default(),
                DEFAULT_LOCK_SCRIPT_HASH_TYPE,
            )
        }
    };

    let context = TemplateContext {
        spec: &args.chain,
        rpc_port: &args.rpc_port,
        p2p_port: &args.p2p_port,
        log_to_file: args.log_to_file,
        log_to_stdout: args.log_to_stdout,
        block_assembler: &block_assembler,
    };

    println!(
        "{} CKB directory in {}",
        if !exported {
            "Initialized"
        } else {
            "Reinitialized"
        },
        args.root_dir.display()
    );

    println!("create {}", CKB_CONFIG_FILE_NAME);
    Resource::bundled_ckb_config().export(&context, &args.root_dir)?;
    println!("create {}", MINER_CONFIG_FILE_NAME);
    Resource::bundled_miner_config().export(&context, &args.root_dir)?;

    if args.chain == DEFAULT_SPEC {
        println!("create {}", SPEC_DEV_FILE_NAME);
        Resource::bundled(SPEC_DEV_FILE_NAME.to_string()).export(&context, &args.root_dir)?;
    }

    Ok(())
}
