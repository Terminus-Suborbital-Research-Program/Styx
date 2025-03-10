#![no_std]
#![no_main]

// fn setup_defmt() {
//     rtt_target::rtt_init_defmt!()
// }

// #[cfg(test)]
// #[embedded_test::tests(setup=crate::setup_defmt())]
// mod tests {
//     use rp235x_hal::pac::Peripherals;

//     #[init]
//     fn init() -> Peripherals {
//         Peripherals::take().unwrap()
//     }

//     #[test]
//     fn trivial_passes() {
//         assert!(true);
//     }

//     #[test]
//     #[should_panic]
//     fn trivial_fails() {
//         assert!(false);
//     }
// }
