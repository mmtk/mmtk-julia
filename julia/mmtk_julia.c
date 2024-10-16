#include "mmtk_julia.h"
#include "mmtk.h"
#include "gc-mmtk.h"
#include "mmtk_julia_types.h"
#include <stdbool.h>
#include <stddef.h>
#include "gc-common.h"
#include "threading.h"

extern int64_t perm_scanned_bytes;
extern void run_finalizer(jl_task_t *ct, void *o, void *ff);
extern int gc_n_threads;
extern jl_ptls_t* gc_all_tls_states;
extern jl_value_t *cmpswap_names JL_GLOBALLY_ROOTED;
extern jl_genericmemory_t *jl_global_roots_list JL_GLOBALLY_ROOTED;
extern jl_genericmemory_t *jl_global_roots_keyset JL_GLOBALLY_ROOTED;
extern jl_typename_t *jl_array_typename JL_GLOBALLY_ROOTED;
extern long BI_METADATA_START_ALIGNED_DOWN;
extern long BI_METADATA_END_ALIGNED_UP;
extern uint64_t finalizer_rngState[JL_RNG_SIZE];
extern const unsigned pool_sizes[];
extern size_t mmtk_get_obj_size(void* obj);
extern void jl_rng_split(uint64_t to[JL_RNG_SIZE], uint64_t from[JL_RNG_SIZE]);
extern void _jl_free_stack(jl_ptls_t ptls, void *stkbuf, size_t bufsz);
extern void free_stack(void *stkbuf, size_t bufsz);
extern jl_mutex_t finalizers_lock;
extern void jl_gc_wait_for_the_world(jl_ptls_t* gc_all_tls_states, int gc_n_threads);
extern void mmtk_block_thread_for_gc(void);
extern int64_t live_bytes;
extern void jl_throw_out_of_memory_error(void);
extern uint32_t jl_get_gc_disable_counter(void);


extern void* new_mutator_iterator(void);
extern jl_ptls_t get_next_mutator_tls(void*);
extern void* close_mutator_iterator(void*);

JL_DLLEXPORT void (jl_mmtk_harness_begin)(void)
{
    jl_ptls_t ptls = jl_current_task->ptls;
    mmtk_harness_begin(ptls);
}

JL_DLLEXPORT void (jl_mmtk_harness_end)(void)
{
    mmtk_harness_end();
}

// This is used in mmtk_sweep_malloced_memory and it is slightly different 
// from jl_gc_free_memory from gc-stock.c as the stock GC updates the 
// information in the global variable gc_heap_stats (which is specific to the stock GC)
static void jl_gc_free_memory(jl_value_t *v, int isaligned) JL_NOTSAFEPOINT
{
    assert(jl_is_genericmemory(v));
    jl_genericmemory_t *m = (jl_genericmemory_t*)v;
    assert(jl_genericmemory_how(m) == 1 || jl_genericmemory_how(m) == 2);
    char *d = (char*)m->ptr;
    if (isaligned)
        jl_free_aligned(d);
    else
        free(d);
    gc_num.freed += jl_genericmemory_nbytes(m);
    gc_num.freecall++;
}

static void mmtk_sweep_malloced_memory(void) JL_NOTSAFEPOINT
{
    void* iter = new_mutator_iterator();
    jl_ptls_t ptls2 = get_next_mutator_tls(iter);
    while(ptls2 != NULL) {
        mallocmemory_t *ma = ptls2->gc_tls_common.heap.mallocarrays;
        mallocmemory_t **pma = &ptls2->gc_tls_common.heap.mallocarrays;
        while (ma != NULL) {
            mallocmemory_t *nxt = ma->next;
            jl_value_t *a = (jl_value_t*)((uintptr_t)ma->a & ~1);
            if (!mmtk_object_is_managed_by_mmtk(a)) {
                pma = &ma->next;
                ma = nxt;
                continue;
            }
            if (mmtk_is_live_object(a)) {
                // if the array has been forwarded, the reference needs to be updated
                jl_genericmemory_t *maybe_forwarded = (jl_genericmemory_t*)mmtk_get_possibly_forwared(ma->a);
                ma->a = maybe_forwarded;
                pma = &ma->next;
            }
            else {
                *pma = nxt;
                int isaligned = (uintptr_t)ma->a & 1;
                jl_gc_free_memory(a, isaligned);
                ma->next = ptls2->gc_tls_common.heap.mafreelist;
                ptls2->gc_tls_common.heap.mafreelist = ma;
            }
            ma = nxt;
        }
        ptls2 = get_next_mutator_tls(iter);
    }
    gc_sweep_sysimg();
    close_mutator_iterator(iter);
}

