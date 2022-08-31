use log::info;

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) -> () {
        info!("testing {}...", core::any::type_name::<T>());
        self();
        info!("[ok]")
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    // TODO multi-thread
    // TODO should panic, unwinding after multi thread
    // https://os.phil-opp.com/freestanding-rust-binary/#the-eh-personality-language-item
    // https://www.reddit.com/r/rust/comments/phws7n/unwinding_vs_abortion_upon_panic/
    // https://github.com/dmoka/fluent-asserter
    info!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    // info!("test result: ok.");
    test_complete();
}

fn test_complete() {
    exit_qemu(QemuExitCode::Success);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}
