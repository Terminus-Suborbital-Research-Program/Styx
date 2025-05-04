// use std::{
//     sync::{Arc, Mutex},
//     thread::sleep,
//     time::Duration,
// };

// use bin_packets::{ApplicationPacket, JupiterStatus, JupiterTelemetry, UnixTimestampMillis};

// use crate::now_millis;

// use super::Task;

// pub struct TelemetryPacketViewer {
//     view: Arc<Mutex<JupiterStatus>>,
// }

// impl TelemetryPacketViewer {
//     pub(super) fn new(view: &Arc<Mutex<JupiterStatus>>) -> Self {
//         TelemetryPacketViewer {
//             view: Arc::clone(view),
//         }
//     }

//     pub fn duplicate(&self) -> Self {
//         TelemetryPacketViewer::new(&self.view)
//     }
// }

// // pub struct TelemetryLogger {
// //     cache: PacketsCacheHandler,
// //     last: Arc<Mutex<JupiterStatus>>,
// //     last_switch_time: UnixTimestampMillis,
// //     packet_sequence: u16,
// // }

// impl TelemetryLogger {
//     // pub fn new(db: &PacketsCacheHandler) -> Self {
//     //     TelemetryLogger {
//     //         cache: db.duplicate(),
//     //         last: Arc::new(Mutex::new(JupiterStatus {
//     //             time_in_phase: (UnixTimestampMillis::epoch() - UnixTimestampMillis::epoch())
//     //                 .unwrap(),
//     //             packet_number: 0,
//     //             timestamp: now_millis(),
//     //             telemetry: dummy_telemetry(),
//     //         })),
//     //         packet_sequence: 0,
//     //         last_switch_time: now_millis(),
//     //     }
//     // }

//     fn current_telemetry(&mut self) -> JupiterStatus {
//         let now = now_millis();
//         let packet = JupiterStatus {
//             time_in_phase: (now - self.last_switch_time).unwrap(),
//             packet_number: self.packet_sequence,
//             timestamp: now,
//             telemetry: dummy_telemetry(),
//         };

//         self.packet_sequence += 1;

//         packet
//     }

//     pub fn last_packet_view(&self) -> TelemetryPacketViewer {
//         TelemetryPacketViewer::new(&self.last)
//     }
// }

// impl Task for TelemetryLogger {
//     type Context = ();

//     fn task(&mut self, _context: &mut Self::Context) {
//         loop {
//             // let packet = self.current_telemetry();

//             // let mut last = self.last.lock().unwrap();
//             // *last = packet;
//             // self.cache
//             //     .insert_cached_packet(CachedPacket::from(ApplicationPacket::JupiterStatus(packet)));

//             // println!("Telemetry: {:?}", packet);

//             sleep(Duration::from_millis(1000));
//         }
//     }
// }

// fn dummy_telemetry() -> JupiterTelemetry {
//     JupiterTelemetry {
//         battery_voltage: 3.3,
//         timestamp: now_millis(),
//         packet_number: 0,
//         high_g_accel: 32.,
//         low_g_accel: 9.8,
//         temp_c: 99.0,
//         gyro: 32.8,
//         pressure_bar: 9999999.0,
//         humidity: 0.0,
//     }
// }
