#[cfg(test)]
mod vault_fcn {
    use crate::state::traits::VaultDepositorBase;
    use crate::withdraw_request::WithdrawRequest;
    use crate::{Vault, VaultDepositor, WithdrawUnit};
    use anchor_lang::prelude::Pubkey;
    use drift::math::constants::{ONE_YEAR, QUOTE_PRECISION_U64};
    use drift::math::insurance::if_shares_to_vault_amount as depositor_shares_to_vault_amount;

    #[test]
    fn test_manager_withdraw() {
        let now = 0;
        let mut vault = Vault::default();
        let mut vp = None;
        vault.management_fee = 1000; // 10 bps
        vault.redeem_period = 60;

        let mut vault_equity = 0;
        let amount = 100_000_000; // $100
        vault
            .manager_deposit(&mut vp, amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;
        vault_equity -= 1;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        assert_eq!(vault.total_deposits, 100000000);
        assert_eq!(vault.manager_total_deposits, 100000000);
        assert_eq!(vault.manager_total_withdraws, 0);

        vault
            .manager_request_withdraw(&mut vp, amount - 1, WithdrawUnit::Token, vault_equity, now)
            .unwrap();

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        assert_eq!(vault.total_deposits, 100000000);
        assert_eq!(vault.manager_total_deposits, 100000000);
        assert_eq!(vault.manager_total_withdraws, 0);

        let err = vault
            .manager_withdraw(&mut vp, vault_equity, now + 50)
            .is_err();
        assert!(err);

        let withdraw = vault
            .manager_withdraw(&mut vp, vault_equity, now + 60)
            .unwrap();
        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.total_deposits, 100000000);
        assert_eq!(vault.manager_total_deposits, 100000000);
        assert_eq!(vault.manager_total_withdraws, 99999999);
        assert_eq!(withdraw, 99999999);
    }

    #[test]
    fn test_smol_management_fee() {
        let now = 0;
        let mut vault = Vault::default();
        let mut vp = None;
        vault.management_fee = 1000; // 10 bps

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 100 * QUOTE_PRECISION_U64;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vd.deposit(amount, vault_equity, &mut vault, &mut vp, now)
            .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200000000);
        assert_eq!(vault.last_fee_update_ts, 0);
        vault_equity += amount;

        let user_eq_before =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(user_eq_before, 100000000);

