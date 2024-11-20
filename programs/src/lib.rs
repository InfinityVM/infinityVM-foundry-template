/// The ELF (executable and linkable format) file for the square root program.
pub const SQUARE_ROOT_ELF: &[u8] = include_bytes!("../../target/sp1/square-root/square-root");

#[cfg(test)]
mod tests {
    use crate::SQUARE_ROOT_ELF;
    use alloy::{
        sol,
        sol_types::{SolType, SolValue},
    };
    use alloy_primitives::U256;
    use sp1_sdk::{ProverClient, SP1Stdin};

    sol! {
        struct NumberWithSquareRoot {
            uint256 number;
            uint256 square_root;
        }
    }

    const MAX_CYCLES: u64 = 1_000_000;

    #[test]
    fn executes_square_root() {
        // Input for program
        let number = U256::from(9);
        let onchain_input = number.abi_encode();

        let mut stdin = SP1Stdin::new();
        stdin.write_slice(&onchain_input);

        let client = ProverClient::new();
        let (output, _) =
            client.execute(SQUARE_ROOT_ELF, stdin).max_cycles(MAX_CYCLES).run().unwrap();

        // Decode output and check result
        let number_with_square_root =
            <NumberWithSquareRoot as SolType>::abi_decode(&output.to_vec(), false).unwrap();
        assert_eq!(number_with_square_root.square_root, U256::from(3));
    }
}
