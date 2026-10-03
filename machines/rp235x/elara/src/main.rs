#![cfg_attr(all(target_arch = "arm", target_os = "none"), no_std)]
#![cfg_attr(all(target_arch = "arm", target_os = "none"), no_main)]

#[cfg(all(target_arch = "arm", target_os = "none"))]
mod board_io;
#[cfg(all(target_arch = "arm", target_os = "none"))]
mod device_constants;
#[cfg(all(target_arch = "arm", target_os = "none"))]
mod gnc;

#[cfg(all(target_arch = "arm", target_os = "none"))]
use cortex_m_rt::{entry, exception};
#[cfg(all(target_arch = "arm", target_os = "none"))]
use panic_halt as _;

#[cfg(all(target_arch = "arm", target_os = "none"))]
use board_io::Board;
#[cfg(all(target_arch = "arm", target_os = "none"))]
use gnc::GncController;
#[cfg(all(target_arch = "arm", target_os = "none"))]
use pulsar::arch::Arch;
#[cfg(all(target_arch = "arm", target_os = "none"))]
use pulsar::arch::arm;
#[cfg(all(target_arch = "arm", target_os = "none"))]
use pulsar::kernel::hooks::KernelHooks;
#[cfg(all(target_arch = "arm", target_os = "none"))]
use pulsar::kernel::{AppStorage, Context, TaskId, bootstrap_app};
#[cfg(all(target_arch = "arm", target_os = "none"))]
use pulsar::messages::SpscMailbox;
#[cfg(all(target_arch = "arm", target_os = "none"))]
use pulsar_macros::app;
#[cfg(all(target_arch = "arm", target_os = "none"))]
use rp235x_hal as hal;
#[cfg(all(target_arch = "arm", target_os = "none"))]
use rtt_target::ChannelMode::NoBlockSkip;
#[cfg(all(target_arch = "arm", target_os = "none"))]
use rtt_target::{rtt_init, set_print_channel, UpChannel};

#[cfg(all(target_arch = "arm", target_os = "none"))]
use crate::device_constants::{COMPUTE_POLL_HZ, GNC_HZ, IMU_SAMPLE_HZ, TELEMETRY_HZ, TICK_HZ};
#[cfg(all(target_arch = "arm", target_os = "none"))]
use elara::{ComputeMessage, EjectedSatellitePhase, IMU_VALID_FLAG, TelemetryFrame};

#[cfg(all(target_arch = "arm", target_os = "none"))]
static APP_RESOURCES: AppStorage<satellite_app::SharedResources, satellite_app::LocalResources> =
    AppStorage::uninit();
#[cfg(all(target_arch = "arm", target_os = "none"))]
#[cfg(all(target_arch = "arm", target_os = "none"))]
static COMPUTE_MAILBOX: SpscMailbox<ComputeMessage, 8> = SpscMailbox::new();

