#include "mmtk.h"
#include "gc.h"

extern Julia_Upcalls mmtk_upcalls;

static void combine_thread_gc_counts(jl_gc_num_t *dest);

static int calculate_roots(jl_ptls_t ptls);

static void run_finalizer_function(jl_value_t *o, jl_value_t *ff, bool is_ptr);

int get_jl_last_err(void);

void set_jl_last_err(int e);

size_t get_lo_size(bigval_t obj);

int set_gc_initial_state(jl_ptls_t ptls);

void set_gc_final_state(int old_state);

int set_gc_running_state(jl_ptls_t ptls);

int set_gc_old_state(int old_state);

void mark_object_as_scanned(jl_value_t* obj);

int object_has_been_scanned(jl_value_t* obj);

static void sweep_malloced_arrays(void) JL_NOTSAFEPOINT;

static void jl_gc_wait_for_the_world(void);

void mmtk_jl_gc_run_all_finalizers();

void mmtk_jl_run_finalizers(jl_ptls_t tls);

JL_DLLEXPORT void scan_julia_obj(jl_value_t* obj, closure_pointer closure, ProcessEdgeFn process_edge, ProcessOffsetEdgeFn process_offset_edge);