        vault
            .apply_fee(&mut vp, vault_equity, now + ONE_YEAR as i64)
            .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200200200);

        let oo =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(oo, 99900000);

        assert_eq!(vault.last_fee_update_ts, now + ONE_YEAR as i64);
    }

    #[test]
    fn test_excessive_management_fee() {
        let now = 1000;
        let mut vault = Vault::default();
        let mut vp = None;
        vault.management_fee = 1000000;
        vault.last_fee_update_ts = 0;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 100 * QUOTE_PRECISION_U64;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vd.deposit(amount, vault_equity, &mut vault, &mut vp, now)
            .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200000000);
        assert_eq!(vault.shares_base, 0);
        assert_eq!(vault.last_fee_update_ts, 1000);
        vault_equity += amount;

        vault
            .apply_fee(&mut vp, vault_equity, now + ONE_YEAR as i64)
            .unwrap();
        assert_eq!(vault.user_shares, 10);
        assert_eq!(vault.total_shares, 2000000000);
        assert_eq!(vault.shares_base, 7);

        let vd_amount_left =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(vd_amount_left, 1);
        assert_eq!(vault.last_fee_update_ts, now + ONE_YEAR as i64);
    }

    #[test]
    fn test_management_fee_high_frequency() {
        // asymptotic nature of calling -100% annualized on shorter time scale
        let mut now = 0;
        let mut vault = Vault::default();
        let mut vp = None;
        vault.management_fee = 1000000; // 100%
        vault.last_fee_update_ts = 0;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 100 * QUOTE_PRECISION_U64;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vd.deposit(amount, vault_equity, &mut vault, &mut vp, now)
            .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200000000);
        assert_eq!(vault.shares_base, 0);
        // assert_eq!(vault.last_fee_update_ts(, 1000);
        vault_equity += amount;

        while now < ONE_YEAR as i64 {
            vault.apply_fee(&mut vp, vault_equity, now).unwrap();
            now += 60 * 60 * 24 * 7; // every week
        }
        vault.apply_fee(&mut vp, vault_equity, now).unwrap();

        let vd_amount_left =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(vd_amount_left, 35832760); // ~$35
        assert_eq!(vault.last_fee_update_ts, now);
    }

    #[test]
    fn test_manager_alone_deposit_withdraw() {
        let mut now = 123456789;
        let mut vault = Vault::default();
        let mut vp = None;
        vault.management_fee = 100; // .01%
        vault.last_fee_update_ts = now;
        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vault
            .manager_deposit(&mut vp, amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        now += 60 * 60;

        let vault_manager_amount = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount, 100000000);

        vault
            .manager_request_withdraw(&mut vp, amount, WithdrawUnit::Token, vault_equity, now)
            .unwrap();

        let withdrew = vault.manager_withdraw(&mut vp, vault_equity, now).unwrap();
        assert_eq!(amount, withdrew);
        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 0);

        let vault_manager_amount_after = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount_after, 0);
    }

    #[test]
    fn test_negative_management_fee() {
        let now = 0;
        let mut vault = Vault::default();
        let mut vp = None;
        vault.management_fee = -2_147_483_648; // -214700% annualized (manager pays 24% hourly, .4% per minute)

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 100 * QUOTE_PRECISION_U64;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vd.deposit(amount, vault_equity, &mut vault, &mut vp, now)
            .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200000000);
        assert_eq!(vault.last_fee_update_ts, 0);
        vault_equity += amount;

        let user_eq_before =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(user_eq_before, 100000000);

        // one second since inception
        vault.apply_fee(&mut vp, vault_equity, now + 1_i64).unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 199986200);

        let oo =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(oo, 100006900); // up half a cent

        // one minute since inception
        vault
            .apply_fee(&mut vp, vault_equity, now + 60_i64)
            .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 199185855);

        let oo =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(oo, 100408736); // up 40 cents

        // one year since inception
        vault
            .apply_fee(&mut vp, vault_equity, now + ONE_YEAR as i64)
            .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 100000000);

        let oo =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(oo, 200000000); // up $100

        assert_eq!(vault.last_fee_update_ts, now + ONE_YEAR as i64);
    }

    #[test]
    fn test_negative_management_fee_manager_alone() {
        let mut now = 0;
        let mut vault = Vault::default();
        let mut vp = None;
        vault.management_fee = -2_147_483_648; // -214700% annualized (manager pays 24% hourly, .4% per minute)
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        now += 100000;
        vault
            .manager_deposit(&mut vp, amount, vault_equity, now)
            .unwrap();

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, amount as u128);
        assert_eq!(vault.last_fee_update_ts, now);
        vault_equity += amount;

        now += 100000;
        vault
            .manager_request_withdraw(&mut vp, amount, WithdrawUnit::Token, vault_equity, now)
            .unwrap();
        let withdrew = vault.manager_withdraw(&mut vp, vault_equity, now).unwrap();
        assert_eq!(withdrew, amount);
    }

    #[test]
    fn test_manager_deposit_withdraw_with_user_flat() {
        let mut now = 123456789;
        let mut vault = Vault::default();
        let mut vp = None;
        vault.management_fee = 0;
        vault.profit_share = 150_000; // 15%

        vault.last_fee_update_ts = now;
        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vault
            .manager_deposit(&mut vp, amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        now += 60 * 60;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        vd.deposit(amount * 20, vault_equity, &mut vault, &mut vp, now)
            .unwrap(); // new user deposits $2000
        now += 60 * 60;
        assert_eq!(vault.user_shares, 2000000000);
        assert_eq!(vault.total_shares, 2000000000 + 100000000);
        vault_equity += amount * 20;

        now += 60 * 60 * 24; // 1 day later

        vd.apply_profit_share(vault_equity, &mut vault, &mut vp)
            .unwrap();
        vault.apply_fee(&mut vp, vault_equity, now).unwrap();

        let vault_manager_amount = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount, 100000000);
        vault
            .manager_request_withdraw(&mut vp, amount, WithdrawUnit::Token, vault_equity, now)
            .unwrap();

        let withdrew = vault.manager_withdraw(&mut vp, vault_equity, now).unwrap();
        assert_eq!(amount, withdrew);
        assert_eq!(vault.user_shares, 2000000000);
        assert_eq!(vault.total_shares, 2000000000);

        let vault_manager_amount_after = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount_after, 0);
    }

    #[test]
    fn test_manager_deposit_withdraw_with_user_manager_fee_loss() {
        let mut now = 123456789;
        let mut vault = Vault::default();
        let mut vp = None;
        vault.management_fee = 100; // .01%
        vault.profit_share = 150000; // 15%

        vault.last_fee_update_ts = now;
        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vault
            .manager_deposit(&mut vp, amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        now += 60 * 60;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        vd.deposit(amount * 20, vault_equity, &mut vault, &mut vp, now)
            .unwrap(); // new user deposits $2000
        now += 60 * 60;
        assert_eq!(vault.user_shares, 2000000000);
        assert_eq!(vault.total_shares, 2000000000 + 100000000);
        vault_equity += amount * 20;

        let mut cnt = 0;
        while (vault.total_shares == 2000000000 + 100000000) && cnt < 400 {
            now += 60 * 60 * 24; // 1 day later

            vd.apply_profit_share(vault_equity, &mut vault, &mut vp)
                .unwrap();
            vault.apply_fee(&mut vp, vault_equity, now).unwrap();
            // crate::msg!("vault last ts: {} vs {}", vault.last_fee_update_ts, now);
            cnt += 1;
        }

        assert_eq!(cnt, 4); // 4 days

        let vault_manager_amount = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount, 100001999);
        vault
            .manager_request_withdraw(&mut vp, amount, WithdrawUnit::Token, vault_equity, now)
            .unwrap();

        let withdrew = vault.manager_withdraw(&mut vp, vault_equity, now).unwrap();
        assert_eq!(amount, withdrew);
        assert_eq!(vault.user_shares, 2000000000);
        assert_eq!(vault.total_shares, 2000002000);
        vault_equity -= withdrew;

        let vault_manager_amount_after = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount_after, 1999); // gainz

        let vd_amount = depositor_shares_to_vault_amount(
            vd.checked_vault_shares(&vault).unwrap(),
            vault.total_shares,
            vault_equity,
        )
        .unwrap();
        assert_eq!(vd_amount, 1999998000); // loss

        assert_eq!(vd_amount + vault_manager_amount_after, vault_equity - 1);
    }

    #[test]
    fn test_manager_deposit_withdraw_with_user_gain() {
        let mut now = 123456789;
        let mut vault = Vault::default();
        let mut vp = None;
        vault.management_fee = 100; // .01%
        vault.profit_share = 150000; // 15%

        vault.last_fee_update_ts = now;
        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vault
            .manager_deposit(&mut vp, amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        now += 60 * 60;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        vd.deposit(amount * 20, vault_equity, &mut vault, &mut vp, now)
            .unwrap(); // new user deposits $2000
        now += 60 * 60;
        assert_eq!(vault.user_shares, 2000000000);
        assert_eq!(vault.total_shares, 2000000000 + 100000000);
        vault_equity += amount * 20;

        // up 50%
        vault_equity *= 3;
        vault_equity /= 2;

        assert_eq!(vault_equity, 3_150_000_000);

        let mut cnt = 0;
        while (vault.total_shares == 2000000000 + 100000000) && cnt < 400 {
            now += 60 * 60 * 24; // 1 day later

            vd.apply_profit_share(vault_equity, &mut vault, &mut vp)
                .unwrap();
            vault.apply_fee(&mut vp, vault_equity, now).unwrap();
            // crate::msg!("vault last ts: {} vs {}", vault.last_fee_update_ts, now);
            cnt += 1;
        }

        assert_eq!(cnt, 4); // 4 days
        assert_eq!(
            vd.cumulative_profit_share_amount,
            (1000 * QUOTE_PRECISION_U64) as i64
        );
        assert_eq!(vd.net_deposits, (2000 * QUOTE_PRECISION_U64) as i64);

        let vault_manager_amount = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount, 300002849); //$300??

        vault
            .manager_request_withdraw(&mut vp, amount, WithdrawUnit::Token, vault_equity, now)
            .unwrap();
        assert_eq!(amount, vault.last_manager_withdraw_request.value);

        let withdrew = vault.manager_withdraw(&mut vp, vault_equity, now).unwrap();
        assert_eq!(amount - 1, withdrew); // todo: slight round out of favor
        assert_eq!(vault.user_shares, 1900000000);
        assert_eq!(vault.total_shares, 2033335367);
        vault_equity -= withdrew;

        let vault_manager_amount_after = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount_after, 200_002_850); // gainz

        let vd_amount = depositor_shares_to_vault_amount(
            vd.checked_vault_shares(&vault).unwrap(),
            vault.total_shares,
            vault_equity,
        )
        .unwrap();
        assert_eq!(vd_amount, 2_849_997_150); // gainz

        assert_eq!(vd_amount + vault_manager_amount_after, vault_equity - 1);
    }

    #[test]
    fn test_vd_withdraw_on_drawdown() {
        let mut now = 123456789;
        let vault = &mut Vault::default();

        let mut vault_equity: u64 = 0;
        let deposit_amount: u64 = 100 * QUOTE_PRECISION_U64;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.shares_base, 0);

        let vd = &mut VaultDepositor::new(
            Pubkey::default(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            now,
        );
        vd.deposit(deposit_amount, vault_equity, vault, &mut None, now)
            .unwrap(); // new user deposits $2000
        let vd_shares = vd.get_vault_shares();
        now += 100;
        assert_eq!(vault.user_shares, deposit_amount as u128);
        assert_eq!(vault.total_shares, deposit_amount as u128);
        assert_eq!(vd.get_vault_shares(), vault.user_shares);
        assert_eq!(vd.get_vault_shares_base(), vault.shares_base);
        vault_equity += deposit_amount;

        // down 50%
        vault_equity /= 2;
        now += 100;

        // user withdraws
        vd.request_withdraw(
            vd_shares as u64,
            WithdrawUnit::Shares,
            vault_equity,
            vault,
            &mut None,
            now,
        )
        .expect("request withdraw");

        assert_eq!(
            vd.last_withdraw_request,
            WithdrawRequest {
                shares: vd_shares,
                value: vault_equity,
                ts: now,
            }
        );

        // down another 50%
        vault_equity /= 2;
        now += 100;

        let (withdraw_amount, finishing_liquidation) = vd
            .withdraw(vault_equity, vault, &mut None, now)
            .expect("withdraw");
        assert_eq!(withdraw_amount, vault_equity);
        assert!(!finishing_liquidation);
    }

    #[test]
    fn test_vd_request_withdraw_after_rebase() {
        let mut now = 123456789;
        let vault = &mut Vault::default();
        let mut vp = None;

        let mut vault_equity: u64 = 0;
        let deposit_amount: u64 = 100 * QUOTE_PRECISION_U64;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.shares_base, 0);

        let vd = &mut VaultDepositor::new(
            Pubkey::default(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            now,
        );
        vd.deposit(deposit_amount, vault_equity, vault, &mut vp, now)
            .unwrap(); // new user deposits $2000
        let vd_shares = vd.checked_vault_shares(vault).unwrap();
        now += 100;
        assert_eq!(vault.user_shares, deposit_amount as u128);
        assert_eq!(vault.total_shares, deposit_amount as u128);
        assert_eq!(vault.shares_base, 0);
        assert_eq!(vd.checked_vault_shares(vault).unwrap(), vault.user_shares);
        assert_eq!(vd.vault_shares_base, vault.shares_base);
        vault_equity += deposit_amount;

        // down 99.9%
        vault_equity /= 1000;
        now += 100;

        // request_withdraw triggers rebase
        vd.request_withdraw(
            vd_shares as u64,
            WithdrawUnit::Shares,
            vault_equity,
            vault,
            &mut vp,
            now,
        )
        .expect("request withdraw");

        assert_eq!(
            vd.last_withdraw_request,
            WithdrawRequest {
                shares: vd_shares / 100, // expected rebase by expo_diff 2
                value: vault_equity,
                ts: now,
            }
        );

        println!(
            "last_withdraw_request 1: {:?}, vault eq: {}",
            vd.last_withdraw_request, vault_equity
        );

        // // down another 50%
        // vault_equity /= 2;
        now += 100;

        let (withdraw_amount, finishing_liquidation) = vd
            .withdraw(vault_equity, vault, &mut vp, now)
            .expect("withdraw");
        assert_eq!(withdraw_amount, vault_equity);
        println!(
            "final withdraw_amount 1: {}, vault eq: {}",
            withdraw_amount, vault_equity
        );
        assert!(!finishing_liquidation);
    }

    #[test]
    fn test_vd_request_withdraw_before_rebase() {
        let mut now = 123456789;
        let vault = &mut Vault::default();
        let mut vp = None;

        let mut vault_equity: u64 = 0;
        let deposit_amount: u64 = 100 * QUOTE_PRECISION_U64;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.shares_base, 0);

        let vd = &mut VaultDepositor::new(
            Pubkey::default(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            now,
        );
        vd.deposit(deposit_amount, vault_equity, vault, &mut vp, now)
            .unwrap(); // new user deposits $2000
        let vd_shares = vd.checked_vault_shares(vault).unwrap();
        now += 100;
        assert_eq!(vault.user_shares, deposit_amount as u128);
        assert_eq!(vault.total_shares, deposit_amount as u128);
        assert_eq!(vault.shares_base, 0);
        assert_eq!(vd.checked_vault_shares(vault).unwrap(), vault.user_shares);
        assert_eq!(vd.vault_shares_base, vault.shares_base);
        vault_equity += deposit_amount;

        vd.request_withdraw(
            vd_shares as u64,
            WithdrawUnit::Shares,
            vault_equity,
            vault,
            &mut vp,
            now,
        )
        .expect("request withdraw");

        assert_eq!(
            vd.last_withdraw_request,
            WithdrawRequest {
                shares: vd_shares,
                value: vault_equity,
                ts: now,
            }
        );
        println!(
            "last_withdraw_request 2: {:?}, vault equity: {}",
            vd.last_withdraw_request, vault_equity
        );

        // down 99.9%
        vault_equity /= 1000;
        now += 100;

        // withdraw will trigger a rebase
        let (withdraw_amount, finishing_liquidation) = vd
            .withdraw(vault_equity, vault, &mut None, now)
            .expect("withdraw");
        assert_eq!(withdraw_amount, vault_equity);
        println!(
            "final withdraw_amount 2: {}, vault eq: {}",
            withdraw_amount, vault_equity
        );
        assert!(!finishing_liquidation);
    }
}