void mmtk_wait_in_a_safepoint(void) {
    jl_ptls_t ptls = jl_current_task->ptls;
    jl_gc_safepoint_(ptls);
}

void mmtk_exit_from_safepoint(int8_t old_state) {
    jl_ptls_t ptls = jl_current_task->ptls;
    jl_gc_state_set(ptls, old_state, JL_GC_STATE_WAITING);
}

// based on jl_gc_collect from gc.c
JL_DLLEXPORT void jl_gc_prepare_to_collect(void)
{
    // FIXME: set to JL_GC_AUTO since we're calling it from mmtk
    // maybe just remove this?
    JL_PROBE_GC_BEGIN(JL_GC_AUTO);

    jl_task_t *ct = jl_current_task;
    jl_ptls_t ptls = ct->ptls;
    if (jl_atomic_load_acquire(&jl_gc_disable_counter)) {
        size_t localbytes = jl_atomic_load_relaxed(&ptls->gc_tls_common.gc_num.allocd) + gc_num.interval;
        jl_atomic_store_relaxed(&ptls->gc_tls_common.gc_num.allocd, -(int64_t)gc_num.interval);
        static_assert(sizeof(_Atomic(uint64_t)) == sizeof(gc_num.deferred_alloc), "");
        jl_atomic_fetch_add_relaxed((_Atomic(uint64_t)*)&gc_num.deferred_alloc, localbytes);
        return;
    }

    int8_t old_state = jl_atomic_load_relaxed(&ptls->gc_state);
    jl_atomic_store_release(&ptls->gc_state, JL_GC_STATE_WAITING);
    // `jl_safepoint_start_gc()` makes sure only one thread can run the GC.
    uint64_t t0 = jl_hrtime();
    if (!jl_safepoint_start_gc()) {
        jl_gc_state_set(ptls, old_state, JL_GC_STATE_WAITING);
        jl_safepoint_wait_thread_resume(); // block in thread-suspend now if requested, after clearing the gc_state
        return;
    }

    JL_TIMING_SUSPEND_TASK(GC, ct);
    JL_TIMING(GC, GC);

    int last_errno = errno;
#ifdef _OS_WINDOWS_
    DWORD last_error = GetLastError();
#endif
    // Now we are ready to wait for other threads to hit the safepoint,
    // we can do a few things that doesn't require synchronization.
    //
    // We must sync here with the tls_lock operations, so that we have a
    // seq-cst order between these events now we know that either the new
    // thread must run into our safepoint flag or we must observe the
    // existence of the thread in the jl_n_threads count.
    //
    // TODO: concurrently queue objects
    jl_fence();
    gc_n_threads = jl_atomic_load_acquire(&jl_n_threads);
    gc_all_tls_states = jl_atomic_load_relaxed(&jl_all_tls_states);
    jl_gc_wait_for_the_world(gc_all_tls_states, gc_n_threads);
    JL_PROBE_GC_STOP_THE_WORLD();

    uint64_t t1 = jl_hrtime();
    uint64_t duration = t1 - t0;
    if (duration > gc_num.max_time_to_safepoint)
        gc_num.max_time_to_safepoint = duration;
    gc_num.time_to_safepoint = duration;
    gc_num.total_time_to_safepoint += duration;

    if (!jl_atomic_load_acquire(&jl_gc_disable_counter)) {
        JL_LOCK_NOGC(&finalizers_lock); // all the other threads are stopped, so this does not make sense, right? otherwise, failing that, this seems like plausibly a deadlock
#ifndef __clang_gcanalyzer__
        mmtk_block_thread_for_gc();
#endif
        JL_UNLOCK_NOGC(&finalizers_lock);
    }

    gc_n_threads = 0;
    gc_all_tls_states = NULL;
    jl_safepoint_end_gc();
    jl_gc_state_set(ptls, old_state, JL_GC_STATE_WAITING);
    JL_PROBE_GC_END();
    jl_safepoint_wait_thread_resume(); // block in thread-suspend now if requested, after clearing the gc_state

    // Only disable finalizers on current thread
    // Doing this on all threads is racy (it's impossible to check
    // or wait for finalizers on other threads without dead lock).
    if (!ptls->finalizers_inhibited && ptls->locks.len == 0) {
        JL_TIMING(GC, GC_Finalizers);
        run_finalizers(ct, 0);
    }
    JL_PROBE_GC_FINALIZER();

#ifdef _OS_WINDOWS_
    SetLastError(last_error);
#endif
    errno = last_errno;
}

