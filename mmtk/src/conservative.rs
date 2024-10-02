use crate::julia_types::*;
use crate::UPCALLS;

use mmtk::memory_manager;
use mmtk::util::constants::BYTES_IN_ADDRESS;
use mmtk::util::{Address, ObjectReference};

use std::collections::HashSet;
use std::sync::Mutex;

lazy_static! {
    pub static ref CONSERVATIVE_ROOTS: Mutex<HashSet<ObjectReference>> = Mutex::new(HashSet::new());
}

pub fn pin_conservative_roots() {
    crate::early_return_for_non_moving!(());

    let mut roots = CONSERVATIVE_ROOTS.lock().unwrap();
    let n_roots = roots.len();
    roots.retain(|obj| mmtk::memory_manager::pin_object(*obj));
    let n_pinned = roots.len();
    log::debug!("Conservative roots: {}, pinned: {}", n_roots, n_pinned);
}

pub fn unpin_conservative_roots() {
    crate::early_return_for_non_moving!(());

    let mut roots = CONSERVATIVE_ROOTS.lock().unwrap();
    let n_pinned = roots.len();
    let mut n_live = 0;
    roots.drain().for_each(|obj| {
        if mmtk::memory_manager::is_live_object(obj) {
            n_live += 1;
            mmtk::memory_manager::unpin_object(obj);
        }
    });
    log::debug!(
        "Conservative roots: pinned: {}, unpinned/live {}",
        n_pinned,
        n_live
    );
}

// See jl_guard_size
// TODO: Are we sure there are always guard pages we need to skip?
const JL_GUARD_PAGE: usize = 4096 * 8;

pub fn mmtk_conservative_scan_task_stack(
    ta: *const mmtk_jl_task_t,
    buffer: &mut Vec<ObjectReference>,
) {
    crate::early_return_for_non_moving!(());

    let mut size: u64 = 0;
    let mut ptid: i32 = 0;
    log::debug!("mmtk_conservative_scan_native_stack begin ta = {:?}", ta);
    let stk = unsafe {
        ((*UPCALLS).mmtk_jl_task_stack_buffer)(ta, &mut size as *mut _, &mut ptid as *mut _)
    };
    log::debug!(
        "mmtk_conservative_scan_native_stack continue stk = {}, size = {}, ptid = {:x}",
        stk,
        size,
        ptid
    );
    if !stk.is_zero() {
        log::debug!("Conservatively scan the stack");

        let guard_page_start = stk + JL_GUARD_PAGE;
        log::debug!("Skip guard page: {}, {}", stk, guard_page_start);

        conservative_scan_range(guard_page_start, stk + size as usize, buffer);
    } else {
        log::debug!("Skip stack for {:?}", ta);
    }
}

pub fn mmtk_conservative_scan_ptls_stack(
    ptls: &mut mmtk_jl_tls_states_t,
    buffer: &mut Vec<ObjectReference>,
) {
    crate::early_return_for_non_moving!(());

    let stackbase: Address = Address::from_ptr(ptls.stackbase);
    let stacksize = ptls.stacksize;

    let stackstart = stackbase - stacksize;
    let guard_page_start = stackstart + JL_GUARD_PAGE;
    log::debug!(
        "mmtk_conservative_scan_ptls_stack: {} to {}",
        stackstart,
        stackbase
    );
    conservative_scan_range(guard_page_start, stackbase, buffer);
}

pub fn mmtk_conservative_scan_task_registers(
    ta: *const mmtk_jl_task_t,
    buffer: &mut Vec<ObjectReference>,
) {
    crate::early_return_for_non_moving!(());

    let (lo, hi) = get_range(&unsafe { &*ta }.ctx);
    conservative_scan_range(lo, hi, buffer);
}

pub fn mmtk_conservative_scan_ptls_registers(
    ptls: &mut mmtk_jl_tls_states_t,
    buffer: &mut Vec<ObjectReference>,
) {
    crate::early_return_for_non_moving!(());

    let (lo, hi) = get_range(&ptls.ctx_at_the_time_gc_started);
    conservative_scan_range(lo, hi, buffer);
}

// TODO: This scans the entire context type, which is slower.
// We actually only need to scan registers.
fn get_range<T>(ctx: &T) -> (Address, Address) {
    let start = Address::from_ptr(ctx);
    let ty_size = std::mem::size_of::<T>();
    (start, start + ty_size)
}

fn conservative_scan_range(lo: Address, hi: Address, buffer: &mut Vec<ObjectReference>) {
    // The high address is exclusive
    let hi = if hi.is_aligned_to(BYTES_IN_ADDRESS) {
        hi - BYTES_IN_ADDRESS
    } else {
        hi.align_down(BYTES_IN_ADDRESS)
    };
    let lo = lo.align_up(BYTES_IN_ADDRESS);
    log::trace!("Scan {} (lo) {} (hi)", lo, hi);

    let mut cursor = hi;
    while cursor >= lo {
        let addr = unsafe { cursor.load::<Address>() };
        if let Some(obj) = is_potential_mmtk_object(addr) {
            buffer.push(obj);
        }
        cursor -= BYTES_IN_ADDRESS;
    }
}

fn is_potential_mmtk_object(addr: Address) -> Option<ObjectReference> {
    if crate::object_model::is_addr_in_immixspace(addr) {
        // We only care about immix space. If the object is in other spaces, we won't move them, and we don't need to pin them.
        memory_manager::find_object_from_internal_pointer(addr, usize::MAX)
    } else {
        None
    }
}