#[cfg(test)]
mod vault_v1_fcn {
    use std::cell::RefCell;

    use anchor_lang::prelude::Pubkey;
    use drift::math::constants::{ONE_YEAR, QUOTE_PRECISION_U64};
    use drift::math::insurance::if_shares_to_vault_amount as depositor_shares_to_vault_amount;

    use crate::state::{Vault, VaultProtocol};
    use crate::{VaultDepositor, WithdrawUnit};

    const USER_SHARES_AFTER_1500_BPS_FEE: u64 = 99_850_025;

    #[test]
    fn test_manager_withdraw_v1() {
        let now = 0;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = 1000; // 10 bps
        vp.borrow_mut().protocol_fee = 500; // 5 bps
        vault.redeem_period = 60;

        let mut vault_equity = 0;
        let amount = 100_000_000; // $100
        vault
            .manager_deposit(&mut Some(vp.borrow_mut()), amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;
        vault_equity -= 1;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        assert_eq!(vault.total_deposits, 100000000);
        assert_eq!(vault.manager_total_deposits, 100000000);
        assert_eq!(vault.manager_total_withdraws, 0);

        vault
            .manager_request_withdraw(
                &mut Some(vp.borrow_mut()),
                amount - 1,
                WithdrawUnit::Token,
                vault_equity,
                now,
            )
            .unwrap();

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        assert_eq!(vault.total_deposits, 100000000);
        assert_eq!(vault.manager_total_deposits, 100000000);
        assert_eq!(vault.manager_total_withdraws, 0);

        let err = vault
            .manager_withdraw(&mut Some(vp.borrow_mut()), vault_equity, now + 50)
            .is_err();
        assert!(err);

        let withdraw = vault
            .manager_withdraw(&mut Some(vp.borrow_mut()), vault_equity, now + 60)
            .unwrap();
        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.total_deposits, 100000000);
        assert_eq!(vault.manager_total_deposits, 100000000);
        assert_eq!(vault.manager_total_withdraws, 99999999);
        assert_eq!(withdraw, 99999999);
    }

