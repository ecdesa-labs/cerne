/// 1 RDEC in wei: like ether, the RDEC has 18 decimals, and MetaMask sends every amount in wei.
pub const WEI_PER_RDEC: u128 = 1_000_000_000_000_000_000;

/// An amount in wei as people read it: `rdec(100_000_000_000_000_000)` is `"0.1 RDEC"`.
pub fn rdec(wei: u128) -> String {
    let whole = wei / WEI_PER_RDEC;
    let fraction = format!("{:018}", wei % WEI_PER_RDEC);
    let fraction = fraction.trim_end_matches('0');

    if fraction.is_empty() {
        format!("{whole} RDEC")
    } else {
        format!("{whole}.{fraction} RDEC")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_amounts_have_no_decimals() {
        assert_eq!(rdec(100 * WEI_PER_RDEC), "100 RDEC");
    }

    #[test]
    fn fractions_keep_only_their_digits() {
        assert_eq!(rdec(WEI_PER_RDEC / 10), "0.1 RDEC");
        assert_eq!(rdec(21_000 * 1_000_000_000), "0.000021 RDEC");
    }
}
