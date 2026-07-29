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

#[macro_export]
macro_rules! read_reg {
    ($x:literal) => {
        {
            let reg;
            core::arch::asm!(
                concat!("mov {}, ", $x),
                out(reg) reg,
                options(nomem, preserves_flags, nostack)
            );
            reg
        }
    };
}

#[macro_export]
macro_rules! load_reg {
    ($x:literal, $y:expr) => {
        core::arch::asm!(
            concat!("mov ", $x, ", {}"),
            in(reg) $y,
            options(nomem, preserves_flags, nostack)
        )
    };
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct DescTablePtr {
    pub limit: u16,
    pub ptr: usize,
}