    #[test]
    fn test_management_and_protocol_fee_v1() {
        let now = 0;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = 1000; // 10 bps
        vp.borrow_mut().protocol_fee = 500; // 5 bps

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 100 * QUOTE_PRECISION_U64;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vd.deposit(
            amount,
            vault_equity,
            &mut vault,
            &mut Some(vp.borrow_mut()),
            now,
        )
        .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200000000);
        assert_eq!(vault.last_fee_update_ts, 0);
        vault_equity += amount;

        let user_eq_before =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(user_eq_before, 100_000_000);

        vault
            .apply_fee(
                &mut Some(vp.borrow_mut()),
                vault_equity,
                now + ONE_YEAR as i64,
            )
            .unwrap();
        assert_eq!(vault.user_shares, 100_000_000);

        let manager_shares = vault
            .get_manager_shares(&mut Some(vp.borrow_mut()))
            .unwrap();
        println!("manager shares: {}", manager_shares);
        assert_eq!(manager_shares, 100_000_000 + 200_400);

        let protocol_shares = vault.get_protocol_shares(&mut Some(vp.borrow_mut()));
        println!("protocol shares: {}", protocol_shares);
        assert_eq!(protocol_shares, 100_000);

        assert_eq!(vault.total_shares, 200_000_000 + 200_400 + 100_000);
        println!("total shares: {}", vault.total_shares);

