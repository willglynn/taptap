use super::*;

#[test]
fn enumeration_sequence() {
    let mut rx = gateway::link::Receiver::new(gateway::transport::Receiver::new(
        pv::application::Receiver::new(Observer::default()),
    ));

    // Receive the exchange from the doc, in two parts
    let (left, right) = crate::test_data::ENUMERATION_SEQUENCE.split_at(300);
    rx.extend_from_slice(left);
    {
        let observer = rx.sink().sink().sink();
        assert!(observer.enumeration_state.is_some());
        assert_eq!(
            observer
                .persistent_state
                .gateway_identities
                .iter()
                .collect::<Vec<_>>(),
            vec![]
        );
        assert_eq!(
            observer
                .persistent_state
                .gateway_versions
                .iter()
                .collect::<Vec<_>>(),
            vec![]
        );
    }

    // Finish the sequence
    rx.extend_from_slice(right);
    let observer = rx.sink().sink().sink();
    assert!(observer.enumeration_state.is_none());
    assert_eq!(
        observer
            .persistent_state
            .gateway_identities
            .iter()
            .collect::<Vec<_>>(),
        vec![
            (
                &GatewayID::try_from(0x1201).unwrap(),
                &LongAddress([0x04, 0xC0, 0x5B, 0x30, 0x00, 0x02, 0xBE, 0x16])
            ),
            (
                &GatewayID::try_from(0x1202).unwrap(),
                &LongAddress([0x04, 0xC0, 0x5B, 0x30, 0x00, 0x02, 0xBE, 0x16])
            ),
        ]
    );
    assert_eq!(
        observer
            .persistent_state
            .gateway_versions
            .iter()
            .collect::<Vec<_>>(),
        vec![(
            &GatewayID::try_from(0x1201).unwrap(),
            &String::from("Mgate Version G8.59\rJul  6 2020\r16:51:51\rGW-H158.4.3S0.12\r")
        ),]
    );
}

#[test]
fn issue_20_initial() {
    let mut rx = gateway::link::Receiver::new(gateway::transport::Receiver::new(
        pv::application::Receiver::new(Observer::default()),
    ));

    rx.extend_from_slice(crate::test_data::ISSUE_20_INITIAL);

    {
        let link_counters = rx.counters();
        assert!(link_counters.noise < 10);
        assert_eq!(link_counters.giants, 0);
        assert_eq!(link_counters.runts, 0);
        assert_eq!(link_counters.checksums, 0);
    }

    {
        let transport_counters = rx.sink().counters();
        assert_eq!(transport_counters.invalid_receive_responses, 0);
        assert!(transport_counters.receive_responses > 0);
    }

    {
        let pv_counters = rx.sink().sink().counters();
        assert_eq!(pv_counters.invalid_power_reports, 0);
        assert!(pv_counters.power_reports > 0);
    }
}
