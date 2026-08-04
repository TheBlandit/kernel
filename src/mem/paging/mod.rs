#[cfg(target_arch = "x86_64")]
mod level4;

mod phy;

pub mod alloc;
pub mod free;
pub mod init;
pub mod realloc;

use crate::{mem::MemPageBuffer, utils::DescTablePtr};
use core::ptr::null_mut;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum PageUse {
    Free = 0,
    Used = 1,
    Null = 2,
    Invalid = 3,
    Unknown = 4,
    Loader = 5,
}

enum PagingType {
    #[cfg(target_arch = "x86")]
    /// 32 Bit
    Level2,
    #[cfg(target_arch = "x86")]
    /// PAE
    Level3,
    #[cfg(target_arch = "x86_64")]
    Level4,
    #[cfg(target_arch = "x86_64")]
    Level5,
}

static mut TOTAL_PAGES: usize = 0;
static mut PAGING_TYPE: PagingType = PagingType::Level4;
static mut PHY_PAGE_USE: *mut PageUse = null_mut();
/// Only valid after high_jump
static mut GDTR: DescTablePtr = DescTablePtr { ptr: 0, limit: 0 };
/// 1s in range (M,12]
static mut PHY_PAGE_MASK: usize = 0;
/// 1s in range (M,0]
static mut LIN_ADDR_MASK: usize = 0;
/// 1s in the range (63,M-1]
pub static mut TO_HIGH_MASK: usize = 0;