#[cfg(all(target_arch = "arm", target_os = "none"))]
fn enable_fpu() {
    const SCB_CPACR: *mut u32 = 0xE000_ED88 as *mut u32;
    const CP10_CP11_FULL_ACCESS: u32 = 0b1111 << 20;

    unsafe {
        let cpacr = core::ptr::read_volatile(SCB_CPACR);
        core::ptr::write_volatile(SCB_CPACR, cpacr | CP10_CP11_FULL_ACCESS);
    }
    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
fn init_logs() -> UpChannel {
    let channels = rtt_init! {
        up: {
            0: { size: 1024, mode: NoBlockSkip, name: "print" }
            1: { size: 4096, mode: NoBlockSkip, name: "telemetry" }
        }
    };

    let print_channel = channels.up.0;
    set_print_channel(print_channel);
    channels.up.1
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[cfg(all(target_arch = "arm", target_os = "none"))]
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"Satellite bring-up firmware with Pulsar"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];

#[cfg(all(target_arch = "arm", target_os = "none"))]
#[app(cores = 1usize)]
mod satellite_app {
    use super::{
        Board, COMPUTE_POLL_HZ, Context, GNC_HZ, GncController,
        IMU_SAMPLE_HZ, TELEMETRY_HZ, TaskId, TelemetryFrame, UpChannel,
    };
    use super::EjectedSatellitePhase;

    #[init]
    fn init() {}

    #[shared]
    pub struct SharedState {
        pub active_phase: EjectedSatellitePhase,
        pub latest_telemetry: TelemetryFrame,
    }

    impl Default for SharedState {
        fn default() -> Self {
            Self {
                active_phase: EjectedSatellitePhase::Despin,
                latest_telemetry: TelemetryFrame::default(),
            }
        }
    }

    #[local]
    pub struct LocalState {
        pub board: Option<Board>,
        pub gnc: Option<GncController>,
        pub telemetry_channel: Option<UpChannel>,
    }

    impl Default for LocalState {
        fn default() -> Self {
            Self {
                board: None,
                gnc: None,
                telemetry_channel: None,
            }
        }
    }

    pub fn init_task(
        _app_init: &InitResources,
        _mapped_tasks: &[::pulsar::kernel::TaskControlBlock; TASK_COUNT],
        _shared: &mut SharedResources,
        _local: &mut LocalResources,
    ) {
        rtt_target::rprintln!("boot: pulsar init task complete");
    }

    #[task(shared = [latest_telemetry], priority = 5, core = 0, state = Ready, rate_hz = 1)]
    fn heartbeat_task(_id: TaskId, _cx: &mut Context) {
        let shared = unsafe { super::shared_mut() };
        rtt_target::rprintln!(
            "heartbeat: seq={} flags=0x{:02x}",
            shared.latest_telemetry.snapshot.seq,
            shared.latest_telemetry.snapshot.flags,
        );
    }

    #[task(local = [board], shared = [active_phase, latest_telemetry], priority = 20, core = 0, state = Ready, rate_hz = IMU_SAMPLE_HZ)]
    fn sensor_task(_id: TaskId, _cx: &mut Context) {
        let local = unsafe { super::local_mut() };
        let shared = unsafe { super::shared_mut() };
        let board = local.board.as_mut().expect("board not initialized");
        rtt_target::rprintln!("task: sensor start");
            let frame = TelemetryFrame::new(
                board.next_snapshot(elara::GpsFix::default()),
                shared.active_phase,
        );
        shared.latest_telemetry = frame;
        rtt_target::rprintln!(
            "task: sensor done seq={} flags=0x{:02x}",
            shared.latest_telemetry.snapshot.seq,
            shared.latest_telemetry.snapshot.flags,
        );

    }

    #[task(local = [board], priority = 15, core = 0, state = Ready, rate_hz = COMPUTE_POLL_HZ)]
    fn compute_uart_task(_id: TaskId, _cx: &mut Context) {
        let local = unsafe { super::local_mut() };
        let board = local.board.as_mut().expect("board not initialized");
        board.drain_compute_uart(|message| super::COMPUTE_MAILBOX.try_send(message).is_ok());
    }

    #[task(local = [board, telemetry_channel], shared = [latest_telemetry], priority = 12, core = 0, state = Ready, rate_hz = COMPUTE_POLL_HZ)]
    fn compute_message_task(_id: TaskId, _cx: &mut Context) {
        let local = unsafe { super::local_mut() };
        let shared = unsafe { super::shared_mut() };
        let board = local.board.as_mut().expect("board not initialized");
        let telemetry_channel = local
            .telemetry_channel
            .as_mut()
            .expect("telemetry channel not initialized");

        while let Some(message) = super::COMPUTE_MAILBOX.try_recv() {
            board.handle_compute_message(message, &shared.latest_telemetry, telemetry_channel);
        }
    }

    #[task(local = [board, gnc], shared = [latest_telemetry], priority = 11, core = 0, state = Ready, rate_hz = GNC_HZ)]
    fn gnc_task(_id: TaskId, _cx: &mut Context) {
        let local = unsafe { super::local_mut() };
        let shared = unsafe { super::shared_mut() };
        let board = local.board.as_mut().expect("board not initialized");
        let gnc = local.gnc.as_mut().expect("gnc not initialized");

        if (shared.latest_telemetry.snapshot.flags & super::IMU_VALID_FLAG) == 0 {
            let applied_motors = board.apply_gnc_motor_commands([0; 3]);
            shared.latest_telemetry.snapshot.motor_x = applied_motors[0];
            shared.latest_telemetry.snapshot.motor_y = applied_motors[1];
            shared.latest_telemetry.snapshot.motor_z = applied_motors[2];
            return;
        }

        let output = gnc.update(&shared.latest_telemetry);
        let applied_motors = board.apply_gnc_motor_commands(output.motor_trim_commands);
        shared.latest_telemetry.gnc = output.telemetry;
        shared.latest_telemetry.snapshot.motor_x = applied_motors[0];
        shared.latest_telemetry.snapshot.motor_y = applied_motors[1];
        shared.latest_telemetry.snapshot.motor_z = applied_motors[2];
    }

    #[task(local = [board, telemetry_channel], shared = [latest_telemetry], priority = 10, core = 0, state = Ready, rate_hz = TELEMETRY_HZ)]
    fn telemetry_task(_id: TaskId, _cx: &mut Context) {
        let local = unsafe { super::local_mut() };
        let shared = unsafe { super::shared_mut() };
        let board = local.board.as_mut().expect("board not initialized");
        let telemetry_channel = local
            .telemetry_channel
            .as_mut()
            .expect("telemetry channel not initialized");

        board.write_telemetry(&shared.latest_telemetry, telemetry_channel);
    }

    #[idle(core = 0)]
    fn idle() {
        cortex_m::asm::wfi();
    }
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
#[entry]
fn main() -> ! {
    enable_fpu();
    let telemetry_channel = init_logs();
    rtt_target::rprintln!("boot: logs initialized");

    let internal = satellite_app::internal_main();
    bootstrap_app(internal, &APP_RESOURCES, satellite_app::init_task);
    rtt_target::rprintln!("boot: scheduler bootstrapped");

    let pac = hal::pac::Peripherals::take().unwrap();
    rtt_target::rprintln!("boot: starting board bring-up");
    let board = match Board::new(pac) {
        Ok(board) => board,
        Err(err) => {
            rtt_target::rprintln!("boot: board bring-up failed: {}", err);
            loop {
                cortex_m::asm::bkpt();
            }
        }
    };
    rtt_target::rprintln!("boot: board bring-up complete");
    let system_clock_hz = board.system_clock_hz();

    unsafe {
        let local = local_mut();
        local.board = Some(board);
        local.gnc = Some(GncController::new(1.0f32 / GNC_HZ as f32));
        local.telemetry_channel = Some(telemetry_channel);
    }
    rtt_target::rprintln!("boot: local state initialized");

    let mapped = satellite_app::mapped_tcbs();
    let now_ns = <pulsar::arch::arm::ArmArch as Arch>::now_ns() as u32;
    let sched = unsafe { satellite_app::install_scheduler_storage(mapped, now_ns) };
    rtt_target::rprintln!("boot: scheduler installed");

    arm::rt::set_tick_ns(1_000_000_000u32 / TICK_HZ);
    arm::systick::configure(system_clock_hz, TICK_HZ);
    rtt_target::rprintln!("boot: systick configured, entering run loop");

    sched.run_forever::<BoardHooks>(0)
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
#[exception]
fn SysTick() {
    arm::systick::tick(1_000_000_000u32 / TICK_HZ);
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
#[exception]
fn PendSV() {
    unsafe { arm::rt::on_pendsv() }
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
struct BoardHooks;

#[cfg(all(target_arch = "arm", target_os = "none"))]
impl KernelHooks for BoardHooks {
    fn idle(_core: u8) {
        cortex_m::asm::wfi();
    }

    fn panic(msg: &'static str) -> ! {
        rtt_target::rprintln!("kernel panic: {}", msg);
        loop {
            cortex_m::asm::bkpt();
        }
    }
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
unsafe fn shared_mut() -> &'static mut satellite_app::SharedResources {
    unsafe { APP_RESOURCES.shared_mut() }
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
unsafe fn local_mut() -> &'static mut satellite_app::LocalResources {
    unsafe { APP_RESOURCES.local_mut() }
}

#[cfg(not(all(target_arch = "arm", target_os = "none")))]
fn main() {
    println!("satellite firmware is intended for thumbv8m.main-none-eabihf");
}