extern void run_finalizers(jl_task_t *ct, int finalizers_thread);

// We implement finalization in the binding side. These functions
// returns some pointers so MMTk can manipulate finalizer lists.

extern jl_mutex_t finalizers_lock;
extern arraylist_t to_finalize;
extern arraylist_t finalizer_list_marked;

void* get_thread_finalizer_list(void* ptls_raw) {
    jl_ptls_t ptls = (jl_ptls_t) ptls_raw;
    return (void*)&ptls->finalizers;
}

void* get_to_finalize_list(void) {
    return (void*)&to_finalize;
}

void* get_marked_finalizers_list(void) {
    return (void*)&finalizer_list_marked;
}

int* get_jl_gc_have_pending_finalizers(void) {
    return (int*)&jl_gc_have_pending_finalizers;
}

static void add_node_to_roots_buffer(RootsWorkClosure* closure, RootsWorkBuffer* buf, size_t* buf_len, void* root) {
    if (root == NULL)
        return;

    buf->ptr[*buf_len] = root;
    *buf_len += 1;
    if (*buf_len >= buf->cap) {
        RootsWorkBuffer new_buf = (closure->report_nodes_func)(buf->ptr, *buf_len, buf->cap, closure->data, true);
        *buf = new_buf;
        *buf_len = 0;
    }
}

static void add_node_to_tpinned_roots_buffer(RootsWorkClosure* closure, RootsWorkBuffer* buf, size_t* buf_len, void* root) {
    if (root == NULL)
        return;

    buf->ptr[*buf_len] = root;
    *buf_len += 1;
    if (*buf_len >= buf->cap) {
        RootsWorkBuffer new_buf = (closure->report_tpinned_nodes_func)(buf->ptr, *buf_len, buf->cap, closure->data, true);
        *buf = new_buf;
        *buf_len = 0;
    }
}

void scan_vm_specific_roots(RootsWorkClosure* closure)
{
    // Create a new buf
    RootsWorkBuffer buf = (closure->report_nodes_func)((void**)0, 0, 0, closure->data, true);
    size_t len = 0;

    // add module
    add_node_to_roots_buffer(closure, &buf, &len, jl_main_module);

    // buildin values
    add_node_to_roots_buffer(closure, &buf, &len, jl_an_empty_vec_any);
    add_node_to_roots_buffer(closure, &buf, &len, jl_module_init_order);
    for (size_t i = 0; i < jl_current_modules.size; i += 2) {
        if (jl_current_modules.table[i + 1] != HT_NOTFOUND) {
            add_node_to_roots_buffer(closure, &buf, &len, jl_current_modules.table[i]);
        }
    }
    add_node_to_roots_buffer(closure, &buf, &len, jl_anytuple_type_type);
    for (size_t i = 0; i < N_CALL_CACHE; i++) {
         jl_typemap_entry_t *v = jl_atomic_load_relaxed(&call_cache[i]);
        add_node_to_roots_buffer(closure, &buf, &len, v);
    }
    add_node_to_roots_buffer(closure, &buf, &len, _jl_debug_method_invalidation);

    // constants
    add_node_to_roots_buffer(closure, &buf, &len, jl_emptytuple_type);
    add_node_to_roots_buffer(closure, &buf, &len, cmpswap_names);

    // jl_global_roots_table must be transitively pinned 
    RootsWorkBuffer tpinned_buf = (closure->report_tpinned_nodes_func)((void**)0, 0, 0, closure->data, true);
    size_t tpinned_len = 0;
    add_node_to_tpinned_roots_buffer(closure, &tpinned_buf, &tpinned_len, jl_global_roots_list);
    add_node_to_tpinned_roots_buffer(closure, &tpinned_buf, &tpinned_len, jl_global_roots_keyset);

    // Push the result of the work.
    (closure->report_nodes_func)(buf.ptr, len, buf.cap, closure->data, false);
    (closure->report_tpinned_nodes_func)(tpinned_buf.ptr, tpinned_len, tpinned_buf.cap, closure->data, false);
}

