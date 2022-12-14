#ifndef MMTK_H
#define MMTK_H

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif


typedef struct {
    void* a;
    void* b;
} closure_pointer;
typedef void* MMTk_Mutator;
typedef void* MMTk_TraceLocal;
typedef void* (*TraceSlotFn)(void* slot, long offset);
typedef void* (*TraceObjFn)(void* obj, bool scan_obj);
typedef void* (*ScanObjFn)(void* obj);
typedef void* (*DispatchScnObjFn)(void** vec, int len, int cap, int final, closure_pointer closure);
typedef void* (*ProcessEdgeFn)(closure_pointer closure, void* slot);
typedef void* (*ProcessOffsetEdgeFn)(closure_pointer closure, void* slot, int offset);

/**
 * Allocation
 */
extern MMTk_Mutator bind_mutator(void *tls, int tid);
extern void add_mutator_ref(void* mutator_ref);
extern void destroy_mutator(MMTk_Mutator mutator);

extern void* alloc(MMTk_Mutator mutator, size_t size,
    size_t align, size_t offset, int allocator);

extern void* alloc_large(MMTk_Mutator mutator, size_t size,
    size_t align, size_t offset, int allocator);    

extern void post_alloc(MMTk_Mutator mutator, void* refer,
    int bytes, int allocator);

extern void* mmtk_counted_malloc(size_t size);
extern void* mmtk_malloc(size_t size);
extern void* mmtk_counted_calloc(size_t n, size_t size);
extern void* mmtk_calloc(size_t n, size_t size);
extern void* mmtk_realloc(void* addr, size_t size);
extern void* mmtk_realloc_with_old_size(void* addr, size_t size, size_t old_size);
extern void mmtk_free_with_size(void* addr, size_t old_size);
extern void mmtk_free(void* addr);
extern void* mmtk_malloc_aligned(size_t size, size_t alignment);
extern void mmtk_free_aligned(void* addr);

extern bool is_live_object(void* ref);
extern bool is_mapped_object(void* ref);
extern bool is_mapped_address(void* addr);
extern void modify_check(void* ref);
extern int object_is_managed_by_mmtk(void* addr);



/**
 * Tracing
 */
extern void report_delayed_root_edge(MMTk_TraceLocal trace_local,
                                     void* addr);

extern bool will_not_move_in_current_collection(MMTk_TraceLocal trace_local,
                                                void* obj);

extern void process_interior_edge(MMTk_TraceLocal trace_local, void* target,
                                  void* slot, bool root);

extern void* trace_get_forwarded_referent(MMTk_TraceLocal trace_local, void* obj);

extern void* trace_get_forwarded_reference(MMTk_TraceLocal trace_local, void* obj);

extern void* trace_retain_referent(MMTk_TraceLocal trace_local, void* obj);

/**
 * Julia-specific
 */

typedef struct {
    void (* scan_julia_obj) (void* obj, closure_pointer closure, ProcessEdgeFn process_edge, ProcessOffsetEdgeFn process_offset_edge);
    void (* scan_julia_exc_obj) (void* obj, closure_pointer closure, ProcessEdgeFn process_edge);
    void* (* get_stackbase) (signed short tid);
    void (* calculate_roots) (void* tls);
    void (* run_finalizer_function) (void* obj, void* function, bool is_ptr);
    unsigned long (* get_jl_last_err) (void);
    void (* set_jl_last_err) (unsigned long errno);
    unsigned long (* get_lo_size) (void* obj);
    unsigned long (* get_so_size) (void* obj);
    void* (* get_obj_start_ref) (void* obj);
    void (* wait_for_the_world) (void);
    signed char (* set_gc_initial_state) (void* tls);
    void (* set_gc_final_state) (signed char old_state);
    void (* set_gc_old_state) (signed char old_state);
    void (* mmtk_jl_run_finalizers) (void* tls);
    void (* jl_throw_out_of_memory_error) (void);
    void (* mark_object_as_scanned) (void* obj);
    signed char (* object_has_been_scanned) (void* obj);
    void (* sweep_malloced_array) (void);
    signed char (* wait_in_a_safepoint) (void);
    void (* exit_from_safepoint) (signed char old_state);
    void (* mmtk_sweep_stack_pools) (void);
} Julia_Upcalls;

/**
 * Misc
 */
extern void gc_init(long long heap_size, Julia_Upcalls *calls, long header_size);
extern bool will_never_move(void* object);
extern bool process(char* name, char* value);
extern void scan_region(void);
extern void handle_user_collection_request(void *tls);
extern void initialize_collection(void* tls);
extern void enable_collection(void);
extern void disable_collection(void);
extern void start_control_collector(void *tls);
extern void start_worker(void *tls, void* worker, void* mmtk);
extern void process_julia_obj(void* addr);
extern void register_finalizer(void* obj, void* function, bool is_ptr);
extern void run_finalizers_for_obj(void* obj);
extern void mmtk_run_finalizers(bool at_exit);
extern void mmtk_gc_poll(void *tls);
extern void add_object_to_mmtk_roots(void* obj);

/**
 * VM Accounting
 */
extern size_t free_bytes(void);
extern size_t total_bytes(void);
extern size_t used_bytes(void);
extern void* starting_heap_address(void);
extern void* last_heap_address(void);

/**
 * Reference Processing
 */
extern void mmtk_add_weak_candidate(void* ref);
extern void mmtk_add_soft_candidate(void* ref);
extern void mmtk_add_phantom_candidate(void* ref);

extern void harness_begin(void *tls);
extern void harness_end(void);

#ifdef __cplusplus
}
#endif

#endif // MMTK_H

