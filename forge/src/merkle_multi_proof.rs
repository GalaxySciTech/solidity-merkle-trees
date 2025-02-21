#![cfg(test)]

use crate::{
    forge::{execute_single, single_runner},
    keccak256, runner, Keccak256, Token,
};
use ethers::abi::Uint;
use primitive_types::{H256, U256};
use rs_merkle::{merkelize_sorted, MerkleTree};

#[tokio::test(flavor = "multi_thread")]
async fn multi_merkle_proof() {
    let leaves = (0..167).map(|_| H256::random().as_bytes().to_vec()).collect::<Vec<_>>();
    let leaf_hashes = leaves.iter().map(keccak256).collect::<Vec<[u8; 32]>>();

    let tree = MerkleTree::<Keccak256>::from_leaves(&leaf_hashes);

    let indices = vec![0, 2, 5, 9, 20, 25, 31];
    let leaves_with_indices = indices
        .iter()
        .map(|i| {
            Token::Tuple(vec![
                Token::Uint(U256::from(*i)),
                Token::FixedBytes(leaf_hashes[*i].to_vec()),
            ])
        })
        .collect::<Vec<_>>();

    let proof = tree.proof_2d(&indices);

    let args = proof
        .into_iter()
        .map(|layers| {
            let layers = layers
                .into_iter()
                .map(|(index, node)| {
                    Token::Tuple(vec![
                        Token::Uint(U256::from(index)),
                        Token::FixedBytes(node.to_vec()),
                    ])
                })
                .collect::<Vec<_>>();
            Token::Array(layers)
        })
        .collect::<Vec<_>>();

    let mut runner = runner();
    let (mut contract, address) = single_runner(&mut runner, "MerkleMultiProofTest").await;

    let calculated = execute_single::<_, [u8; 32]>(
        &mut contract,
        address.clone(),
        "CalculateRoot",
        (args.clone(), leaves_with_indices),
    )
    .unwrap();

    assert_eq!(tree.root().unwrap(), calculated);

    {
        {
            for i in [2usize, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048] {
                let calculated = execute_single::<_, u32>(
                    &mut contract,
                    address.clone(),
                    "TreeHeight",
                    (Token::Uint(Uint::from(i))),
                )
                .unwrap();
                assert_eq!(calculated as u32, i.ilog2());
            }

            let calculated = execute_single::<_, u32>(
                &mut contract,
                address.clone(),
                "TreeHeight",
                (Token::Uint(Uint::from(leaf_hashes.len()))),
            )
            .unwrap();

            let len = merkelize_sorted::<Keccak256>(leaf_hashes.clone()).len();

            assert_eq!(calculated as usize, len);
        }

        let beefy_root =
            beefy_merkle_tree::merkle_root::<sp_runtime::traits::Keccak256, _>(leaves.clone());

        assert_eq!(beefy_root, H256(calculated));
    }
}
