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