JL_DLLEXPORT void scan_julia_exc_obj(void* obj_raw, void* closure, ProcessSlotFn process_slot) {
    jl_task_t *ta = (jl_task_t*)obj_raw;

    if (ta->excstack) { // inlining label `excstack` from mark_loop
        // if it is not managed by MMTk, nothing needs to be done because the object does not need to be scanned
        if (mmtk_object_is_managed_by_mmtk(ta->excstack)) {
            process_slot(closure, &ta->excstack);
        }
        jl_excstack_t *excstack = ta->excstack;
        size_t itr = ta->excstack->top;
        size_t bt_index = 0;
        size_t jlval_index = 0;
        while (itr > 0) {
            size_t bt_size = jl_excstack_bt_size(excstack, itr);
            jl_bt_element_t *bt_data = jl_excstack_bt_data(excstack, itr);
            for (; bt_index < bt_size; bt_index += jl_bt_entry_size(bt_data + bt_index)) {
                jl_bt_element_t *bt_entry = bt_data + bt_index;
                if (jl_bt_is_native(bt_entry))
                    continue;
                // Found an extended backtrace entry: iterate over any
                // GC-managed values inside.
                size_t njlvals = jl_bt_num_jlvals(bt_entry);
                while (jlval_index < njlvals) {
                    jl_value_t** new_obj_slot = &bt_entry[2 + jlval_index].jlvalue;
                    jlval_index += 1;
                    process_slot(closure, new_obj_slot);
                }
                jlval_index = 0;
            }

            jl_bt_element_t *stack_raw = (jl_bt_element_t *)(excstack+1);
            jl_value_t** stack_obj_slot = &stack_raw[itr-1].jlvalue;

            itr = jl_excstack_next(excstack, itr);
            bt_index = 0;
            jlval_index = 0;
            process_slot(closure, stack_obj_slot);
        }
    }
}

// number of stacks to always keep available per pool - from gc-stacks.c
#define MIN_STACK_MAPPINGS_PER_POOL 5

#define jl_genericmemory_elsize(a) (((jl_datatype_t*)jl_typetagof(a))->layout->size)

// if data is inlined inside the genericmemory object --- to->ptr needs to be updated when copying the array
void update_inlined_array(void* from, void* to) {
    jl_value_t* jl_from = (jl_value_t*) from;
    jl_value_t* jl_to = (jl_value_t*) to;

    uintptr_t tag_to = (uintptr_t)jl_typeof(jl_to);
    jl_datatype_t *vt = (jl_datatype_t*)tag_to;

    if(vt->name == jl_genericmemory_typename) {
        jl_genericmemory_t *a = (jl_genericmemory_t*)jl_from;
        jl_genericmemory_t *b = (jl_genericmemory_t*)jl_to;
        int how = jl_genericmemory_how(a);

        if (how == 0 && mmtk_object_is_managed_by_mmtk(a->ptr)) { // a is inlined (a->ptr points into the mmtk object)
            size_t offset_of_data = ((size_t)a->ptr - (size_t)a);
            if (offset_of_data > 0) {
                b->ptr = (void*)((size_t) b + offset_of_data);
            }
        }
    }
}

