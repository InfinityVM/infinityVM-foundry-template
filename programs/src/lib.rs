//! Generated crate containing the image ID and ELF binary of the build guest.
include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    use alloy_primitives::U256;
    use alloy_sol_types::{sol, SolValue, SolType};
    use risc0_zkvm::{ExecutorEnv, Executor, LocalProver};

    type NumberWithSquareRoot = sol! {
        tuple(uint256,uint256)
    };

    const MAX_CYCLES: u64 = 1_000_000;
    
    #[test]
    fn proves_square_root() {
        // Input for program
        let number = U256::from(9);

        // Execute program on input
        let env = ExecutorEnv::builder().session_limit(Some(MAX_CYCLES)).write_slice(&number.abi_encode()).build().unwrap();
        let prover = LocalProver::new("locals only");
        let prove_info = prover.execute(env, super::SQUARE_ROOT_ELF).unwrap();
        
        // Decode output and check result
        let number_with_square_root = NumberWithSquareRoot::abi_decode(&prove_info.journal.bytes, false).unwrap();
        assert_eq!(number_with_square_root.1, U256::from(3));
    }
}
