use core::arch::asm;

pub fn get_stack_pointer() -> usize {
    let sp: usize;
    unsafe {
        asm!("mov {}, sp", out(reg) sp);
    }
    sp
}