// modified sweep_stack_pools from gc-stacks.c
void mmtk_sweep_stack_pools(void)
{
    // Stack sweeping algorithm:
    //    // deallocate stacks if we have too many sitting around unused
    //    for (stk in halfof(free_stacks))
    //        free_stack(stk, pool_sz);
    //    // then sweep the task stacks
    //    for (t in live_tasks)
    //        if (!gc-marked(t))
    //            stkbuf = t->stkbuf
    //            bufsz = t->bufsz
    //            if (stkbuf)
    //                push(free_stacks[sz], stkbuf)
    assert(gc_n_threads);
    for (int i = 0; i < jl_n_threads; i++) {
        jl_ptls_t ptls2 = gc_all_tls_states[i];
        if (ptls2 == NULL)
            continue;

        // free half of stacks that remain unused since last sweep
        for (int p = 0; p < JL_N_STACK_POOLS; p++) {
            small_arraylist_t *al = &ptls2->gc_tls_common.heap.free_stacks[p];
            size_t n_to_free;
            if (jl_atomic_load_relaxed(&ptls2->current_task) == NULL) {
                n_to_free = al->len; // not alive yet or dead, so it does not need these anymore
            }
            else if (al->len > MIN_STACK_MAPPINGS_PER_POOL) {
                n_to_free = al->len / 2;
                if (n_to_free > (al->len - MIN_STACK_MAPPINGS_PER_POOL))
                    n_to_free = al->len - MIN_STACK_MAPPINGS_PER_POOL;
            }
            else {
                n_to_free = 0;
            }
            for (int n = 0; n < n_to_free; n++) {
                void *stk = small_arraylist_pop(al);
                free_stack(stk, pool_sizes[p]);
            }
            if (jl_atomic_load_relaxed(&ptls2->current_task) == NULL) {
                small_arraylist_free(al);
            }
        }
        if (jl_atomic_load_relaxed(&ptls2->current_task) == NULL) {
            small_arraylist_free(ptls2->gc_tls_common.heap.free_stacks);
        }

        small_arraylist_t *live_tasks = &ptls2->gc_tls_common.heap.live_tasks;
        size_t n = 0;
        size_t ndel = 0;
        size_t l = live_tasks->len;
        void **lst = live_tasks->items;
        if (l == 0)
            continue;
        while (1) {
            jl_task_t *t = (jl_task_t*)lst[n];
            if (mmtk_is_live_object(t)) {
                jl_task_t *maybe_forwarded = (jl_task_t*)mmtk_get_possibly_forwared(t);
                live_tasks->items[n] = maybe_forwarded;
                t = maybe_forwarded;
                assert(jl_is_task(t));
                if (t->ctx.stkbuf == NULL)
                    ndel++; // jl_release_task_stack called
                else
                    n++;
            } else {
                ndel++;
                void *stkbuf = t->ctx.stkbuf;
                size_t bufsz = t->ctx.bufsz;
                if (stkbuf) {
                    t->ctx.stkbuf = NULL;
                    _jl_free_stack(ptls2, stkbuf, bufsz);
                }
#ifdef _COMPILER_TSAN_ENABLED_
                if (t->ctx.tsan_state) {
                    __tsan_destroy_fiber(t->ctx.tsan_state);
                    t->ctx.tsan_state = NULL;
                }
#endif
            }
            if (n >= l - ndel)
                break;
            void *tmp = lst[n];
            lst[n] = lst[n + ndel];
            lst[n + ndel] = tmp;
        }
        live_tasks->len -= ndel;
    }
}

JL_DLLEXPORT void* get_stackbase(int16_t tid) {
    assert(tid >= 0);
    jl_ptls_t ptls2 = jl_all_tls_states[tid];
    return ptls2->stackbase;
}

const bool PRINT_OBJ_TYPE = false;

void update_gc_stats(uint64_t inc, size_t mmtk_live_bytes, bool is_nursery_gc) {
    gc_num.total_time += inc;
    gc_num.pause += 1;
    gc_num.full_sweep += !(is_nursery_gc);
    gc_num.total_allocd += gc_num.allocd;
    gc_num.allocd = 0;
    live_bytes = mmtk_live_bytes;
}

