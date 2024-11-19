/// The ELF (executable and linkable format) file for the square root program.
pub const SQUARE_ROOT_ELF: &[u8] = include_bytes!("../square-root/elf/square-root");

#[cfg(test)]
mod tests {
    use crate::SQUARE_ROOT_ELF;
    use alloy_primitives::U256;
    use alloy_sol_types::{sol, SolType, SolValue};
    use sp1_sdk::{ProverClient, SP1Stdin};

    type NumberWithSquareRoot = sol! {
        tuple(uint256,uint256)
    };

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
            NumberWithSquareRoot::abi_decode(&output.to_vec(), false).unwrap();
        assert_eq!(number_with_square_root.1, U256::from(3));
    }
}
