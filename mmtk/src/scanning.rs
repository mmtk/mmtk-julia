use crate::edges::JuliaVMEdge;
use crate::julia_scanning::process_offset_edge;
use crate::julia_scanning::{get_stack_addr, process_edge, read_stack};
use crate::julia_types::{mmtk_jl_gcframe_t, mmtk_jl_task_t, UINT32_MAX};
use crate::{SINGLETON, UPCALLS};
use mmtk::memory_manager;
use mmtk::scheduler::*;
use mmtk::util::ObjectReference;
use mmtk::util::{opaque_pointer::*, Address};
use mmtk::vm::edge_shape::Edge;
use mmtk::vm::EdgeVisitor;
use mmtk::vm::ObjectTracerContext;
use mmtk::vm::RootsWorkFactory;
use mmtk::vm::Scanning;
use mmtk::vm::VMBinding;
use mmtk::Mutator;
use mmtk::MMTK;

use crate::JuliaVM;

pub struct VMScanning {}

impl Scanning<JuliaVM> for VMScanning {
    fn scan_roots_in_mutator_thread(
        _tls: VMWorkerThread,
        mutator: &'static mut Mutator<JuliaVM>,
        mut factory: impl RootsWorkFactory<JuliaVMEdge>,
    ) {
        // This allows us to reuse mmtk_scan_gcstack which expectes an EdgeVisitor
        // Push the nodes as they need to be transitively pinned
        struct EdgeBuffer {
            pub buffer: Vec<ObjectReference>,
        }
        impl mmtk::vm::EdgeVisitor<JuliaVMEdge> for EdgeBuffer {
            fn visit_edge(&mut self, edge: JuliaVMEdge) {
                match edge {
                    JuliaVMEdge::Simple(se) => {
                        let slot = se.as_address();
                        let object = unsafe { slot.load::<ObjectReference>() };
                        if !object.is_null() {
                            self.buffer.push(object);
                        }
                    }
                    JuliaVMEdge::Offset(oe) => {
                        let object = oe.load();
                        if !object.is_null() {
                            self.buffer.push(object);
                        }
                    }
                }
            }
        }

        use crate::julia_scanning::*;
        use crate::julia_types::*;

        let ptls: &mut mmtk__jl_tls_states_t = unsafe { std::mem::transmute(mutator.mutator_tls) };
        let mut tpinning_edge_buffer = EdgeBuffer { buffer: vec![] }; // need to be transitively pinned
        let mut pinning_edge_buffer = EdgeBuffer { buffer: vec![] }; // roots from the shadow stack that we know that do not need to be transitively pinned
        let mut node_buffer = vec![];

        // Scan thread local from ptls: See gc_queue_thread_local in gc.c
        let mut root_scan_task = |task: *const mmtk__jl_task_t, task_is_root: bool| {
            if !task.is_null() {
                unsafe {
                    mmtk_scan_gcstack_roots(
                        task,
                        &mut tpinning_edge_buffer,
                        &mut pinning_edge_buffer,
                    );
                }
                if task_is_root {
                    // captures wrong root nodes before creating the work
                    debug_assert!(
                        Address::from_ptr(task).as_usize() % 16 == 0
                            || Address::from_ptr(task).as_usize() % 8 == 0,
                        "root node {:?} is not aligned to 8 or 16",
                        Address::from_ptr(task)
                    );

                    node_buffer.push(ObjectReference::from_raw_address(Address::from_ptr(task)));
                }
            }
        };
        root_scan_task(ptls.root_task, true);

        // need to iterate over live tasks as well to process their shadow stacks
        // we should not set the task themselves as roots as we will know which ones are still alive after GC
        let mut i = 0;
        while i < ptls.heap.live_tasks.len {
            let mut task_address = Address::from_ptr(ptls.heap.live_tasks.items);
            task_address = task_address.shift::<Address>(i as isize);
            let task = unsafe { task_address.load::<*const mmtk_jl_task_t>() };
            root_scan_task(task, false);
            i += 1;
        }

        root_scan_task(ptls.current_task as *mut mmtk__jl_task_t, true);
        root_scan_task(ptls.next_task, true);
        root_scan_task(ptls.previous_task, true);
        if !ptls.previous_exception.is_null() {
            node_buffer.push(ObjectReference::from_raw_address(Address::from_mut_ptr(
                ptls.previous_exception,
            )));
        }

        // Scan backtrace buffer: See gc_queue_bt_buf in gc.c
        let mut i = 0;
        while i < ptls.bt_size {
            let bt_entry = unsafe { ptls.bt_data.add(i) };
            let bt_entry_size = mmtk_jl_bt_entry_size(bt_entry);
            if mmtk_jl_bt_is_native(bt_entry) {
                i += bt_entry_size;
                continue;
            }
            let njlvals = mmtk_jl_bt_num_jlvals(bt_entry);
            for j in 0..njlvals {
                let bt_entry_value = mmtk_jl_bt_entry_jlvalue(bt_entry, j);

                // captures wrong root nodes before creating the work
                debug_assert!(
                    bt_entry_value.to_raw_address().as_usize() % 16 == 0
                        || bt_entry_value.to_raw_address().as_usize() % 8 == 0,
                    "root node {:?} is not aligned to 8 or 16",
                    bt_entry_value
                );

                node_buffer.push(bt_entry_value);
            }
            i += bt_entry_size;
        }

        // We do not need gc_queue_remset from gc.c (we are not using remset in the thread)

        // Push work
        const CAPACITY_PER_PACKET: usize = 4096;
        for tpinning_roots in tpinning_edge_buffer
            .buffer
            .chunks(CAPACITY_PER_PACKET)
            .map(|c| c.to_vec())
        {
            factory.create_process_tpinning_roots_work(tpinning_roots);
        }
        for pinning_roots in pinning_edge_buffer
            .buffer
            .chunks(CAPACITY_PER_PACKET)
            .map(|c| c.to_vec())
        {
            factory.create_process_pinning_roots_work(pinning_roots);
        }
        for nodes in node_buffer.chunks(CAPACITY_PER_PACKET).map(|c| c.to_vec()) {
            factory.create_process_pinning_roots_work(nodes);
        }
    }

