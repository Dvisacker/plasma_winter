extern crate rand;
extern crate pairing;
extern crate bellman;
extern crate plasma;
extern crate sapling_crypto;
extern crate hex;
extern crate ff;

use ff::{Field, PrimeField};
use rand::thread_rng;
use bellman::groth16::{
    generate_random_parameters, VerifyingKey
};
use pairing::bn256::{Bn256, Fr};
use sapling_crypto::alt_babyjubjub::AltJubjubBn256;
use pairing::{Engine};

use plasma::vk_contract_generator::hardcode_vk;
use plasma::circuit::baby_plasma::Update;
use plasma::circuit::plasma_constants;
use plasma::balance_tree::BabyBalanceTree;


// Create some parameters, create a proof, and verify the proof.
fn main() {

    let rng = &mut thread_rng();
    let params = {
        let params = &AltJubjubBn256::new();
        let c = Update::<Bn256> {
            params,
            number_of_transactions: 0,
            old_root: Some(Fr::zero()),
            new_root: Some(Fr::zero()),
            public_data_commitment: Some(Fr::zero()),
            block_number: Some(Fr::one()),
            total_fee: Some(Fr::zero()),
            transactions: vec![/*Some((transaction, transaction_witness))*/],
        };
        generate_random_parameters(c, rng).unwrap()
    };

    let tree_depth = *plasma_constants::BALANCE_TREE_DEPTH as u32;
    let initial_root = format!("{}", BabyBalanceTree::new(tree_depth).root_hash().into_repr());
    println!("{}", generate_vk_contract(&params.vk, initial_root.as_ref(), tree_depth));
}

fn generate_vk_contract<E: Engine>(vk: &VerifyingKey<E>, initial_root: &str, tree_depth: u32) -> String {
    format!(
        r#"
// This contract is generated programmatically

pragma solidity ^0.5.0;


// Hardcoded constants to avoid accessing store
contract VerificationKeys {{

    // For tree depth {tree_depth}
    bytes32 constant EMPTY_TREE_ROOT = {initial_root};

    function getVkUpdateCircuit() internal pure returns (uint256[14] memory vk, uint256[] memory gammaABC) {{

        {vk}

    }}

}}
"#,
        vk = hardcode_vk(&vk),
        initial_root = initial_root,
        tree_depth = tree_depth,
    )
}