// Library to generate a EVM verifier contract

use bellman::{Circuit, ConstraintSystem, SynthesisError};
use ff::{Field, PrimeField};
use pairing::{Engine, CurveAffine, EncodedPoint};
use bellman::groth16;
use pairing::bn256::{Bn256, Fr};
use std::fmt;

fn unpack<T: CurveAffine>(t: &T) -> Vec<String>
{
    t.into_uncompressed().as_ref().chunks(32).map(|c| "0x".to_owned() + &hex::encode(c)).collect()
}

fn unpack_g1<E: Engine>(point: & E::G1Affine) -> Vec<String> {
    let uncompressed = point.into_uncompressed();
    let uncompressed_slice = uncompressed.as_ref();

    uncompressed_slice.chunks(32).map(|c| "0x".to_owned() + &hex::encode(c)).collect()
}

fn unpack_g2<E: Engine>(point: & E::G2Affine) -> Vec<String> {
    let uncompressed = point.into_uncompressed();
    let uncompressed_slice = uncompressed.as_ref();
    uncompressed_slice.chunks(32).map(|c| "0x".to_owned() + &hex::encode(c)).collect()

    // let to_reorder: Vec<String> = uncompressed_slice.chunks(32).map(|c| "0x".to_owned() + &hex::encode(c)).collect();

    // vec![to_reorder[1].clone(), to_reorder[0].clone(), to_reorder[3].clone(), to_reorder[2].clone()]
}

const SHIFT: &str = "        ";

fn render_array(name: &str, allocate: bool, values: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push('\n');
    let flattened: Vec<&String> = values.into_iter().flatten().collect();
    if allocate {
        out.push_str(&format!("{}{} = new uint256[]({});\n", SHIFT, name, flattened.len()));
    }
    for (i, s) in flattened.iter().enumerate() {
        out.push_str(&format!("{}{}[{}] = {};\n", SHIFT, name, i, s));
    }
    out
}

pub fn hardcode_vk<E: Engine>(vk: &groth16::VerifyingKey<E>) -> String {
    let mut out = String::new();

    let values = &[
        unpack_g1::<E>(&vk.alpha_g1),
        unpack_g2::<E>(&vk.beta_g2),
        unpack_g2::<E>(&vk.gamma_g2),
        unpack_g2::<E>(&vk.delta_g2),
    ];
    out.push_str(&render_array("vk", false, values));

    let ic: Vec<Vec<String>> = vk.ic.iter().map(unpack_g1::<E>).collect();
    out.push_str(&render_array("gammaABC", true, ic.as_slice()));

    out
}