    fn scan_vm_specific_roots(
        _tls: VMWorkerThread,
        mut factory: impl RootsWorkFactory<JuliaVMEdge>,
    ) {
        use crate::edges::RootsWorkClosure;
        let mut roots_closure = RootsWorkClosure::from_roots_work_factory(&mut factory);
        unsafe {
            ((*UPCALLS).scan_vm_specific_roots)(&mut roots_closure as _);
        }
    }

    fn scan_object<EV: EdgeVisitor<JuliaVMEdge>>(
        _tls: VMWorkerThread,
        object: ObjectReference,
        edge_visitor: &mut EV,
    ) {
        process_object(object, edge_visitor);
    }
    fn notify_initial_thread_scan_complete(_partial_scan: bool, _tls: VMWorkerThread) {
        let sweep_vm_specific_work = SweepVMSpecific::new();
        memory_manager::add_work_packet(
            &SINGLETON,
            WorkBucketStage::Compact,
            sweep_vm_specific_work,
        );
    }
    fn supports_return_barrier() -> bool {
        unimplemented!()
    }

    fn prepare_for_roots_re_scanning() {
        unimplemented!()
    }

    fn process_weak_refs(
        _worker: &mut GCWorker<JuliaVM>,
        tracer_context: impl ObjectTracerContext<JuliaVM>,
    ) -> bool {
        let single_thread_process_finalizer = ScanFinalizersSingleThreaded { tracer_context };
        memory_manager::add_work_packet(
            &SINGLETON,
            WorkBucketStage::VMRefClosure,
            single_thread_process_finalizer,
        );

        // We have pushed work. No need to repeat this method.
        false
    }
}