        let oo =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        // 1000 mgmt fee + 500 protocol fee = 1500 bps fee
        // this is user shares after 1500 bps fee
        assert_eq!(oo, USER_SHARES_AFTER_1500_BPS_FEE);

        assert_eq!(vault.last_fee_update_ts, now + ONE_YEAR as i64);
    }

    #[test]
    fn test_odd_management_and_protocol_fee_v1() {
        let now = 0;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = 1001; // 10.01 bps
        vp.borrow_mut().protocol_fee = 499; // 4.99 bps

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 100 * QUOTE_PRECISION_U64;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vd.deposit(
            amount,
            vault_equity,
            &mut vault,
            &mut Some(vp.borrow_mut()),
            now,
        )
        .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200000000);
        assert_eq!(vault.last_fee_update_ts, 0);
        vault_equity += amount;

        let user_eq_before =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(user_eq_before, 100_000_000);

        vault
            .apply_fee(
                &mut Some(vp.borrow_mut()),
                vault_equity,
                now + ONE_YEAR as i64,
            )
            .unwrap();
        assert_eq!(vault.user_shares, 100_000_000);

        let manager_shares = vault
            .get_manager_shares(&mut Some(vp.borrow_mut()))
            .unwrap();
        println!("manager shares: {}", manager_shares);
        // 200_400 shares at 1000 point profit share
        // 200_600 shares at 1001 point profit share
        // 200_400 / 1000 = 200.4
        // 200_600 / 1001 = 200.399999 (ends up as 200.4 in the program due to u64 rounding)
        assert_eq!(manager_shares, 100_000_000 + 200_600);

        let protocol_shares = vault.get_protocol_shares(&mut Some(vp.borrow_mut()));
        println!("protocol shares: {}", protocol_shares);
        // 100_000 shares at 500 point profit share
        // 99_800 shares at 499 point profit share
        // 100_000 / 500 = 200
        // 99_800 / 499 = 200
        assert_eq!(protocol_shares, 99_800);

        assert_eq!(vault.total_shares, 200_000_000 + 200_600 + 99_800);
        println!("total shares: {}", vault.total_shares);

        let oo =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        // 1001 mgmt fee + 499 protocol fee = 1500 bps fee
        // this is user shares after 1500 bps fee
        assert_eq!(oo, USER_SHARES_AFTER_1500_BPS_FEE);

        assert_eq!(vault.last_fee_update_ts, now + ONE_YEAR as i64);
    }

    #[test]
    fn test_protocol_fee_alone_v1() {
        let now = 0;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = 0; // 0 bps
        vp.borrow_mut().protocol_fee = 500; // 5 bps

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 100 * QUOTE_PRECISION_U64;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vd.deposit(
            amount,
            vault_equity,
            &mut vault,
            &mut Some(vp.borrow_mut()),
            now,
        )
        .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200000000);
        assert_eq!(vault.last_fee_update_ts, 0);
        vault_equity += amount;

        let user_eq_before =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(user_eq_before, 100_000_000);

        vault
            .apply_fee(
                &mut Some(vp.borrow_mut()),
                vault_equity,
                now + ONE_YEAR as i64,
            )
            .unwrap();
        assert_eq!(vault.user_shares, 100_000_000);

        let manager_shares = vault
            .get_manager_shares(&mut Some(vp.borrow_mut()))
            .unwrap();
        println!("manager shares: {}", manager_shares);
        assert_eq!(manager_shares, 100_000_000);

        let protocol_shares = vault.get_protocol_shares(&mut Some(vp.borrow_mut()));
        println!("protocol shares: {}", protocol_shares);
        assert_eq!(protocol_shares, 100_000);

        assert_eq!(vault.total_shares, 200_000_000 + 100_000);
        println!("total shares: {}", vault.total_shares);

        let oo =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(oo, 99950024);

        assert_eq!(vault.last_fee_update_ts, now + ONE_YEAR as i64);
    }

    #[test]
    fn test_excessive_fee_v1() {
        let now = 1000;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = 600_000;
        vp.borrow_mut().protocol_fee = 400_000;
        vault.last_fee_update_ts = 0;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 100 * QUOTE_PRECISION_U64;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vd.deposit(
            amount,
            vault_equity,
            &mut vault,
            &mut Some(vp.borrow_mut()),
            now,
        )
        .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200000000);
        assert_eq!(vault.shares_base, 0);
        assert_eq!(vault.last_fee_update_ts, 1000);
        vault_equity += amount;

        vault
            .apply_fee(
                &mut Some(vp.borrow_mut()),
                vault_equity,
                now + ONE_YEAR as i64,
            )
            .unwrap();
        assert_eq!(vault.user_shares, 10);
        assert_eq!(vault.total_shares, 2000000000);
        assert_eq!(vault.shares_base, 7);

        let vd_amount_left =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(vd_amount_left, 1);
        assert_eq!(vault.last_fee_update_ts, now + ONE_YEAR as i64);
    }

    #[test]
    fn test_fee_high_frequency_v1() {
        // asymptotic nature of calling -100% annualized on shorter time scale
        let mut now = 0;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = 600_000; // 60%
        vp.borrow_mut().protocol_fee = 400_000; // 40%
                                                // vault.management_fee = 1_000_000; // 100%
        vault.last_fee_update_ts = 0;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 100 * QUOTE_PRECISION_U64;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vd.deposit(
            amount,
            vault_equity,
            &mut vault,
            &mut Some(vp.borrow_mut()),
            now,
        )
        .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200000000);
        assert_eq!(vault.shares_base, 0);
        // assert_eq!(vault.last_fee_update_ts(, 1000);
        vault_equity += amount;

        while now < ONE_YEAR as i64 {
            vault
                .apply_fee(&mut Some(vp.borrow_mut()), vault_equity, now)
                .unwrap();
            now += 60 * 60 * 24 * 7; // every week
        }
        vault
            .apply_fee(&mut Some(vp.borrow_mut()), vault_equity, now)
            .unwrap();

        let vd_amount_left =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(vd_amount_left, 35832760); // ~$35 // 54152987
        assert_eq!(vault.last_fee_update_ts, now);
    }

    #[test]
    fn test_manager_alone_deposit_withdraw_v1() {
        let mut now = 123456789;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = 100; // .01%
        vault.last_fee_update_ts = now;
        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vault
            .manager_deposit(&mut Some(vp.borrow_mut()), amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        now += 60 * 60;

        let vault_manager_amount = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount, 100000000);

        vault
            .manager_request_withdraw(
                &mut Some(vp.borrow_mut()),
                amount,
                WithdrawUnit::Token,
                vault_equity,
                now,
            )
            .unwrap();

        let withdrew = vault
            .manager_withdraw(&mut Some(vp.borrow_mut()), vault_equity, now)
            .unwrap();
        assert_eq!(amount, withdrew);
        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 0);

        let vault_manager_amount_after = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount_after, 0);
    }

    #[test]
    fn test_negative_management_fee_v1() {
        let now = 0;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = -2_147_483_648; // -214700% annualized (manager pays 24% hourly, .4% per minute)

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 100 * QUOTE_PRECISION_U64;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vd.deposit(
            amount,
            vault_equity,
            &mut vault,
            &mut Some(vp.borrow_mut()),
            now,
        )
        .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 200000000);
        assert_eq!(vault.last_fee_update_ts, 0);
        vault_equity += amount;

        let user_eq_before =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(user_eq_before, 100000000);

        // one second since inception
        vault
            .apply_fee(&mut Some(vp.borrow_mut()), vault_equity, now + 1_i64)
            .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 199986200);

        let oo =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(oo, 100006900); // up half a cent

        // one minute since inception
        vault
            .apply_fee(&mut Some(vp.borrow_mut()), vault_equity, now + 60_i64)
            .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 199185855);

        let oo =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(oo, 100408736); // up 40 cents

        // one year since inception
        vault
            .apply_fee(
                &mut Some(vp.borrow_mut()),
                vault_equity,
                now + ONE_YEAR as i64,
            )
            .unwrap();
        assert_eq!(vault.user_shares, 100000000);
        assert_eq!(vault.total_shares, 100000000);

        let oo =
            depositor_shares_to_vault_amount(vault.user_shares, vault.total_shares, vault_equity)
                .unwrap();
        assert_eq!(oo, 200000000); // up $100

        assert_eq!(vault.last_fee_update_ts, now + ONE_YEAR as i64);
    }

    #[test]
    fn test_negative_management_fee_manager_alone_v1() {
        let mut now = 0;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = -2_147_483_648; // -214700% annualized (manager pays 24% hourly, .4% per minute)
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.last_fee_update_ts, 0);

        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        now += 100000;
        vault
            .manager_deposit(&mut Some(vp.borrow_mut()), amount, vault_equity, now)
            .unwrap();

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, amount as u128);
        assert_eq!(vault.last_fee_update_ts, now);
        vault_equity += amount;

        now += 100000;
        vault
            .manager_request_withdraw(
                &mut Some(vp.borrow_mut()),
                amount,
                WithdrawUnit::Token,
                vault_equity,
                now,
            )
            .unwrap();
        let withdrew = vault
            .manager_withdraw(&mut Some(vp.borrow_mut()), vault_equity, now)
            .unwrap();
        assert_eq!(withdrew, amount);
    }

    #[test]
    fn test_manager_deposit_withdraw_with_user_flat_v1() {
        let mut now = 123456789;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = 0;
        vault.profit_share = 150_000; // 15%

        vault.last_fee_update_ts = now;
        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vault
            .manager_deposit(&mut Some(vp.borrow_mut()), amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        now += 60 * 60;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        vd.deposit(
            amount * 20,
            vault_equity,
            &mut vault,
            &mut Some(vp.borrow_mut()),
            now,
        )
        .unwrap(); // new user deposits $2000
        now += 60 * 60;
        assert_eq!(vault.user_shares, 2000000000);
        assert_eq!(vault.total_shares, 2000000000 + 100000000);
        vault_equity += amount * 20;

        now += 60 * 60 * 24; // 1 day later

        vd.apply_profit_share(vault_equity, &mut vault, &mut Some(vp.borrow_mut()))
            .unwrap();
        vault
            .apply_fee(&mut Some(vp.borrow_mut()), vault_equity, now)
            .unwrap();

        let vault_manager_amount = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount, 100000000);
        vault
            .manager_request_withdraw(
                &mut Some(vp.borrow_mut()),
                amount,
                WithdrawUnit::Token,
                vault_equity,
                now,
            )
            .unwrap();

        let withdrew = vault
            .manager_withdraw(&mut Some(vp.borrow_mut()), vault_equity, now)
            .unwrap();
        assert_eq!(amount, withdrew);
        assert_eq!(vault.user_shares, 2000000000);
        assert_eq!(vault.total_shares, 2000000000);

        let vault_manager_amount_after = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount_after, 0);
    }

    #[test]
    fn test_manager_deposit_withdraw_with_user_manager_fee_loss_v1() {
        let mut now = 123456789;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = 100; // .01%
        vault.profit_share = 150000; // 15%

        vault.last_fee_update_ts = now;
        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vault
            .manager_deposit(&mut Some(vp.borrow_mut()), amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100000000);
        now += 60 * 60;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        vd.deposit(
            amount * 20,
            vault_equity,
            &mut vault,
            &mut Some(vp.borrow_mut()),
            now,
        )
        .unwrap(); // new user deposits $2000
        now += 60 * 60;
        assert_eq!(vault.user_shares, 2000000000);
        assert_eq!(vault.total_shares, 2000000000 + 100000000);
        vault_equity += amount * 20;

        let mut cnt = 0;
        while (vault.total_shares == 2000000000 + 100000000) && cnt < 400 {
            now += 60 * 60 * 24; // 1 day later

            vd.apply_profit_share(vault_equity, &mut vault, &mut Some(vp.borrow_mut()))
                .unwrap();
            vault
                .apply_fee(&mut Some(vp.borrow_mut()), vault_equity, now)
                .unwrap();
            // crate::msg!("vault last ts: {} vs {}", vault.last_fee_update_ts, now);
            cnt += 1;
        }

        assert_eq!(cnt, 4); // 4 days

        let vault_manager_amount = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount, 100001999);
        vault
            .manager_request_withdraw(
                &mut Some(vp.borrow_mut()),
                amount,
                WithdrawUnit::Token,
                vault_equity,
                now,
            )
            .unwrap();

        let withdrew = vault
            .manager_withdraw(&mut Some(vp.borrow_mut()), vault_equity, now)
            .unwrap();
        assert_eq!(amount, withdrew);
        assert_eq!(vault.user_shares, 2000000000);
        assert_eq!(vault.total_shares, 2000002000);
        vault_equity -= withdrew;

        let vault_manager_amount_after = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount_after, 1999); // gainz

        let vd_amount = depositor_shares_to_vault_amount(
            vd.checked_vault_shares(&vault).unwrap(),
            vault.total_shares,
            vault_equity,
        )
        .unwrap();
        assert_eq!(vd_amount, 1999998000); // loss

        assert_eq!(vd_amount + vault_manager_amount_after, vault_equity - 1);
    }

    #[test]
    fn test_manager_deposit_withdraw_with_user_gain_v1() {
        let mut now = 123456789;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vault.management_fee = 100; // .01%
        vault.profit_share = 150000; // 15%

        vault.last_fee_update_ts = now;
        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vault
            .manager_deposit(&mut Some(vp.borrow_mut()), amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100_000_000);
        now += 60 * 60;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        vd.deposit(
            amount * 20,
            vault_equity,
            &mut vault,
            &mut Some(vp.borrow_mut()),
            now,
        )
        .unwrap(); // new user deposits $2000
        now += 60 * 60;
        assert_eq!(vault.user_shares, 2_000_000_000);
        assert_eq!(vault.total_shares, 2_000_000_000 + 100_000_000);
        vault_equity += amount * 20;

        // up 50%
        vault_equity *= 3;
        vault_equity /= 2;

        assert_eq!(vault_equity, 3_150_000_000);

        let mut cnt = 0;
        while (vault.total_shares == 2_000_000_000 + 100_000_000) && cnt < 400 {
            now += 60 * 60 * 24; // 1 day later

            vd.apply_profit_share(vault_equity, &mut vault, &mut Some(vp.borrow_mut()))
                .unwrap();
            vault
                .apply_fee(&mut Some(vp.borrow_mut()), vault_equity, now)
                .unwrap();
            cnt += 1;
        }

        assert_eq!(cnt, 4); // 4 days
        assert_eq!(
            vd.cumulative_profit_share_amount,
            (1000 * QUOTE_PRECISION_U64) as i64
        );
        assert_eq!(vd.net_deposits, (2000 * QUOTE_PRECISION_U64) as i64);

        let vault_manager_amount = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount, 300002849); //$300??

        vault
            .manager_request_withdraw(
                &mut Some(vp.borrow_mut()),
                amount,
                WithdrawUnit::Token,
                vault_equity,
                now,
            )
            .unwrap();
        assert_eq!(amount, vault.last_manager_withdraw_request.value);

        let withdrew = vault
            .manager_withdraw(&mut Some(vp.borrow_mut()), vault_equity, now)
            .unwrap();
        assert_eq!(amount - 1, withdrew); // todo: slight round out of favor
        assert_eq!(vault.user_shares, 1900000000);
        assert_eq!(vault.total_shares, 2033335367);
        vault_equity -= withdrew;

        let vault_manager_amount_after = depositor_shares_to_vault_amount(
            vault.total_shares - vault.user_shares,
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(vault_manager_amount_after, 200_002_850); // gainz

        let vd_amount = depositor_shares_to_vault_amount(
            vd.checked_vault_shares(&vault).unwrap(),
            vault.total_shares,
            vault_equity,
        )
        .unwrap();
        assert_eq!(vd_amount, 2_849_997_150); // gainz

        assert_eq!(vd_amount + vault_manager_amount_after, vault_equity - 1);
    }

    #[test]
    fn test_protocol_withdraw_with_user_gain_v1() {
        let mut now = 123456789;
        let mut vault = Vault::default();
        let vp = RefCell::new(VaultProtocol::default());
        vp.borrow_mut().protocol_fee = 100; // .01% (1 bps)
        vp.borrow_mut().protocol_profit_share = 150_000; // 15%

        vault.last_fee_update_ts = now;
        let mut vault_equity: u64 = 0;
        let amount: u64 = 100 * QUOTE_PRECISION_U64;
        vault
            .manager_deposit(&mut Some(vp.borrow_mut()), amount, vault_equity, now)
            .unwrap();
        vault_equity += amount;

        assert_eq!(vault.user_shares, 0);
        assert_eq!(vault.total_shares, 100_000_000);
        now += 60 * 60;

        let vd =
            &mut VaultDepositor::new(Pubkey::default(), Pubkey::default(), Pubkey::default(), now);
        vd.deposit(
            amount * 20,
            vault_equity,
            &mut vault,
            &mut Some(vp.borrow_mut()),
            now,
        )
        .unwrap(); // new user deposits $2000
        now += 60 * 60;
        assert_eq!(vault.user_shares, 2_000_000_000);
        assert_eq!(vault.total_shares, 2_000_000_000 + 100_000_000);
        vault_equity += amount * 20;

        // up 50%
        vault_equity *= 3;
        vault_equity /= 2;

        assert_eq!(vault_equity, 3_150_000_000);

        let mut cnt = 0;
        while (vault.total_shares == 2_000_000_000 + 100_000_000) && cnt < 400 {
            now += 60 * 60 * 24; // 1 day later

            vd.apply_profit_share(vault_equity, &mut vault, &mut Some(vp.borrow_mut()))
                .unwrap();
            vault
                .apply_fee(&mut Some(vp.borrow_mut()), vault_equity, now)
                .unwrap();
            cnt += 1;
        }

        assert_eq!(cnt, 4); // 4 days
        assert_eq!(
            vd.cumulative_profit_share_amount,
            (1000 * QUOTE_PRECISION_U64) as i64
        );
        assert_eq!(vd.net_deposits, (2000 * QUOTE_PRECISION_U64) as i64);

        let protocol_amount = depositor_shares_to_vault_amount(
            vault.get_protocol_shares(&mut Some(vp.borrow_mut())),
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(protocol_amount, 150002999);

        vault
            .protocol_request_withdraw(
                &mut Some(vp.borrow_mut()),
                amount,
                WithdrawUnit::Token,
                vault_equity,
                now,
            )
            .unwrap();
        assert_eq!(amount, vp.borrow().last_protocol_withdraw_request.value);

        let withdrew = vault
            .protocol_withdraw(&mut Some(vp.borrow_mut()), vault_equity, now)
            .unwrap();
        assert_eq!(amount - 1, withdrew); // todo: slight round out of favor
        assert_eq!(vault.user_shares, 1900000000);
        assert_eq!(vault.total_shares, 2033335367);
        vault_equity -= withdrew;

        let protocol_amount_after = depositor_shares_to_vault_amount(
            vault.get_protocol_shares(&mut Some(vp.borrow_mut())),
            vault.total_shares,
            vault_equity,
        )
        .unwrap();
        let manager_amount = depositor_shares_to_vault_amount(
            vault
                .get_manager_shares(&mut Some(vp.borrow_mut()))
                .unwrap(),
            vault.total_shares,
            vault_equity,
        )
        .unwrap();

        assert_eq!(protocol_amount_after, 50_003_000);
        assert_eq!(manager_amount, 149999850);

        let vd_amount = depositor_shares_to_vault_amount(
            vd.checked_vault_shares(&vault).unwrap(),
            vault.total_shares,
            vault_equity,
        )
        .unwrap();
        assert_eq!(vd_amount, 2_849_997_150);

        assert_eq!(
            vd_amount + protocol_amount_after + manager_amount,
            vault_equity - 1
        );
    }
}
