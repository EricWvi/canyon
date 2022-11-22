use log::info;

pub trait Testable {
    fn run(&self) -> ();
    fn name(&self) -> &'static str;
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

    fn name(&self) -> &'static str {
        core::any::type_name::<T>()
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    let fn_name = include_str!("../target/test-func").trim();
    // TODO multi-thread
    // TODO should panic, unwinding after multi thread
    // https://os.phil-opp.com/freestanding-rust-binary/#the-eh-personality-language-item
    // https://www.reddit.com/r/rust/comments/phws7n/unwinding_vs_abortion_upon_panic/
    // https://github.com/dmoka/fluent-asserter
    let mut origin = tests.iter();
    let mut filter = tests.iter().filter(|t| t.name().contains(fn_name));

    let tests: &mut dyn Iterator<Item = &&dyn Testable> = if fn_name.is_empty() {
        info!("Running {} tests", origin.clone().count());
        &mut origin
    } else {
        info!("Running {} tests", filter.clone().count());
        &mut filter
    };

    for test in tests {
        test.run();
    }
    info!("test result: ok.");
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