pub unsafe fn mmtk_scan_gcstack_roots<EV: EdgeVisitor<JuliaVMEdge>>(
    ta: *const mmtk_jl_task_t,
    tpinned_closure: &mut EV,
    pinned_closure: &mut EV,
) {
    let stkbuf = (*ta).stkbuf;
    let copy_stack = (*ta).copy_stack_custom();

    let mut s = (*ta).gcstack;
    let (mut offset, mut lb, mut ub) = (0 as isize, 0 as u64, u64::MAX);

    #[cfg(feature = "julia_copy_stack")]
    if stkbuf != std::ptr::null_mut() && copy_stack != 0 && (*ta).ptls == std::ptr::null_mut() {
        if ((*ta).tid as i16) < 0 {
            panic!("tid must be positive.")
        }
        let stackbase = ((*UPCALLS).get_stackbase)((*ta).tid);
        ub = stackbase as u64;
        lb = ub - ((*ta).copy_stack() as u64);
        offset = (*ta).stkbuf as isize - lb as isize;
    }

    if s != std::ptr::null_mut() {
        let s_nroots_addr = ::std::ptr::addr_of!((*s).nroots);
        let mut nroots = read_stack(Address::from_ptr(s_nroots_addr), offset, lb, ub);
        debug_assert!(nroots.as_usize() as u32 <= UINT32_MAX);
        let mut nr = nroots >> 3;

        loop {
            let rts = Address::from_mut_ptr(s).shift::<Address>(2);
            let mut i = 0;
            while i < nr {
                // root should be only pinned and not transitively pinned
                if (nroots.as_usize() & 4) != 0 {
                    if (nroots.as_usize() & 1) != 0 {
                        let slot = read_stack(rts.shift::<Address>(i as isize), offset, lb, ub);
                        let real_addr = get_stack_addr(slot, offset, lb, ub);
                        process_edge(pinned_closure, real_addr);
                    } else {
                        let real_addr =
                            get_stack_addr(rts.shift::<Address>(i as isize), offset, lb, ub);

                        let slot = read_stack(rts.shift::<Address>(i as isize), offset, lb, ub);
                        use crate::julia_finalizer::gc_ptr_tag;
                        // malloced pointer tagged in jl_gc_add_quiescent
                        // skip both the next element (native function), and the object
                        if slot & 3usize == 3 {
                            i += 2;
                            continue;
                        }

                        // pointer is not malloced but function is native, so skip it
                        if gc_ptr_tag(slot, 1) {
                            process_offset_edge(pinned_closure, real_addr, 1);
                            i += 2;
                            continue;
                        }

                        process_edge(pinned_closure, real_addr);
                    }

                    i += 1;
                } else {
                    // root should be transitively pinned
                    if (nroots.as_usize() & 1) != 0 {
                        let slot = read_stack(rts.shift::<Address>(i as isize), offset, lb, ub);
                        let real_addr = get_stack_addr(slot, offset, lb, ub);
                        process_edge(tpinned_closure, real_addr);
                    } else {
                        let real_addr =
                            get_stack_addr(rts.shift::<Address>(i as isize), offset, lb, ub);

                        let slot = read_stack(rts.shift::<Address>(i as isize), offset, lb, ub);
                        use crate::julia_finalizer::gc_ptr_tag;
                        // malloced pointer tagged in jl_gc_add_quiescent
                        // skip both the next element (native function), and the object
                        if slot & 3usize == 3 {
                            i += 2;
                            continue;
                        }

                        // pointer is not malloced but function is native, so skip it
                        if gc_ptr_tag(slot, 1) {
                            process_offset_edge(tpinned_closure, real_addr, 1);
                            i += 2;
                            continue;
                        }

                        process_edge(tpinned_closure, real_addr);
                    }

                    i += 1;
                }
            }

            let s_prev_address = ::std::ptr::addr_of!((*s).prev);
            let sprev = read_stack(Address::from_ptr(s_prev_address), offset, lb, ub);
            if sprev.is_zero() {
                break;
            }

            s = sprev.to_mut_ptr::<mmtk_jl_gcframe_t>();
            let s_nroots_addr = ::std::ptr::addr_of!((*s).nroots);
            let new_nroots = read_stack(Address::from_ptr(s_nroots_addr), offset, lb, ub);
            nroots = new_nroots;
            nr = nroots >> 3;
            continue;
        }
    }
}

pub fn process_object<EV: EdgeVisitor<JuliaVMEdge>>(object: ObjectReference, closure: &mut EV) {
    let addr = object.to_raw_address();
    unsafe {
        crate::julia_scanning::scan_julia_object(addr, closure);
    }
}

// Sweep malloced arrays work
pub struct SweepVMSpecific {
    swept: bool,
}

impl SweepVMSpecific {
    pub fn new() -> Self {
        Self { swept: false }
    }
}

impl<VM: VMBinding> GCWork<VM> for SweepVMSpecific {
    fn do_work(&mut self, _worker: &mut GCWorker<VM>, _mmtk: &'static MMTK<VM>) {
        // call sweep malloced arrays and sweep stack pools from UPCALLS
        unsafe { ((*UPCALLS).mmtk_sweep_malloced_array)() }
        unsafe { ((*UPCALLS).mmtk_sweep_stack_pools)() }
        unsafe { ((*UPCALLS).mmtk_sweep_weak_refs)() }
        self.swept = true;
    }
}

pub struct ScanFinalizersSingleThreaded<C: ObjectTracerContext<JuliaVM>> {
    tracer_context: C,
}

impl<C: ObjectTracerContext<JuliaVM>> GCWork<JuliaVM> for ScanFinalizersSingleThreaded<C> {
    fn do_work(&mut self, worker: &mut GCWorker<JuliaVM>, _mmtk: &'static MMTK<JuliaVM>) {
        unsafe { ((*UPCALLS).mmtk_clear_weak_refs)() }
        self.tracer_context.with_tracer(worker, |tracer| {
            crate::julia_finalizer::scan_finalizers_in_rust(tracer);
        });
    }
}
