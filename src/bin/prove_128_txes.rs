extern crate rand;
extern crate pairing;
extern crate sapling_crypto;
extern crate ff;
extern crate hex;
extern crate crypto;
extern crate plasma;
extern crate time;
extern crate bellman;

use time::PreciseTime;

use ff::{Field, PrimeField, PrimeFieldRepr, BitIterator};
use pairing::bn256::*;
use rand::{SeedableRng, Rng, XorShiftRng};
use sapling_crypto::alt_babyjubjub::{AltJubjubBn256};
use plasma::balance_tree::{BabyBalanceTree, BabyLeaf};
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use std::collections::HashMap;

use bellman::groth16::{
    create_random_proof, 
    generate_random_parameters, 
    prepare_verifying_key, 
    verify_proof,
};

use sapling_crypto::jubjub::{
    FixedGenerators,
};

use sapling_crypto::eddsa::{
    PrivateKey,
    PublicKey
};

use plasma::circuit::utils::*;
use plasma::circuit::plasma_constants;
use plasma::circuit::baby_plasma::{Transaction, TransactionWitness, Update};
use sapling_crypto::circuit::float_point::{convert_to_float};

const TXES_TO_TEST: usize = 1600;

fn main() {
    println!("Will try to make a proof for {} transactons", TXES_TO_TEST);

    let p_g = FixedGenerators::SpendingKeyGenerator;
    let params = &AltJubjubBn256::new();
    let rng = &mut XorShiftRng::from_seed([0x3dbe6258, 0x8d313d76, 0x3237db17, 0xe5bc0654]);
    let tree_depth = *plasma_constants::BALANCE_TREE_DEPTH as u32;

    let capacity: u32 = 1 << tree_depth;

    let mut existing_accounts: Vec<(u32, PrivateKey<Bn256>, PublicKey<Bn256>)> = vec![];

    let mut tree = BabyBalanceTree::new(tree_depth);

    let number_of_accounts = 10000;

    let mut existing_account_hm = HashMap::<u32, bool>::new();

    let default_balance_string = "1000000";
    let transfer_amount : u128 = 1000;
    let fee_amount : u128 = 0;

    for _ in 0..number_of_accounts {
        let mut leaf_number : u32 = rng.gen();
        leaf_number = leaf_number % capacity;
        if existing_account_hm.get(&leaf_number).is_some() {
            continue;
        } else {
            existing_account_hm.insert(leaf_number, true);
        }

        let sk = PrivateKey::<Bn256>(rng.gen());
        let pk = PublicKey::from_private(&sk, p_g, params);
        let (x, y) = pk.0.into_xy();

        existing_accounts.push((leaf_number, sk, pk));

        let leaf = BabyLeaf {
            balance:    Fr::from_str(default_balance_string).unwrap(),
            nonce:      Fr::zero(),
            pub_x:      x,
            pub_y:      y,
        };

        tree.insert(leaf_number, leaf.clone());
    }

    let num_accounts = existing_accounts.len();

    println!("Inserted {} accounts", num_accounts);

    let initial_root = tree.root_hash();

    let mut witnesses: Vec<Option<(Transaction<Bn256>, TransactionWitness<Bn256>)>> = vec![];
    let mut public_data_vector: Vec<bool> = vec![];

    let transfer_amount_as_field_element = Fr::from_str(&transfer_amount.to_string()).unwrap();

    let transfer_amount_bits = convert_to_float(
        transfer_amount,
        *plasma_constants::AMOUNT_EXPONENT_BIT_WIDTH,
        *plasma_constants::AMOUNT_MANTISSA_BIT_WIDTH,
        10
    ).unwrap();

    let transfer_amount_encoded: Fr = le_bit_vector_into_field_element(&transfer_amount_bits);

    let fee_as_field_element = Fr::from_str(&fee_amount.to_string()).unwrap();

    let fee_bits = convert_to_float(
        fee_amount,
        *plasma_constants::FEE_EXPONENT_BIT_WIDTH,
        *plasma_constants::FEE_MANTISSA_BIT_WIDTH,
        10
    ).unwrap();

    let fee_encoded: Fr = le_bit_vector_into_field_element(&fee_bits);

    let mut total_fees = Fr::zero();

    for _ in 0..TXES_TO_TEST {
        let mut sender_account_number: usize = rng.gen();
        sender_account_number = sender_account_number % num_accounts;
        let sender_account_info: &(u32, PrivateKey<Bn256>, PublicKey<Bn256>) = existing_accounts.get(sender_account_number).clone().unwrap();

        let mut recipient_account_number: usize = rng.gen();
        recipient_account_number = recipient_account_number % num_accounts;
        if recipient_account_number == sender_account_number {
            recipient_account_number = recipient_account_number+1 % num_accounts;
        }
        let recipient_account_info: &(u32, PrivateKey<Bn256>, PublicKey<Bn256>) = existing_accounts.get(recipient_account_number).clone().unwrap();

        let sender_leaf_number = sender_account_info.0;
        let recipient_leaf_number = recipient_account_info.0;

        let mut items = tree.items.clone();

        let sender_leaf = items.get(&sender_leaf_number).unwrap().clone();        
        let recipient_leaf = items.get(&recipient_leaf_number).unwrap().clone();

        let path_from : Vec<Option<(Fr, bool)>> = tree.merkle_path(sender_leaf_number).into_iter().map(|e| Some(e)).collect();
        let path_to: Vec<Option<(Fr, bool)>> = tree.merkle_path(recipient_leaf_number).into_iter().map(|e| Some(e)).collect();

        // println!("Making a transfer from {} to {}", sender_leaf_number, recipient_leaf_number);

        let from = Fr::from_str(& sender_leaf_number.to_string());
        let to = Fr::from_str(& recipient_leaf_number.to_string());

        let mut transaction : Transaction<Bn256> = Transaction {
            from: from,
            to: to,
            amount: Some(transfer_amount_encoded.clone()),
            fee: Some(fee_encoded.clone()),
            nonce: Some(sender_leaf.nonce),
            good_until_block: Some(Fr::one()),
            signature: None
        };

        let sender_sk = &sender_account_info.1;

        transaction.sign(
            &sender_sk,
            p_g,
            params,
            rng
        );

        assert!(transaction.signature.is_some());

        assert!(tree.verify_proof(sender_leaf_number, sender_leaf.clone(), tree.merkle_path(sender_leaf_number)));
        assert!(tree.verify_proof(recipient_leaf_number, recipient_leaf.clone(), tree.merkle_path(recipient_leaf_number)));

        // println!("Sender: balance: {}, nonce: {}, pub_x: {}, pub_y: {}", sender_leaf.balance, sender_leaf.nonce, sender_leaf.pub_x, sender_leaf.pub_y);
        // println!("Recipient: balance: {}, nonce: {}, pub_x: {}, pub_y: {}", recipient_leaf.balance, recipient_leaf.nonce, recipient_leaf.pub_x, recipient_leaf.pub_y);

        let mut updated_sender_leaf = sender_leaf.clone();
        let mut updated_recipient_leaf = recipient_leaf.clone();

        updated_sender_leaf.balance.sub_assign(&transfer_amount_as_field_element);
        updated_sender_leaf.balance.sub_assign(&fee_as_field_element);

        updated_sender_leaf.nonce.add_assign(&Fr::one());

        updated_recipient_leaf.balance.add_assign(&transfer_amount_as_field_element);

        total_fees.add_assign(&fee_as_field_element);

        // println!("Updated sender: balance: {}, nonce: {}, pub_x: {}, pub_y: {}", updated_sender_leaf.balance, updated_sender_leaf.nonce, updated_sender_leaf.pub_x, updated_sender_leaf.pub_y);
        // println!("Updated recipient: balance: {}, nonce: {}, pub_x: {}, pub_y: {}", updated_recipient_leaf.balance, updated_recipient_leaf.nonce, updated_recipient_leaf.pub_x, updated_recipient_leaf.pub_y);

        tree.insert(sender_leaf_number, updated_sender_leaf.clone());
        tree.insert(recipient_leaf_number, updated_recipient_leaf.clone());

        assert!(tree.verify_proof(sender_leaf_number, updated_sender_leaf.clone(), tree.merkle_path(sender_leaf_number)));
        assert!(tree.verify_proof(recipient_leaf_number, updated_recipient_leaf.clone(), tree.merkle_path(recipient_leaf_number)));


        let public_data = transaction.public_data_into_bits();
        public_data_vector.extend(public_data.into_iter());

        let transaction_witness = TransactionWitness {
            auth_path_from: path_from,
            balance_from: Some(sender_leaf.balance),
            nonce_from: Some(sender_leaf.nonce),
            pub_x_from: Some(sender_leaf.pub_x),
            pub_y_from: Some(sender_leaf.pub_y),
            auth_path_to: path_to,
            balance_to: Some(recipient_leaf.balance),
            nonce_to: Some(recipient_leaf.nonce),
            pub_x_to: Some(recipient_leaf.pub_x),
            pub_y_to: Some(recipient_leaf.pub_y)
        };

        let witness = (transaction.clone(), transaction_witness);

        witnesses.push(Some(witness));
    }

    let block_number = Fr::one();

    let final_root = tree.root_hash();

    let mut public_data_initial_bits = vec![];

    // these two are BE encodings because an iterator is BE. This is also an Ethereum standard behavior

    let block_number_bits: Vec<bool> = BitIterator::new(block_number.into_repr()).collect();
    for _ in 0..256-block_number_bits.len() {
        public_data_initial_bits.push(false);
    }
    public_data_initial_bits.extend(block_number_bits.into_iter());

    let total_fee_bits: Vec<bool> = BitIterator::new(total_fees.into_repr()).collect();
    for _ in 0..256-total_fee_bits.len() {
        public_data_initial_bits.push(false);
    }
    public_data_initial_bits.extend(total_fee_bits.into_iter());

    assert_eq!(public_data_initial_bits.len(), 512);

    let mut h = Sha256::new();

    let bytes_to_hash = be_bit_vector_into_bytes(&public_data_initial_bits);

    h.input(&bytes_to_hash);

    let mut hash_result = [0u8; 32];
    h.result(&mut hash_result[..]);

    {    
        let packed_transaction_data_bytes = be_bit_vector_into_bytes(&public_data_vector);

        println!("Public data size for Ethereum smart contract call = {}", packed_transaction_data_bytes.len());

        let mut next_round_hash_bytes = vec![];
        next_round_hash_bytes.extend(hash_result.iter());
        next_round_hash_bytes.extend(packed_transaction_data_bytes);

        let mut h = Sha256::new();

        h.input(&next_round_hash_bytes);

        // let mut hash_result = [0u8; 32];
        h.result(&mut hash_result[..]);
    }

    
    // clip to fit into field element

    hash_result[0] &= 0x1f; // temporary solution

    let mut repr = Fr::zero().into_repr();
    repr.read_be(&hash_result[..]).expect("pack hash as field element");

    let public_data_commitment = Fr::from_repr(repr).unwrap();

    // print!("Final data commitment as field element = {}\n", public_data_commitment);

    // let instance_for_test_cs = Update {
    //     params: params,
    //     number_of_transactions: TXES_TO_TEST,
    //     old_root: Some(initial_root),
    //     new_root: Some(final_root),
    //     public_data_commitment: Some(public_data_commitment),
    //     block_number: Some(Fr::one()),
    //     total_fee: Some(total_fees),
    //     transactions: witnesses.clone(),
    // };

    // {
    //     let mut cs = TestConstraintSystem::new();

    //     instance_for_test_cs.synthesize(&mut cs).unwrap();

    //     println!("Total of {} constraints", cs.num_constraints());
    //     println!("{} constraints per TX for {} transactions", cs.num_constraints() / TXES_TO_TEST, TXES_TO_TEST);

    //     assert_eq!(cs.num_inputs(), 4);

    //     assert_eq!(cs.get_input(0, "ONE"), Fr::one());
    //     assert_eq!(cs.get_input(1, "old root input/input variable"), initial_root);
    //     assert_eq!(cs.get_input(2, "new root input/input variable"), final_root);
    //     assert_eq!(cs.get_input(3, "rolling hash input/input variable"), public_data_commitment);

    //     // let err = cs.which_is_unsatisfied();
    //     // if err.is_some() {
    //     //     panic!("ERROR satisfying in {}\n", err.unwrap());
    //     // } else {
    //     //     println!("Test constraint system is satisfied");
    //     // }
    // }

    let instance_for_generation = Update {
        params: params,
        number_of_transactions: TXES_TO_TEST,
        old_root: Some(initial_root),
        new_root: Some(final_root),
        public_data_commitment: Some(public_data_commitment),
        block_number: Some(Fr::one()),
        total_fee: Some(total_fees),
        transactions: witnesses.clone(),
    };

    println!("generating setup...");
    let start = PreciseTime::now();
    let circuit_params = generate_random_parameters(instance_for_generation, rng).unwrap();
    println!("setup generated in {} s", start.to(PreciseTime::now()).num_milliseconds() as f64 / 1000.0);

    let pvk = prepare_verifying_key(&circuit_params.vk);

    let instance_for_proof = Update {
        params: params,
        number_of_transactions: TXES_TO_TEST,
        old_root: Some(initial_root),
        new_root: Some(final_root),
        public_data_commitment: Some(public_data_commitment),
        block_number: Some(Fr::one()),
        total_fee: Some(total_fees),
        transactions: witnesses,
    };

    println!("creating proof...");
    let start = PreciseTime::now();
    let proof = create_random_proof(instance_for_proof, &circuit_params, rng).unwrap();
    println!("proof created in {} s", start.to(PreciseTime::now()).num_milliseconds() as f64 / 1000.0);

    let success = verify_proof(&pvk, &proof, &[initial_root, final_root, public_data_commitment]).unwrap();
    assert!(success);

}
