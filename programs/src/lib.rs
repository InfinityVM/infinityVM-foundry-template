include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    use alloy_primitives::U256;
    use alloy_sol_types::{sol, SolType, SolValue};
    use risc0_zkvm::{Executor, ExecutorEnv, LocalProver};

    type NumberWithSquareRoot = sol! {
        tuple(uint256,uint256)
    };

    const MAX_CYCLES: u64 = 1_000_000;

    #[test]
    fn executes_square_root() {
        // Input for program
        let number = U256::from(9);
        let onchain_input = number.abi_encode();
        let onchain_input_len = onchain_input.len() as u32;

        // Execute program on input, without generating a ZK proof
        let env = ExecutorEnv::builder()
            .session_limit(Some(MAX_CYCLES))
            .write(&onchain_input_len)
            .unwrap()
            .write_slice(&onchain_input)
            .build()
            .unwrap();
        let executor = LocalProver::new("locals only");
        let execute_info = executor.execute(env, super::SQUARE_ROOT_ELF).unwrap();

        // Decode output and check result
        let number_with_square_root =
            NumberWithSquareRoot::abi_decode(&execute_info.journal.bytes, false).unwrap();
        assert_eq!(number_with_square_root.1, U256::from(3));
    }
}
