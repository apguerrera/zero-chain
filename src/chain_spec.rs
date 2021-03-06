use primitives::{Ed25519AuthorityId, ed25519};
use zero_chain_runtime::{
	AccountId, GenesisConfig, ConsensusConfig, TimestampConfig, BalancesConfig,
	SudoConfig, IndicesConfig, ConfTransferConfig
};
use substrate_service;

use zprimitives::{
	prepared_vk::PreparedVk,
	pkd_address::PkdAddress,
	ciphertext::Ciphertext,	
	};
use keys::{ProofGenerationKey, EncryptionKey};
use jubjub::{curve::{JubjubBls12, FixedGenerators, fs}};
use zpairing::{bls12_381::Bls12, Field};
use zcrypto::elgamal;
use std::path::Path;
use std::fs::File;
use std::io::{BufReader, Read};

lazy_static! {
    static ref JUBJUB: JubjubBls12 = { JubjubBls12::new() };
}

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialised `ChainSpec`. This is a specialisation of the general Substrate ChainSpec type.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum Alternative {
	/// Whatever the current runtime is, with just Alice as an auth.
	Development,
	/// Whatever the current runtime is, with simple Alice/Bob auths.
	LocalTestnet,
}

impl Alternative {
	/// Get an actual chain config from one of the alternatives.
	pub(crate) fn load(self) -> Result<ChainSpec, String> {		
		Ok(match self {
			Alternative::Development => ChainSpec::from_genesis(
				"Development",
				"dev",
				|| testnet_genesis(vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().into(),
				], vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into(),
					ed25519::Pair::from_seed(b"Bob                             ").public().0.into(),
					ed25519::Pair::from_seed(b"Charlie                         ").public().0.into(),
				],
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into()
				),
				vec![],
				None,
				None,
				None,
				None
			),
			Alternative::LocalTestnet => ChainSpec::from_genesis(
				"Local Testnet",
				"local_testnet",
				|| testnet_genesis(vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().into(),
					ed25519::Pair::from_seed(b"Bob                             ").public().into(),
				], vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into(),
					ed25519::Pair::from_seed(b"Bob                             ").public().0.into(),
					ed25519::Pair::from_seed(b"Charlie                         ").public().0.into(),
					ed25519::Pair::from_seed(b"Dave                            ").public().0.into(),
					ed25519::Pair::from_seed(b"Eve                             ").public().0.into(),
					ed25519::Pair::from_seed(b"Ferdie                          ").public().0.into(),
				],
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into()
				),
				vec![],
				None,
				None,
				None,
				None
			),
		})
	}

	pub(crate) fn from(s: &str) -> Option<Self> {
		match s {
			"dev" => Some(Alternative::Development),
			"" | "local" => Some(Alternative::LocalTestnet),
			_ => None,
		}
	}
}

fn testnet_genesis(initial_authorities: Vec<Ed25519AuthorityId>, endowed_accounts: Vec<AccountId>, root_key: AccountId) -> GenesisConfig {	
	GenesisConfig {
		consensus: Some(ConsensusConfig {			
			code: include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/zero_chain_runtime_wasm.compact.wasm").to_vec(),
			authorities: initial_authorities.clone(),
		}),
		system: None,
		timestamp: Some(TimestampConfig {
			period: 10,					// 10 second block time.
		}),
		indices: Some(IndicesConfig {
			ids: endowed_accounts.clone(),
		}),
		balances: Some(BalancesConfig {
			existential_deposit: 500,
			transfer_fee: 0,
			creation_fee: 0,
			balances: endowed_accounts.iter().map(|&k|(k, (1 << 60))).collect(),
			vesting: vec![],
		}),
		sudo: Some(SudoConfig {
			key: root_key,
		}),
		// fees: Some(FeesConfig {
		// 	transaction_base_fee: 1,
		// 	transaction_byte_fee: 0,
		// }),
		conf_transfer: Some(ConfTransferConfig {
			encrypted_balance: vec![alice_init(), (PkdAddress::from_slice(b"Alice                           "), Ciphertext(b"Alice                           Bob                             ".to_vec()))],
			verifying_key: get_pvk(),										
			_genesis_phantom_data: Default::default(),
		})
	}
}

fn get_pvk() -> PreparedVk {
	let vk_path = Path::new("./demo/cli/verification.params"); 
	let vk_file = File::open(&vk_path).unwrap();
	let mut vk_reader = BufReader::new(vk_file);

	let mut buf_vk = vec![];
    vk_reader.read_to_end(&mut buf_vk).unwrap();
		
	PreparedVk(buf_vk)
}

fn alice_init() -> (PkdAddress, Ciphertext) {
	let alice_seed = b"Alice                           ";	
	let alice_value = 100 as u32;

	let p_g = FixedGenerators::Diversifier; // 1 same as NoteCommitmentRandomness;	    

	let address = EncryptionKey::<Bls12>::from_ok_bytes(alice_seed, &JUBJUB);

	// The default balance is not encrypted with randomness.
	let enc_alice_bal = elgamal::Ciphertext::encrypt(alice_value, fs::Fs::one(), &address.0, p_g, &JUBJUB);

	let bdk = ProofGenerationKey::<Bls12>::from_ok_bytes(alice_seed, &JUBJUB).bdk();	

	let dec_alice_bal = enc_alice_bal.decrypt(bdk, p_g, &JUBJUB).unwrap();
	assert_eq!(dec_alice_bal, alice_value);	

	(PkdAddress::from_encryption_key(&address), Ciphertext::from_ciphertext(&enc_alice_bal))
}
