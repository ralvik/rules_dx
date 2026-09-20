//! CC opt-out fixture library (issue #471).
//!
//! Proves the kept `use_cc_toolchain = False` execution path: the build
//! script above stamps the crate without any C/C++ toolchain input.

#[cfg(has_cc_optout_stamp)]
pub fn stamped() -> u32 {
    1
}

#[cfg(not(has_cc_optout_stamp))]
pub fn stamped() -> u32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_present() {
        assert_eq!(stamped(), 1);
    }
}
