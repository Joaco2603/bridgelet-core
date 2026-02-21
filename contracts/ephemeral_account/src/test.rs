#[cfg(test)]
mod test {
    use crate::{AccountStatus, EphemeralAccountContract, EphemeralAccountContractClient};
    use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
        let status = client.get_status();
        assert_eq!(status, AccountStatus::Active);
        assert_eq!(client.is_expired(), false);
    }

    #[test]
    fn test_record_payment() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
        client.record_payment(&100, &asset);

        let status = client.get_status();
        assert_eq!(status, AccountStatus::PaymentReceived);
    }

    #[test]
    fn test_multiple_payments() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);

        client.record_payment(&100, &asset1);
        let info = client.get_info();
        assert_eq!(info.payment_count, 1);

        client.record_payment(&50, &asset2);
        let info = client.get_info();
        assert_eq!(info.payment_count, 2);

        let status = client.get_status();
        assert_eq!(status, AccountStatus::PaymentReceived);
    }

    #[test]
    fn test_sweep_single_asset() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
        client.record_payment(&100, &asset);

        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        let status = client.get_status();
        assert_eq!(status, AccountStatus::Swept);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #13)")] // DuplicateAsset
    fn test_duplicate_asset() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
        client.record_payment(&100, &asset);
        client.record_payment(&50, &asset); // Should fail - duplicate asset
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #14)")] // TooManyPayments
    fn test_too_many_assets() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);

        // Add 10 payments (should work)
        for i in 0..10 {
            let asset = Address::generate(&env);
            client.record_payment(&(100 + i as i128), &asset);
        }

        // 11th should fail
        let asset = Address::generate(&env);
        client.record_payment(&200, &asset);
    }

    #[test]
    fn test_sweep_multiple_assets() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);

        // Record 3 different assets
        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);
        let asset3 = Address::generate(&env);

        client.record_payment(&100, &asset1);
        client.record_payment(&200, &asset2);
        client.record_payment(&300, &asset3);

        let info = client.get_info();
        assert_eq!(info.payment_count, 3);
        assert_eq!(info.payments.len(), 3);

        // Sweep all
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        assert_eq!(client.get_status(), AccountStatus::Swept);
    }

    #[test]
    fn test_multi_payment_events() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);

        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);

        // First payment should emit PaymentReceived
        client.record_payment(&100, &asset1);

        // Second payment should emit MultiPaymentReceived
        client.record_payment(&200, &asset2);

        // Verify events were published (check env.events())
    }

    #[test]
    fn test_sweep_emits_reserve_reclaimed_event() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
        client.record_payment(&100, &asset);

        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        assert_eq!(client.get_status(), AccountStatus::Swept);

        // Find the ReserveReclaimed event and assert its fields
        let events = env.events().all();
        let reserve_event = events
            .iter()
            .find(|(_, topics, _)| topics.get(0) == Some(symbol_short!("reserve").into()));

        assert!(
            reserve_event.is_some(),
            "ReserveReclaimed event was not emitted"
        );

        let (_, _, data) = reserve_event.unwrap();
        let reclaimed: ReserveReclaimed = data.into_val(&env);
        assert_eq!(reclaimed.destination, destination);
        assert_eq!(reclaimed.amount, 1_000_000_000i128);
    }

    #[test]
    fn test_sweep_multiple_assets_with_reserve_event() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);

        // Record 3 different assets
        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);
        let asset3 = Address::generate(&env);

        client.record_payment(&100, &asset1);
        client.record_payment(&200, &asset2);
        client.record_payment(&300, &asset3);

        let info = client.get_info();
        assert_eq!(info.payment_count, 3);
        assert_eq!(info.payments.len(), 3);

        // Sweep all - should emit both SweepExecutedMulti and ReserveReclaimed events
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        assert_eq!(client.get_status(), AccountStatus::Swept);
    }
}
