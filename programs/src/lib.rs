//! Binding to ZKVM programs.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    // TODO: fix this
    #[test]
    fn executes_program() {}
}