#define assert_size(ty_a, ty_b) \
    if(sizeof(ty_a) != sizeof(ty_b)) {\
        printf("%s size = %ld, %s size = %ld. Need to update our type definition.\n", #ty_a, sizeof(ty_a), #ty_b, sizeof(ty_b));\
        exit(1); \
    }

#define PRINT_STRUCT_SIZE false
#define print_sizeof(type) (PRINT_STRUCT_SIZE ? (printf("C " #type " = %zu bytes\n", sizeof(type)), sizeof(type)) : sizeof(type))

#define jl_genericmemory_data_owner_field_addr(a) ((jl_value_t**)((jl_genericmemory_t*)(a) + 1))

void* jl_get_owner_address_to_mmtk(void* m) {
    return (void*)jl_genericmemory_data_owner_field_addr(m);
}

size_t mmtk_jl_genericmemory_how(void *arg) JL_NOTSAFEPOINT
{
    jl_genericmemory_t* m = (jl_genericmemory_t*)arg;
    if (m->ptr == (void*)((char*)m + 16)) // JL_SMALL_BYTE_ALIGNMENT (from julia_internal.h)
        return 0;
    jl_value_t *owner = jl_genericmemory_data_owner_field(m);
    if (owner == (jl_value_t*)m)
        return 1;
    if (owner == NULL)
        return 2;
    return 3;
}


uintptr_t get_abi_structs_checksum_c(void) {
    assert_size(struct mmtk__jl_taggedvalue_bits, struct _jl_taggedvalue_bits);
    assert_size(mmtk_jl_taggedvalue_t, jl_taggedvalue_t);
    assert_size(mmtk_jl_datatype_layout_t, jl_datatype_layout_t);
    assert_size(mmtk_jl_typename_t, jl_typename_t);
    assert_size(mmtk_jl_svec_t, jl_svec_t);
    assert_size(mmtk_jl_datatype_t, jl_datatype_t);
    assert_size(mmtk_jl_array_t, jl_array_t);
    assert_size(mmtk_jl_sym_t, jl_sym_t);
    assert_size(mmtk_jl_binding_t, jl_binding_t);
    assert_size(mmtk_htable_t, htable_t);
    assert_size(mmtk_arraylist_t, arraylist_t);
    assert_size(mmtk_jl_uuid_t, jl_uuid_t);
    assert_size(mmtk_jl_mutex_t, jl_mutex_t);
    assert_size(mmtk_jl_module_t, jl_module_t);
    assert_size(mmtk_jl_excstack_t, jl_excstack_t);
    assert_size(mmtk_jl_bt_element_t, jl_bt_element_t);
    assert_size(mmtk_jl_stack_context_t, jl_stack_context_t);
    assert_size(mmtk_jl_ucontext_t, jl_ucontext_t);
    assert_size(struct mmtk__jl_gcframe_t, struct _jl_gcframe_t);
    assert_size(mmtk_jl_task_t, jl_task_t);
    assert_size(mmtk_jl_weakref_t, jl_weakref_t);

    return print_sizeof(MMTkMutatorContext)
        ^ print_sizeof(struct mmtk__jl_taggedvalue_bits)
        ^ print_sizeof(mmtk_jl_taggedvalue_t)
        ^ print_sizeof(mmtk_jl_datatype_layout_t)
        ^ print_sizeof(mmtk_jl_typename_t)
        ^ print_sizeof(mmtk_jl_svec_t)
        ^ print_sizeof(mmtk_jl_datatype_t)
        ^ print_sizeof(mmtk_jl_array_t)
        ^ print_sizeof(mmtk_jl_sym_t)
        ^ print_sizeof(mmtk_jl_binding_t)
        ^ print_sizeof(mmtk_htable_t)
        ^ print_sizeof(mmtk_arraylist_t)
        ^ print_sizeof(mmtk_jl_uuid_t)
        ^ print_sizeof(mmtk_jl_mutex_t)
        ^ print_sizeof(mmtk_jl_module_t)
        ^ print_sizeof(mmtk_jl_excstack_t)
        ^ print_sizeof(mmtk_jl_bt_element_t)
        ^ print_sizeof(mmtk_jl_stack_context_t)
        ^ print_sizeof(mmtk_jl_ucontext_t)
        ^ print_sizeof(struct mmtk__jl_gcframe_t)
        ^ print_sizeof(mmtk_jl_task_t)
        ^ print_sizeof(mmtk_jl_weakref_t)
        ^ print_sizeof(mmtk_jl_tls_states_t)
        ^ print_sizeof(mmtk_jl_thread_heap_common_t)
        ^ print_sizeof(mmtk_jl_thread_gc_num_common_t);
}

Julia_Upcalls mmtk_upcalls = (Julia_Upcalls) {
    .scan_julia_exc_obj = scan_julia_exc_obj,
    .get_stackbase = get_stackbase,
    .jl_throw_out_of_memory_error = jl_throw_out_of_memory_error,
    .jl_get_gc_disable_counter = jl_get_gc_disable_counter,
    .sweep_malloced_memory = mmtk_sweep_malloced_memory,
    .sweep_stack_pools = mmtk_sweep_stack_pools,
    .wait_in_a_safepoint = mmtk_wait_in_a_safepoint,
    .exit_from_safepoint = mmtk_exit_from_safepoint,
    .jl_hrtime = jl_hrtime,
    .update_gc_stats = update_gc_stats,
    .get_abi_structs_checksum_c = get_abi_structs_checksum_c,
    .get_thread_finalizer_list = get_thread_finalizer_list,
    .get_to_finalize_list = get_to_finalize_list,
    .get_marked_finalizers_list = get_marked_finalizers_list,
    .arraylist_grow = (void (*)(void*, long unsigned int))arraylist_grow,
    .get_jl_gc_have_pending_finalizers = get_jl_gc_have_pending_finalizers,
    .scan_vm_specific_roots = scan_vm_specific_roots,
    .update_inlined_array = update_inlined_array,
    .prepare_to_collect = jl_gc_prepare_to_collect,
    .get_owner_address = jl_get_owner_address_to_mmtk,
    .mmtk_genericmemory_how = mmtk_jl_genericmemory_how,
};
