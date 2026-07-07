#[macro_export]
macro_rules! status_panic {
    ($x:expr) => {
        let status: r_efi::efi::Status = $x;
        if status.is_error() {
            panic!();
        }
    };

    ($x:expr, $y:literal) => {
        let status: r_efi::efi::Status = $x;
        if status.is_error() {
            panic!($y);
        }
    };
}

pub struct MemByteBuffer {
    pub start: usize,
    pub size: usize,
}
