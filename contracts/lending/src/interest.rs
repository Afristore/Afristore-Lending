// ------------------------------------------------------------
// interest.rs — Per-month interest accrual schedule with
// partial-month proration.
//
// The protocol prices loans on a per-month schedule (basis
// points per 30-day month) rather than a flat APR, so longer
// loans are progressively more expensive. The last entry in
// the schedule repeats for any month beyond its length.
//

use crate::types::Position;

/// Seconds in one day (24h).
const SECONDS_PER_DAY: u64 = 86_400;

/// Days in one month for the accrual schedule.
const DAYS_PER_MONTH: u64 = 30;

/// One basis point denominator (10_000 bps = 100%).
const BPS_DENOMINATOR: i128 = 10_000;

/// Compute the accrued interest in USD for a position at `now`.
///
/// Returns 0 if `now` is at or before the position start time.
/// Panics if the position's interest schedule is empty.
///
/// Pure function — no `Env`, no storage access.
#[allow(dead_code)]
pub fn accrued_interest_usd(position: &Position, now: u64) -> i128 {
    if now <= position.start_time {
        return 0;
    }

    let schedule = &position.interest_schedule_bps;
    if schedule.is_empty() {
        panic!("interest schedule must not be empty");
    }

    let elapsed_days = (now - position.start_time) / SECONDS_PER_DAY;
    let full_months = elapsed_days / DAYS_PER_MONTH;
    let partial_days = elapsed_days % DAYS_PER_MONTH;

    let price = position.declared_price_usd;
    let len = schedule.len();
    let last = len - 1;

    // Full months: the rate for month `m` repeats the final entry
    // once the schedule length is exhausted.
    let mut total: i128 = 0;
    for month in 0..full_months {
        let idx = if month >= last as u64 {
            last
        } else {
            month as u32
        };
        let rate = schedule.get(idx).unwrap();
        total += price * i128::from(rate) / BPS_DENOMINATOR;
    }

    // Partial month: prorate the rate in effect during the partial
    // month by the fraction of the month that has elapsed.
    let partial_idx = if full_months >= last as u64 {
        last
    } else {
        full_months as u32
    };
    let partial_rate = schedule.get(partial_idx).unwrap();
    total += price * i128::from(partial_rate) / BPS_DENOMINATOR * i128::from(partial_days)
        / DAYS_PER_MONTH as i128;

    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Position, PositionStatus};
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{vec, Address, Env, Vec};

    /// Build a position with the given price (7-decimal fixed point),
    /// interest schedule (bps per month) and start time.
    fn position(env: &Env, price: i128, schedule: Vec<u32>, start: u64) -> Position {
        Position {
            id: 1,
            listing_id: 1,
            lender: Address::generate(env),
            borrower: Address::generate(env),
            nft_contract: Address::generate(env),
            token_id: 1,
            declared_price_usd: price,
            collateral_currency: Address::generate(env),
            collateral_amount: 0,
            interest_schedule_bps: schedule,
            liquidation_threshold_bps: 0,
            start_time: start,
            max_duration_secs: 0,
            status: PositionStatus::Active,
        }
    }

    #[test]
    fn elapsed_zero_returns_zero() {
        let env = Env::default();
        let pos = position(&env, 100_000_000, vec![&env, 500u32, 600u32], 1_000_000);

        assert_eq!(accrued_interest_usd(&pos, 1_000_000), 0);
        assert_eq!(accrued_interest_usd(&pos, 999_999), 0);
    }

    #[test]
    fn partial_first_month_is_prorated() {
        // 500 bps = 5% / month. 15 days into month 1:
        // 100_000_000 * 500 / 10_000 * 15 / 30 = 2_500_000
        let env = Env::default();
        let pos = position(&env, 100_000_000, vec![&env, 500u32, 600u32], 0);

        assert_eq!(accrued_interest_usd(&pos, 15 * 86_400), 2_500_000);
    }

    #[test]
    fn full_first_month_uses_first_rate() {
        // Exactly 30 days: one full month at 500 bps.
        let env = Env::default();
        let pos = position(&env, 100_000_000, vec![&env, 500u32, 600u32], 0);

        assert_eq!(accrued_interest_usd(&pos, 30 * 86_400), 5_000_000);
    }

    #[test]
    fn month_two_rate_kicks_in_for_partial_month() {
        // 45 days: month 1 full (500 bps) + 15 days of month 2 (600 bps):
        // 5_000_000 + 100_000_000 * 600 / 10_000 * 15 / 30 = 8_000_000
        let env = Env::default();
        let pos = position(&env, 100_000_000, vec![&env, 500u32, 600u32], 0);

        assert_eq!(accrued_interest_usd(&pos, 45 * 86_400), 8_000_000);
    }

    #[test]
    fn full_month_two_uses_second_rate() {
        // 60 days: month 1 (500 bps) + month 2 (600 bps).
        let env = Env::default();
        let pos = position(&env, 100_000_000, vec![&env, 500u32, 600u32], 0);

        assert_eq!(accrued_interest_usd(&pos, 60 * 86_400), 11_000_000);
    }

    #[test]
    fn last_rate_repeats_after_schedule_end() {
        // 90 days = 3 months. Rates: month 1 = 500, month 2 = 600,
        // month 3 = last entry repeats = 600. Total = 17_000_000.
        let env = Env::default();
        let pos = position(&env, 100_000_000, vec![&env, 500u32, 600u32], 0);

        assert_eq!(accrued_interest_usd(&pos, 90 * 86_400), 17_000_000);
    }

    #[test]
    #[should_panic(expected = "interest schedule must not be empty")]
    fn empty_schedule_panics() {
        let env = Env::default();
        let pos = position(&env, 100_000_000, vec![&env], 0);

        accrued_interest_usd(&pos, 15 * 86_400);
    }
}
