//! The guest within the hypervisor.

use alloc::sync::Arc;
use spin::Mutex;

use super::structs::VMM_GLOBAL_STATE;
use crate::memory::{GuestPhysAddr, GuestPhysMemorySetTrait, HostPhysAddr};
use crate::trap_map::{TrapKind, TrapMap};
use crate::PAGE_SIZE;
use crate::{RvmError, RvmResult};

/// Represents a guest within the hypervisor.
#[derive(Debug)]
pub struct Guest {
    pub(super) gpm: Arc<dyn GuestPhysMemorySetTrait>,
    pub(super) traps: Mutex<TrapMap>,
}

impl Guest {
    /// Create a new Guest.
    pub fn new(gpm: Arc<dyn GuestPhysMemorySetTrait>) -> RvmResult<Arc<Self>> {
        VMM_GLOBAL_STATE.lock().alloc()?;
        Ok(Arc::new(Self {
            gpm,
            traps: Mutex::new(TrapMap::default()),
        }))
    }

    /// Get the page table base address.
    pub fn rvm_page_table_phys(&self) -> usize {
        self.gpm.table_phys()
    }

    pub fn add_memory_region(
        &self,
        gpaddr: GuestPhysAddr,
        size: usize,
        hpaddr: Option<HostPhysAddr>,
    ) -> RvmResult {
        if gpaddr & (PAGE_SIZE - 1) != 0 || size & (PAGE_SIZE - 1) != 0 {
            return Err(RvmError::InvalidParam);
        }
        if let Some(hpaddr) = hpaddr {
            if hpaddr & (PAGE_SIZE - 1) != 0 {
                return Err(RvmError::InvalidParam);
            }
        }
        self.gpm.add_map(gpaddr, size, hpaddr)
    }

    pub fn set_trap(&self, kind: TrapKind, addr: usize, size: usize, key: u64) -> RvmResult {
        if size == 0 {
            return Err(RvmError::InvalidParam);
        }
        if addr > usize::MAX - size {
            return Err(RvmError::OutOfRange);
        }
        match kind {
            TrapKind::GuestTrapBell => Err(RvmError::NotSupported),
            TrapKind::GuestTrapIo => {
                if addr + size > u16::MAX as usize {
                    Err(RvmError::OutOfRange)
                } else {
                    self.traps.lock().push(kind, addr, size, key)
                }
            }
            TrapKind::GuestTrapMem => {
                if addr & (PAGE_SIZE - 1) != 0 || size & (PAGE_SIZE - 1) != 0 {
                    Err(RvmError::InvalidParam)
                } else {
                    self.gpm.remove_map(addr, size)?;
                    self.traps.lock().push(kind, addr, size, key)
                }
            }
            _ => Err(RvmError::InvalidParam),
        }
    }
}

impl Drop for Guest {
    fn drop(&mut self) {
        debug!("Guest free {:#x?}", self);
        VMM_GLOBAL_STATE.lock().free();
    }
}
