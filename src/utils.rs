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

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct DescTablePtr {
    pub limit: u16,
    pub ptr: usize,
}
